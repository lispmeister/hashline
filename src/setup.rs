use crate::cli::SetupAgent;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;
use std::process::Command;

const HOOK_PRE_CMD: &str = "hashline hook pre";
const HOOK_POST_CMD: &str = "hashline hook post";
const CLAUDE_MARKER: &str = "NEVER edit a file you haven't read with `hashline read`";

pub fn run(agent: SetupAgent, settings_file: Option<&str>, dry_run: bool, run_tests: bool) -> i32 {
    match agent {
        SetupAgent::Claude => run_claude(settings_file, dry_run, run_tests),
        SetupAgent::Cursor | SetupAgent::Windsurf | SetupAgent::Generic => {
            println!(
                "Scaffold only: agent adapter '{}' is advisory-only for now.",
                agent_name(agent)
            );
            println!(
                "Next step: add the hashline instructions template to your agent rules file and use `hashline doctor --agent {}`.",
                agent_name(agent)
            );
            0
        }
    }
}

fn run_claude(settings_file: Option<&str>, dry_run: bool, run_tests: bool) -> i32 {
    let settings_path = settings_file.unwrap_or(".claude/settings.local.json");
    let mut settings = match load_json_or_empty(settings_path) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Error reading {}: {}", settings_path, e);
            return 2;
        }
    };

    merge_permissions(&mut settings);
    merge_hook(&mut settings, "PreToolUse", "Edit", HOOK_PRE_CMD);
    merge_hook(&mut settings, "PreToolUse", "NotebookEdit", HOOK_PRE_CMD);
    merge_hook(&mut settings, "PreToolUse", "Bash", HOOK_PRE_CMD);
    merge_hook(&mut settings, "PostToolUse", "Bash", HOOK_POST_CMD);

    let mut changed = false;

    if !dry_run {
        if let Err(e) = write_json_pretty(settings_path, &settings) {
            eprintln!("Error writing {}: {}", settings_path, e);
            return 2;
        }
        changed = true;
    }

    let template = embedded_template();
    let claudemd_path = Path::new("CLAUDE.md");
    let old_claude = fs::read_to_string(claudemd_path).unwrap_or_default();
    let needs_inject = !old_claude.contains(CLAUDE_MARKER);

    if needs_inject {
        let mut combined = String::new();
        combined.push_str(template);
        if !template.ends_with('\n') {
            combined.push('\n');
        }
        if !old_claude.is_empty() {
            combined.push('\n');
            combined.push_str(&old_claude);
        }
        if !dry_run {
            if let Err(e) = fs::write(claudemd_path, combined) {
                eprintln!("Error writing CLAUDE.md: {}", e);
                return 2;
            }
            changed = true;
        }
    }

    println!("hashline setup --agent claude");
    println!("- settings file: {}", settings_path);
    println!("- mode: {}", if dry_run { "dry-run" } else { "apply" });
    println!("- template source: embedded in installed hashline binary");
    if needs_inject {
        println!("- CLAUDE.md: hashline instructions inserted at top");
    } else {
        println!("- CLAUDE.md: already contains hashline instructions");
    }

    if run_tests {
        if dry_run {
            println!("- hook tests: skipped in dry-run");
        } else {
            match Command::new("bash")
                .arg("contrib/hooks/tests/test_hooks.sh")
                .status()
            {
                Ok(status) if status.success() => println!("- hook tests: PASS"),
                Ok(status) => {
                    eprintln!("- hook tests: FAIL (exit {})", status);
                    return 2;
                }
                Err(e) => {
                    eprintln!("- hook tests: ERROR ({})", e);
                    return 2;
                }
            }
        }
    }

    if !dry_run && changed {
        println!("Setup complete. Restart your agent session to pick up updated settings.");
    }

    0
}

fn load_json_or_empty(path: &str) -> Result<Value, Box<dyn std::error::Error>> {
    if !Path::new(path).exists() {
        return Ok(json!({}));
    }
    let text = fs::read_to_string(path)?;
    let v: Value = serde_json::from_str(&text)?;
    if v.is_object() {
        Ok(v)
    } else {
        Ok(json!({}))
    }
}

fn write_json_pretty(path: &str, value: &Value) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent)?;
    }
    let text = serde_json::to_string_pretty(value)?;
    fs::write(path, format!("{}\n", text))?;
    Ok(())
}

fn ensure_array<'a>(root: &'a mut Value, path: &[&str]) -> &'a mut Vec<Value> {
    let mut cur = root;
    for key in path {
        if cur.get(*key).is_none() {
            cur[*key] = json!({});
        }
        cur = &mut cur[*key];
    }
    if !cur.is_array() {
        *cur = json!([]);
    }
    cur.as_array_mut().expect("array expected")
}

fn merge_permissions(settings: &mut Value) {
    let allow = ensure_array(settings, &["permissions", "allow"]);
    let required = "Bash(hashline:*)";
    if !allow.iter().any(|v| v.as_str() == Some(required)) {
        allow.push(Value::String(required.to_string()));
    }
}

fn merge_hook(settings: &mut Value, event: &str, matcher: &str, command: &str) {
    let hooks = ensure_array(settings, &["hooks", event]);
    for entry in hooks.iter_mut() {
        let Some(obj) = entry.as_object_mut() else {
            continue;
        };
        if obj.get("matcher").and_then(|v| v.as_str()) != Some(matcher) {
            continue;
        }
        if !obj.get("hooks").map(|v| v.is_array()).unwrap_or(false) {
            obj.insert("hooks".to_string(), json!([]));
        }
        let arr = obj
            .get_mut("hooks")
            .and_then(|v| v.as_array_mut())
            .expect("hooks array expected");
        let exists = arr.iter().any(|h| {
            h.get("type").and_then(|v| v.as_str()) == Some("command")
                && h.get("command").and_then(|v| v.as_str()) == Some(command)
        });
        if !exists {
            arr.push(json!({"type":"command","command":command}));
        }
        return;
    }

    hooks.push(json!({
        "matcher": matcher,
        "hooks": [{"type": "command", "command": command}]
    }));
}

fn embedded_template() -> &'static str {
    let raw = include_str!("../HASHLINE_TEMPLATE.md");
    if let Some((_, tail)) = raw.split_once("\n---\n") {
        tail
    } else {
        raw
    }
}

fn agent_name(agent: SetupAgent) -> &'static str {
    match agent {
        SetupAgent::Claude => "claude",
        SetupAgent::Cursor => "cursor",
        SetupAgent::Windsurf => "windsurf",
        SetupAgent::Generic => "generic",
    }
}
