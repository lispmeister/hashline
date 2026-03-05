//! Claude Code hook handlers for read-before-apply enforcement.
//!
//! `hashline hook pre`  - PreToolUse: blocks Edit/NotebookEdit, enforces read-before-apply for Bash
//! `hashline hook post` - PostToolUse: tracks hashline read/apply session state
use std::io::Read;
use std::path::{Path, PathBuf};
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum HashlineCmdKind {
    Read,
    JsonRead,
    Apply,
    JsonApply,
}

/// Session file path: `<tmp>/hashline_session_<ppid>` unless overridden.
fn session_path() -> PathBuf {
    if let Some(custom) = std::env::var_os("HASHLINE_SESSION_FILE") {
        return PathBuf::from(custom);
    }
    let ppid = get_parent_pid();
    let tmp = std::env::temp_dir();
    tmp.join(format!("hashline_session_{}", ppid))
}
#[cfg(unix)]
fn get_parent_pid() -> u32 {
    std::os::unix::process::parent_id()
}
#[cfg(windows)]
fn get_parent_pid() -> u32 {
    std::env::var("HASHLINE_SESSION_PID")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(std::process::id)
}
fn resolve_path(p: &str) -> String {
    let path = Path::new(p);
    if path.is_absolute() {
        p.to_string()
    } else {
        match std::env::current_dir() {
            Ok(cwd) => cwd.join(p).display().to_string(),
            Err(_) => p.to_string(),
        }
    }
}
fn read_stdin() -> String {
    let mut buf = String::new();
    let _ = std::io::stdin().read_to_string(&mut buf);
    buf
}
/// Extract a string field from a JSON value at a dotted path like "tool_input.command"
fn json_str<'a>(v: &'a serde_json::Value, path: &str) -> Option<&'a str> {
    let mut current = v;
    for key in path.split('.') {
        current = current.get(key)?;
    }
    current.as_str()
}
fn json_bool(v: &serde_json::Value, path: &str) -> Option<bool> {
    let mut current = v;
    for key in path.split('.') {
        current = current.get(key)?;
    }
    current.as_bool()
}

fn strict_mode() -> bool {
    match std::env::var("HASHLINE_HOOK_STRICT") {
        Ok(v) => {
            let s = v.trim().to_ascii_lowercase();
            !s.is_empty() && s != "0" && s != "false" && s != "no" && s != "off"
        }
        Err(_) => false,
    }
}

fn first_shell_line(cmd: &str) -> &str {
    cmd.lines().next().unwrap_or(cmd)
}

fn tokenize_shell_line(line: &str) -> Vec<String> {
    match shell_words::split(line) {
        Ok(tokens) => tokens,
        Err(_) => line.split_whitespace().map(|s| s.to_string()).collect(),
    }
}

fn parse_hashline_cmd(cmd: &str) -> Option<(HashlineCmdKind, Vec<String>, usize)> {
    let tokens = tokenize_shell_line(first_shell_line(cmd));
    let idx = tokens
        .iter()
        .position(|t| t == "hashline" || t.ends_with("/hashline"))?;
    let sub = tokens.get(idx + 1)?.as_str();
    let kind = match sub {
        "read" => HashlineCmdKind::Read,
        "json-read" => HashlineCmdKind::JsonRead,
        "apply" => HashlineCmdKind::Apply,
        "json-apply" => HashlineCmdKind::JsonApply,
        _ => return None,
    };
    Some((kind, tokens, idx + 2))
}

fn has_emit_updated(tokens: &[String], args_start: usize) -> bool {
    tokens[args_start..].iter().any(|t| t == "--emit-updated")
}

