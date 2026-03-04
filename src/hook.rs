//! Claude Code hook handlers for read-before-apply enforcement.
//!
//! `hashline hook pre`  — PreToolUse: blocks Edit/NotebookEdit, enforces read-before-apply for Bash
//! `hashline hook post` — PostToolUse: tracks hashline read/apply session state

use std::io::Read;
use std::path::{Path, PathBuf};

/// Session file path: `<tmp>/hashline_session_<ppid>`
fn session_path() -> PathBuf {
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
    // On Windows, use HASHLINE_SESSION_PID env var if set,
    // otherwise fall back to current process ID
    std::env::var("HASHLINE_SESSION_PID")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| std::process::id())
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

// ── Path extraction from hashline commands ──────────────────────────────────

/// Extract the target file path from a hashline apply/json-apply command string.
fn extract_apply_file(cmd: &str) -> Option<String> {
    // --input variant: read the JSON file and get .path
    if cmd.contains("--input") {
        if let Some(ifile) = extract_input_flag(cmd) {
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
    }

    // Heredoc/inline variant: regex for "path": "<value>"
    extract_path_from_json_text(cmd)
}

fn extract_input_flag(cmd: &str) -> Option<String> {
    // Match --input <file> (space-separated)
    let re = regex::Regex::new(r"--input\s+(\S+)").ok()?;
    re.captures(cmd).map(|c| c[1].to_string())
}

fn extract_path_from_json_text(text: &str) -> Option<String> {
    let re = regex::Regex::new(r#""path"\s*:\s*"([^"]*)""#).ok()?;
    re.captures(text).map(|c| c[1].to_string()).filter(|s| !s.is_empty())
}

/// Extract the file argument from a hashline read/json-read command.
/// The file is the last non-flag, non-numeric token after `hashline (json-)?read`.
fn extract_read_file(cmd: &str) -> Option<String> {
    let first_line = cmd.lines().next().unwrap_or(cmd);
    // Find "hashline read" or "hashline json-read" and take everything after
    let re = regex::Regex::new(r"hashline\s+(json-)?read\s+(.*)").ok()?;
    let caps = re.captures(first_line)?;
    let rest = caps.get(2)?.as_str();
    let tokens: Vec<&str> = rest.split_whitespace()
        .filter(|t| !t.starts_with('-') && t.parse::<u64>().is_err())
        .collect();
    tokens.last().map(|s| s.to_string())
}

/// Detect if the first line of cmd is `hashline (apply|json-apply)`
fn is_apply_cmd(cmd: &str) -> bool {
    let first = cmd.lines().next().unwrap_or(cmd);
    regex::Regex::new(r"^\s*hashline\s+(apply|json-apply)\b")
        .map(|r| r.is_match(first))
        .unwrap_or(false)
}

fn is_read_cmd(cmd: &str) -> bool {
    let first = cmd.lines().next().unwrap_or(cmd);
    regex::Regex::new(r"^\s*hashline\s+(read|json-read)\b")
        .map(|r| r.is_match(first))
        .unwrap_or(false)
}

// ── Session file operations ─────────────────────────────────────────────────

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
    // We need to own the new_entry string, so push it separately
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

// ── Public entry points ─────────────────────────────────────────────────────

/// PreToolUse hook handler. Exit 0 = allow, exit 2 = block.
pub fn pre() -> i32 {
    let input = read_stdin();
    let v: serde_json::Value = match serde_json::from_str(&input) {
        Ok(v) => v,
        Err(_) => return 0, // can't parse, allow through
    };

    // Detect tool type from JSON shape
    if let Some(_file_path) = json_str(&v, "tool_input.file_path") {
        // Edit tool — block
        let file = _file_path;
        eprintln!(
            "BLOCKED: Do not use the Edit tool in this project.\n\
             File: {}\n\
             Use: hashline apply\n\
             See CLAUDE.md.",
            file
        );
        return 2;
    }

    if json_str(&v, "tool_input.command").is_none() {
        // No command field, no file_path → assume NotebookEdit
        eprintln!(
            "BLOCKED: Do not use NotebookEdit in this project. \
             Use hashline apply via Bash. See CLAUDE.md."
        );
        return 2;
    }

    // Bash tool — check read-before-apply
    let cmd = json_str(&v, "tool_input.command").unwrap();
    if !is_apply_cmd(cmd) {
        return 0; // not a hashline apply, allow
    }

    let file = match extract_apply_file(cmd) {
        Some(f) => resolve_path(&f),
        None => return 0, // can't determine file, let hashline catch it
    };

    let session = session_path();
    let read_entry = format!("read:{}", file);
    let stale_entry = format!("stale:{}", file);

    if session_has(&session, &read_entry) {
        return 0; // freshly read
    }

    if session_has(&session, &stale_entry) {
        eprintln!(
            "BLOCKED: \"{}\" was modified by hashline apply but not re-read.\n\
             Anchors are stale. Run:\n  hashline read {}\nbefore applying edits.",
            file, file
        );
        return 2;
    }

    // File not in session at all
    eprintln!(
        "BLOCKED: \"{}\" has not been read with `hashline read` in this session.\n\
         Run:\n  hashline read {}\nbefore applying edits.",
        file, file
    );
    2
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
    } else if is_apply_cmd(&cmd) {
        if let Some(file) = extract_apply_file(&cmd) {
            let file = resolve_path(&file);
            if cmd.contains("--emit-updated") {
                mark_session(&session, &file, "read");
            } else {
                mark_session(&session, &file, "stale");
            }
        }
    }

    0
}
