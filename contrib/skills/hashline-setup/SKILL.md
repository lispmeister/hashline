---
name: hashline-setup
description: Install hashline Claude Code hooks in the current project. Enforces read-before-apply, blocks the Edit tool, and tracks per-session file state. Use when setting up a new project to use hashline.
argument-hint: [settings-file: settings.json or settings.local.json (default)]
disable-model-invocation: true
---

Set up hashline Claude Code hooks in the current project by following these steps exactly.

## Determine target settings file

If `$ARGUMENTS` specifies `settings.json`, use `.claude/settings.json`.
Otherwise default to `.claude/settings.local.json`.

## Step 1 — Verify hashline is installed

```bash
hashline --version
```

If the command fails, stop and tell the user to install hashline first:
- Homebrew: `brew install lispmeister/hashline/hashline`
- From source: `cargo install --path .` (if in the hashline repo)
- Script: `curl -fsSL https://raw.githubusercontent.com/lispmeister/hashline/main/install.sh | sh`

## Step 2 — Install hook scripts

```bash
mkdir -p .claude/hooks/tests
curl -fsSL https://raw.githubusercontent.com/lispmeister/hashline/main/.claude/hooks/check_before_apply.sh \
    -o .claude/hooks/check_before_apply.sh
curl -fsSL https://raw.githubusercontent.com/lispmeister/hashline/main/.claude/hooks/track_hashline.sh \
    -o .claude/hooks/track_hashline.sh
curl -fsSL https://raw.githubusercontent.com/lispmeister/hashline/main/.claude/hooks/tests/test_hooks.sh \
    -o .claude/hooks/tests/test_hooks.sh
chmod +x .claude/hooks/check_before_apply.sh .claude/hooks/track_hashline.sh .claude/hooks/tests/test_hooks.sh
```

## Step 3 — Determine the absolute project path

```bash
pwd
```

Use this value as `PROJECT_ROOT` in hook command paths. Hook commands require absolute paths.

## Step 4 — Merge hook configuration into the settings file

Read the existing settings file (if it exists) and merge in the following structure, preserving any existing keys. Use the Write tool to write the result. Do not overwrite unrelated permissions or hooks.

The hooks to add (substitute the real absolute path for `PROJECT_ROOT`):

```json
{
  "permissions": {
    "allow": [
      "Bash(hashline:*)"
    ]
  },
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
          "command": "bash PROJECT_ROOT/.claude/hooks/check_before_apply.sh"
        }]
      }
    ],
    "PostToolUse": [
      {
        "matcher": "Bash",
        "hooks": [{
          "type": "command",
          "command": "bash PROJECT_ROOT/.claude/hooks/track_hashline.sh"
        }]
      }
    ]
  }
}
```

Rules for the merge:
- If `permissions.allow` already exists, append `"Bash(hashline:*)"` only if not already present.
- If hook matchers for `Edit`, `NotebookEdit`, or `Bash` already exist, do not duplicate them — skip any that are already registered.
- Write the final merged JSON to the target settings file with 2-space indentation.

## Step 5 — Verify the hooks work

```bash
bash .claude/hooks/tests/test_hooks.sh
```

All tests must pass (output ends with `N passed, 0 failed`). If any fail, report the failures and do not proceed.

## Step 6 — Report

Tell the user:
- Which settings file was updated
- The absolute path used for hook commands
- That hooks are now active for this session (a restart may be needed for Claude Code to reload settings)
- How to test manually: `hashline apply` without a prior `hashline read` should be blocked

## Notes

- `.claude/` is commonly gitignored. If the user wants the hook scripts tracked in git, they must `git add -f .claude/hooks/`.
- See `HASHLINE_HOOKS.md` in the hashline repo for full documentation.
- The test suite requires `jq` to be installed.