fn extract_input_flag(tokens: &[String], args_start: usize) -> Option<String> {
    let mut i = args_start;
    while i < tokens.len() {
        let t = &tokens[i];
        if t == "--input" || t == "-i" {
            if i + 1 < tokens.len() {
                return Some(tokens[i + 1].clone());
            }
            return None;
        }
        if let Some(rest) = t.strip_prefix("--input=") {
            if !rest.is_empty() {
                return Some(rest.to_string());
            }
        }
        i += 1;
    }
    None
}
fn extract_path_from_json_text(text: &str) -> Option<String> {
    let re = regex::Regex::new(r#""path"\s*:\s*"([^"]*)""#).ok()?;
    re.captures(text)
        .map(|c| c[1].to_string())
        .filter(|s| !s.is_empty())
}
/// Extract the target file path from a hashline apply/json-apply command string.
fn extract_apply_file(cmd: &str) -> Option<String> {
    let (kind, tokens, args_start) = parse_hashline_cmd(cmd)?;
    if kind != HashlineCmdKind::Apply && kind != HashlineCmdKind::JsonApply {
        return None;
    }

    if let Some(ifile) = extract_input_flag(&tokens, args_start) {
        if Path::new(&ifile).is_file() {
            if let Ok(contents) = std::fs::read_to_string(&ifile) {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&contents) {
                    if let Some(p) = v.get("path").and_then(|v| v.as_str()) {
                        if !p.is_empty() {
                            return Some(p.to_string());
                        }
                    }
                }
            }
        }
    }
    extract_path_from_json_text(cmd)
}
/// Extract the file argument from a hashline read/json-read command.
fn extract_read_file(cmd: &str) -> Option<String> {
    let (kind, tokens, args_start) = parse_hashline_cmd(cmd)?;
    if kind != HashlineCmdKind::Read && kind != HashlineCmdKind::JsonRead {
        return None;
    }

    let mut positional = Vec::new();
    let mut i = args_start;
    while i < tokens.len() {
        let t = &tokens[i];
        if t == "--start-line" || t == "--lines" {
            i += 2;
            continue;
        }
        if t.starts_with('-') {
            i += 1;
            continue;
        }
        positional.push(t.clone());
        i += 1;
    }

    positional.last().cloned()
}

fn apply_kind(cmd: &str) -> Option<HashlineCmdKind> {
    let (kind, _, _) = parse_hashline_cmd(cmd)?;
    match kind {
        HashlineCmdKind::Apply | HashlineCmdKind::JsonApply => Some(kind),
        _ => None,
    }
}
fn is_read_cmd(cmd: &str) -> bool {
    matches!(
        parse_hashline_cmd(cmd).map(|(k, _, _)| k),
        Some(HashlineCmdKind::Read | HashlineCmdKind::JsonRead)
    )
}

fn expected_read_command(kind: HashlineCmdKind, file: &str) -> String {
    match kind {
        HashlineCmdKind::JsonApply => format!("hashline json-read {}", file),
        _ => format!("hashline read {}", file),
    }
}

// -- Session file operations --------------------------------------------------
fn session_has(session: &Path, entry: &str) -> bool {
    std::fs::read_to_string(session)
        .map(|c| c.lines().any(|l| l == entry))
        .unwrap_or(false)
}
fn mark_session(session: &Path, file: &str, state: &str) {
    let read_entry = format!("read:{}", file);
    let stale_entry = format!("stale:{}", file);
    let new_entry = format!("{}:{}", state, file);
    let existing = std::fs::read_to_string(session).unwrap_or_default();
    let lines: Vec<&str> = existing
        .lines()
        .filter(|l| *l != read_entry && *l != stale_entry)
        .collect();
    let mut result = lines.join("\n");
    if !result.is_empty() {
        result.push('\n');
    }
    result.push_str(&new_entry);
    result.push('\n');
    let tmp = session.with_extension("tmp");
    let _ = std::fs::write(&tmp, &result);
    let _ = std::fs::rename(&tmp, session);
}

