# Hashline Claude Code Hooks

This guide explains how to add Claude Code hooks to any project that uses hashline, so the rules in `CLAUDE.md` are mechanically enforced rather than merely stated.

## What the hooks do

| Hook | Event | Effect |
|---|---|---|
| Block `Edit` tool | `PreToolUse/Edit` | Hard block (exit 2) — redirects to `hashline apply` or `hashline json-apply` |
| Block `NotebookEdit` tool | `PreToolUse/NotebookEdit` | Hard block (exit 2) |
| Enforce read-before-apply | `PreToolUse/Bash` | Blocks `hashline apply` or `hashline json-apply` if the target file has not been read with the corresponding read command in the current session, or if its anchors are stale after a prior apply |
| Track session state | `PostToolUse/Bash` | After each `hashline read`, `hashline json-read`, `hashline apply`, or `hashline json-apply`, updates a per-session file that records which files have fresh anchors and which are stale |

The hooks enforce rules 1 and 2 below. Rules 3 and 4 are not mechanically enforceable.

| Rule | Enforced? |
|---|---|
| Don't use the Edit tool | ✅ Hard block |
| Read before apply; re-read after apply (includes JSON read/apply) | ✅ Block with clear error |
| Batch all edits to one file in one apply call | ❌ (advisory only) |
| Prefer anchor ops over `replace` (and semantic JSON ops over line-based) | ❌ (advisory only) |

## Quick install via skill

If you use Claude Code, the fastest way to set up hashline hooks in any project is the bundled skill:

```
/hashline-setup
```

This installs the hook scripts, registers them in `.claude/settings.local.json`, and runs the test suite to verify everything works. See `.claude/skills/hashline-setup/SKILL.md` for the full instructions the skill executes.

To install the skill globally (available in all your projects):

```sh
mkdir -p ~/.claude/skills/hashline-setup
curl -fsSL https://raw.githubusercontent.com/lispmeister/hashline/main/.claude/skills/hashline-setup/SKILL.md \
    -o ~/.claude/skills/hashline-setup/SKILL.md
```


## Installation

### 1. Copy the hook scripts

```sh
mkdir -p .claude/hooks
curl -fsSL https://raw.githubusercontent.com/lispmeister/hashline/main/.claude/hooks/track_hashline.sh \
    -o .claude/hooks/track_hashline.sh
curl -fsSL https://raw.githubusercontent.com/lispmeister/hashline/main/.claude/hooks/check_before_apply.sh \
    -o .claude/hooks/check_before_apply.sh
chmod +x .claude/hooks/track_hashline.sh .claude/hooks/check_before_apply.sh
```

Or copy them manually from this repo's `.claude/hooks/` directory.

### 2. Add the hook registrations to `.claude/settings.json`

If `.claude/` is gitignored in your project (common), use `settings.local.json` instead.

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Edit",
        "hooks": [{
          "type": "command",
          "command": "file=$(cat | jq -r '.tool_input.file_path // \"(unknown)\"'); printf 'BLOCKED: Do not use the Edit tool in this project.\\nFile: %s\\nUse: hashline apply\\nSee CLAUDE.md.\\n' \"$file\" >&2; exit 2"
        }]
      },
      {
        "matcher": "NotebookEdit",
        "hooks": [{
          "type": "command",
          "command": "echo 'BLOCKED: Do not use NotebookEdit in this project. Use hashline apply via Bash. See CLAUDE.md.' >&2; exit 2"
        }]
      },
      {
        "matcher": "Bash",
        "hooks": [{
          "type": "command",
          "command": "bash /absolute/path/to/.claude/hooks/check_before_apply.sh"
        }]
      }
    ],
    "PostToolUse": [
      {
        "matcher": "Bash",
        "hooks": [{
          "type": "command",
          "command": "bash /absolute/path/to/.claude/hooks/track_hashline.sh"
        }]
      }
    ]
  }
}
```

Replace `/absolute/path/to` with the absolute path to your project root. Relative paths are not supported in hook commands because the working directory may vary.

### 3. Add permissions for hashline commands

In the same `settings.json` or `settings.local.json`, ensure hashline is allowed:

```json
{
  "permissions": {
    "allow": [
      "Bash(hashline:*)"
    ]
  }
}
```

## How session tracking works

The two Bash hook scripts share a session file at `/tmp/hashline_session_<PPID>`, where `PPID` is the process ID of the Claude Code process (which is the parent of all hook subprocesses). This gives each Claude Code session its own isolated tracking state.

**Entries in the session file:**

- `read:<absolute-path>` — file was read with `hashline read`; anchors are fresh
- `stale:<absolute-path>` — file was modified by `hashline apply` without `--emit-updated`; anchors are stale and must be refreshed before the next apply

**State transitions:**

```
(not in session)
      │  hashline read
      ▼
   read:<file>
      │  hashline apply (no --emit-updated)
      ▼
  stale:<file>
      │  hashline read  (or apply --emit-updated)
      ▼
   read:<file>
```

## Known limitations

**PPID-based session isolation** works correctly when a single Claude Code process manages the session. It has two edge cases:

1. **PID reuse**: If Claude Code exits and a new process reuses the same OS PID before `/tmp` is cleaned, the new session might inherit the old tracking state. In practice this is rare; the worst outcome is a spurious "already read" entry from a previous session.

2. **Worktrees**: Each Claude Code process in a separate worktree gets its own PPID, so they do not share session state. This is the desired behavior.

**Path normalization**: The hooks resolve relative file paths against `$PWD` (the working directory of the hook process). For `hashline read src/main.rs` and `hashline apply` with `"path": "src/main.rs"`, both resolve to the same absolute path. If for some reason the working directory differs between the read and the apply commands, the lookup may fail. The safe approach is to use the same path format (absolute or relative from project root) consistently.

**Heredoc path extraction**: When `hashline apply` is called with a heredoc, the hook extracts the `"path"` field from the embedded JSON using a regex. This works for standard single-file payloads. If the JSON is minified, deeply nested, or the `path` key is on the same line as other fields in an unusual order, extraction may fail — in which case the hook allows the apply through and relies on hashline's own anchor verification to catch stale anchors.

**Batching**: The hooks cannot enforce "all edits to one file in one apply call". This remains an advisory rule.

## Testing the hooks

A test suite is included at `.claude/hooks/tests/test_hooks.sh`. Run it from the project root:

```sh
bash .claude/hooks/tests/test_hooks.sh
```

The tests feed synthetic PreToolUse/PostToolUse JSON to the scripts and verify exit codes and session state. No external test framework is required.

**Design note on test isolation**: Hook scripts key their session file on `$PPID`. Test scripts must invoke hooks as direct children (not inside pipes or command substitutions) to ensure `PPID == $$`. The test harness uses temporary files for stdin/stderr capture instead of pipes to preserve this invariant.
