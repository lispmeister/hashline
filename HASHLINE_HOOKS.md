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

## Recommended CLI usage

Prefer writing payloads to disk and invoking `hashline apply --emit-updated --input edits.json` (and `hashline json-apply --emit-updated --input json-edits.json`). `--emit-updated` keeps anchors fresh without a follow-up read, and `--input` avoids heredoc guardrails. Heredocs still work for simple payloads, but bracket-notation anchors (e.g. `$["a.b"]["c d"]`) are easier to read when they live in a file.


## Quick install via skill

If you use Claude Code, the fastest way to set up hashline hooks in any project is the bundled skill:

```
/hashline-setup
```

This registers the hooks in `.claude/settings.local.json` and runs the test suite to verify everything works. See `contrib/skills/hashline-setup/SKILL.md` for the full instructions the skill executes.

To install the skill globally (available in all your projects):

```sh
mkdir -p ~/.claude/skills/hashline-setup
curl -fsSL https://raw.githubusercontent.com/lispmeister/hashline/main/contrib/skills/hashline-setup/SKILL.md \
    -o ~/.claude/skills/hashline-setup/SKILL.md
```


## Installation

### 1. Add the hook registrations to `.claude/settings.json`

If `.claude/` is gitignored in your project (common), use `settings.local.json` instead.

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Edit",
        "hooks": [{
          "type": "command",
          "command": "hashline hook pre"
        }]
      },
      {
        "matcher": "NotebookEdit",
        "hooks": [{
          "type": "command",
          "command": "hashline hook pre"
        }]
      },
      {
        "matcher": "Bash",
        "hooks": [{
          "type": "command",
          "command": "hashline hook pre"
        }]
      }
    ],
    "PostToolUse": [
      {
        "matcher": "Bash",
        "hooks": [{
          "type": "command",
          "command": "hashline hook post"
        }]
      }
    ]
  }
}
```

No absolute paths or external scripts needed — all hook logic is built into the `hashline` binary.

### 2. Add permissions for hashline commands

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

The hook subcommands share a session file at `/tmp/hashline_session_<PPID>`, where `PPID` is the parent process ID. You can override this path with `HASHLINE_SESSION_FILE` (used by diagnostics/tests).

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

## Tool detection

`hashline hook pre` detects which tool triggered the hook from the JSON shape:

- If `tool_input.file_path` exists → **Edit tool** → block (exit 2)
- If `tool_input.command` exists → **Bash tool** → check read-before-apply
- Otherwise → **NotebookEdit** → block (exit 2)

This means all three PreToolUse matchers can use the same `hashline hook pre` command.

## Known limitations

**PPID-based session isolation** works correctly when a single Claude Code process manages the session. It has two edge cases:

1. **PID reuse**: If Claude Code exits and a new process reuses the same OS PID before `/tmp` is cleaned, the new session might inherit the old tracking state. In practice this is rare; the worst outcome is a spurious "already read" entry from a previous session.

2. **Worktrees**: Each Claude Code process in a separate worktree gets its own PPID, so they do not share session state. This is the desired behavior.

**Path normalization**: The hooks resolve relative file paths against the current working directory. For `hashline read src/main.rs` and `hashline apply` with `"path": "src/main.rs"`, both resolve to the same absolute path. If for some reason the working directory differs between the read and the apply commands, the lookup may fail. The safe approach is to use the same path format (absolute or relative from project root) consistently.

**Heredoc path extraction**: When `hashline apply` is called with a heredoc, the hook extracts the `"path"` field from embedded JSON via regex. This works for standard payloads. If extraction fails, default mode allows the apply through and relies on hashline's own anchor checks; strict mode (`HASHLINE_HOOK_STRICT=1`) blocks unresolved targets fail-closed.

**Batching**: The hooks cannot enforce "all edits to one file in one apply call". This remains an advisory rule.

## Testing the hooks

A test suite is included at `contrib/hooks/tests/test_hooks.sh`. Run it from the project root:

```sh
bash contrib/hooks/tests/test_hooks.sh
```

The tests feed synthetic PreToolUse/PostToolUse JSON to `hashline hook pre` and `hashline hook post` and verify exit codes and session state. The test harness requires `jq` for constructing test JSON (the hooks themselves do not).

**Design note on test isolation**: Hook subcommands key their session file on `getppid()`. Test scripts must invoke `hashline` as direct children (not inside pipes or command substitutions) to ensure the PPID equals the test script's PID. The test harness uses temporary files for stdin capture instead of pipes to preserve this invariant.
