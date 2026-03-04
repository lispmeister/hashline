use crate::cli::SetupAgent;
use serde_json::Value;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

const CLAUDE_MARKER: &str = "NEVER edit a file you haven't read with `hashline read`";

pub fn run(agent: SetupAgent, simulate: bool) -> i32 {
    match agent {
        SetupAgent::Claude => run_claude(simulate),
        SetupAgent::Cursor | SetupAgent::Windsurf | SetupAgent::Generic => {
            println!("doctor [{}]", agent_name(agent));
            println!("- adapter status: scaffold-only (advisory mode)");
            println!("- suggestion: add hashline template to agent rules and run manual edit loop checks");
            0
        }
    }
}

fn run_claude(simulate: bool) -> i32 {
    let mut failed = false;
    println!("doctor [claude]");

    let version = env!("CARGO_PKG_VERSION");
    println!("- binary version: {}", version);

    let settings_path = if Path::new(".claude/settings.local.json").exists() {
        ".claude/settings.local.json"
    } else if Path::new(".claude/settings.json").exists() {
        ".claude/settings.json"
    } else {
        ""
    };

    if settings_path.is_empty() {
        println!("- settings: FAIL (no .claude/settings.local.json or .claude/settings.json)");
        failed = true;
    } else {
        println!("- settings: OK ({})", settings_path);
        match fs::read_to_string(settings_path)
            .ok()
            .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        {
            Some(v) => {
                let perm_ok = has_allow_permission(&v, "Bash(hashline:*)");
                print_check("permission Bash(hashline:*)", perm_ok, &mut failed);

                let pre_edit = has_hook_command(&v, "PreToolUse", "Edit", "hashline hook pre");
                let pre_nb =
                    has_hook_command(&v, "PreToolUse", "NotebookEdit", "hashline hook pre");
                let pre_bash = has_hook_command(&v, "PreToolUse", "Bash", "hashline hook pre");
                let post_bash = has_hook_command(&v, "PostToolUse", "Bash", "hashline hook post");

                print_check("hook PreToolUse/Edit", pre_edit, &mut failed);
                print_check("hook PreToolUse/NotebookEdit", pre_nb, &mut failed);
                print_check("hook PreToolUse/Bash", pre_bash, &mut failed);
                print_check("hook PostToolUse/Bash", post_bash, &mut failed);
            }
            None => {
                println!("- settings JSON parse: FAIL");
                failed = true;
            }
        }
    }

    let claudemd_ok = fs::read_to_string("CLAUDE.md")
        .map(|s| s.contains(CLAUDE_MARKER))
        .unwrap_or(false);
    print_check("CLAUDE.md hashline template", claudemd_ok, &mut failed);

    if simulate {
        match run_simulation() {
            Ok(true) => println!("- simulated hook enforcement: OK"),
            Ok(false) => {
                println!("- simulated hook enforcement: FAIL");
                failed = true;
            }
            Err(e) => {
                println!("- simulated hook enforcement: ERROR ({})", e);
                failed = true;
            }
        }
    }

    if failed {
        2
    } else {
        0
    }
}

fn print_check(name: &str, ok: bool, failed: &mut bool) {
    if ok {
        println!("- {}: OK", name);
    } else {
        println!("- {}: FAIL", name);
        *failed = true;
    }
}

fn has_allow_permission(v: &Value, needle: &str) -> bool {
    v.get("permissions")
        .and_then(|p| p.get("allow"))
        .and_then(|a| a.as_array())
        .map(|arr| arr.iter().any(|x| x.as_str() == Some(needle)))
        .unwrap_or(false)
}

fn has_hook_command(v: &Value, event: &str, matcher: &str, command: &str) -> bool {
    let Some(entries) = v
        .get("hooks")
        .and_then(|h| h.get(event))
        .and_then(|x| x.as_array())
    else {
        return false;
    };

    entries.iter().any(|entry| {
        entry.get("matcher").and_then(|x| x.as_str()) == Some(matcher)
            && entry
                .get("hooks")
                .and_then(|h| h.as_array())
                .map(|hs| {
                    hs.iter().any(|h| {
                        h.get("type").and_then(|x| x.as_str()) == Some("command")
                            && h.get("command").and_then(|x| x.as_str()) == Some(command)
                    })
                })
                .unwrap_or(false)
    })
}

fn run_simulation() -> Result<bool, Box<dyn std::error::Error>> {
    let tmp = std::env::temp_dir();
    let session = tmp.join(format!("hashline_doctor_session_{}", std::process::id()));
    let target = tmp.join(format!("hashline_doctor_target_{}.txt", std::process::id()));
    let edits = tmp.join(format!("hashline_doctor_edits_{}.json", std::process::id()));

    fs::write(&target, "x\n")?;
    fs::write(
        &edits,
        format!("{{\"path\":\"{}\",\"edits\":[]}}", target.to_string_lossy()),
    )?;

    let payload = format!(
        "{{\"tool_input\":{{\"command\":\"hashline apply --input {}\"}}}}",
        edits.to_string_lossy()
    );

    let exe = std::env::current_exe()?;
    let mut child = Command::new(exe)
        .args(["hook", "pre"])
        .env("HASHLINE_SESSION_FILE", &session)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    if let Some(stdin) = child.stdin.as_mut() {
        stdin.write_all(payload.as_bytes())?;
    }

    let status = child.wait()?;

    let _ = fs::remove_file(&session);
    let _ = fs::remove_file(&target);
    let _ = fs::remove_file(&edits);

    Ok(status.code() == Some(2))
}

fn agent_name(agent: SetupAgent) -> &'static str {
    match agent {
        SetupAgent::Claude => "claude",
        SetupAgent::Cursor => "cursor",
        SetupAgent::Windsurf => "windsurf",
        SetupAgent::Generic => "generic",
    }
}