fn pre_from_value(v: &serde_json::Value) -> i32 {
    if let Some(file_path) = json_str(v, "tool_input.file_path") {
        eprintln!(
            "BLOCKED: Do not use the Edit tool in this project.\nFile: {}\nUse: hashline apply\nSee CLAUDE.md.",
            file_path
        );
        return 2;
    }

    let cmd = match json_str(v, "tool_input.command") {
        Some(c) => c,
        None => {
            eprintln!(
                "BLOCKED: Do not use NotebookEdit in this project. Use hashline apply via Bash. See CLAUDE.md."
            );
            return 2;
        }
    };

    let kind = match apply_kind(cmd) {
        Some(k) => k,
        None => return 0,
    };
    let file = match extract_apply_file(cmd) {
        Some(f) => resolve_path(&f),
        None => {
            if strict_mode() {
                eprintln!(
                    "BLOCKED: Could not determine apply target path in strict mode.\nUse --input with JSON containing \"path\", or an inline payload with \"path\"."
                );
                return 2;
            }
            return 0;
        }
    };

    let session = session_path();
    let read_entry = format!("read:{}", file);
    let stale_entry = format!("stale:{}", file);
    let read_cmd = expected_read_command(kind, &file);
    if session_has(&session, &read_entry) {
        return 0;
    }
    if session_has(&session, &stale_entry) {
        eprintln!(
            "BLOCKED: \"{}\" was modified by hashline apply but not re-read.\nAnchors are stale. Run:\n  {}\nbefore applying edits.",
            file, read_cmd
        );
        return 2;
    }

    eprintln!(
        "BLOCKED: \"{}\" has not been read in this session.\nRun:\n  {}\nbefore applying edits.",
        file, read_cmd
    );
    2
}
/// PreToolUse hook handler. Exit 0 = allow, exit 2 = block.
pub fn pre() -> i32 {
    let input = read_stdin();
    let v: serde_json::Value = match serde_json::from_str(&input) {
        Ok(v) => v,
        Err(_) => return 0,
    };
    pre_from_value(&v)
}
/// PostToolUse hook handler. Always exits 0.
pub fn post() -> i32 {
    let input = read_stdin();
    let v: serde_json::Value = match serde_json::from_str(&input) {
        Ok(v) => v,
        Err(_) => return 0,
    };
    let cmd = match json_str(&v, "tool_input.command") {
        Some(c) => c.to_string(),
        None => return 0,
    };
    let is_error = json_bool(&v, "tool_response.isError").unwrap_or(false);
    if is_error {
        return 0;
    }

    let session = session_path();
    if is_read_cmd(&cmd) {
        if let Some(file) = extract_read_file(&cmd) {
            let file = resolve_path(&file);
            mark_session(&session, &file, "read");
        }
    } else if let Some((kind, tokens, args_start)) = parse_hashline_cmd(&cmd) {
        if matches!(kind, HashlineCmdKind::Apply | HashlineCmdKind::JsonApply) {
            if let Some(file) = extract_apply_file(&cmd) {
                let file = resolve_path(&file);
                if has_emit_updated(&tokens, args_start) {
                    mark_session(&session, &file, "read");
                } else {
                    mark_session(&session, &file, "stale");
                }
            }
        }
    }

    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_apply_with_env_prefix() {
        let kind = apply_kind("FOO=1 BAR=2 hashline apply --input edits.json");
        assert_eq!(kind, Some(HashlineCmdKind::Apply));
    }

    #[test]
    fn extract_input_supports_short_flag() {
        let missing = std::env::temp_dir().join(format!("hashline-missing-input-{}-{}.json", std::process::id(), std::thread::current().name().unwrap_or("t")));
        let _ = std::fs::remove_file(&missing);
        let cmd = format!("hashline apply -i {}", missing.display());

        let f = extract_apply_file(&cmd);
        assert!(f.is_none());
        let tokens = tokenize_shell_line(&cmd);
        let got = extract_input_flag(&tokens, 2);
        assert_eq!(got.as_deref(), Some(missing.to_string_lossy().as_ref()));
    }

    #[test]
    fn extract_read_file_handles_quotes() {
        let got = extract_read_file("hashline read --start-line 2 --lines 5 \"dir/a b.rs\"");
        assert_eq!(got.as_deref(), Some("dir/a b.rs"));
    }

    #[test]
    fn strict_mode_blocks_unresolvable_apply() {
        let v: serde_json::Value = serde_json::json!({
            "tool_input": {"command": "hashline apply --input /tmp/missing.json"}
        });
        std::env::set_var("HASHLINE_HOOK_STRICT", "1");
        let code = pre_from_value(&v);
        std::env::remove_var("HASHLINE_HOOK_STRICT");
        assert_eq!(code, 2);
    }
}
