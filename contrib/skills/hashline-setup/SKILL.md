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

Verify it supports the `hook` subcommand:

```bash
hashline hook --help
```

If this fails, the installed version is too old. Tell the user to upgrade.

## Step 2 — Merge hook configuration into the settings file

Read the existing settings file (if it exists) and merge in the following structure, preserving any existing keys. Use the Write tool to write the result. Do not overwrite unrelated permissions or hooks.

The hooks to add:

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

Rules for the merge:
- If `permissions.allow` already exists, append `"Bash(hashline:*)"` only if not already present.
- If hook matchers for `Edit`, `NotebookEdit`, or `Bash` already exist, do not duplicate them — skip any that are already registered.
- Write the final merged JSON to the target settings file with 2-space indentation.

## Step 3 — Verify the hooks work

Download and run the test suite:

```bash
mkdir -p .claude/hooks/tests
curl -fsSL https://raw.githubusercontent.com/lispmeister/hashline/main/contrib/hooks/tests/test_hooks.sh \
    -o .claude/hooks/tests/test_hooks.sh
chmod +x .claude/hooks/tests/test_hooks.sh
bash .claude/hooks/tests/test_hooks.sh
```

All tests must pass (output ends with `N passed, 0 failed`). If any fail, report the failures and do not proceed.

## Step 4 — Add hashline instructions to CLAUDE.md

Download the hashline editing template and prepend it to the project's `CLAUDE.md` so Claude knows how to use hashline instead of the Edit tool.

```bash
curl -fsSL https://raw.githubusercontent.com/lispmeister/hashline/main/HASHLINE_TEMPLATE.md \
    -o /tmp/hashline_template.md
```

Then read the project's existing `CLAUDE.md` (if it exists). Prepend everything in `/tmp/hashline_template.md` **after the `---` line** (the content below the frontmatter separator) to the top of `CLAUDE.md`. If `CLAUDE.md` does not exist, create it with just the template content (after the `---`).

Do not duplicate — if `CLAUDE.md` already contains the line `NEVER edit a file you haven't read with \`hashline read\``, skip this step.

## Step 5 — Report

Tell the user:
- Which settings file was updated
- That `CLAUDE.md` now contains hashline editing instructions
- That hooks are now active for this session (a restart may be needed for Claude Code to reload settings)
- How to test manually: `hashline apply` without a prior `hashline read` should be blocked

## Notes

- No external scripts or absolute paths are needed — all hook logic is built into the `hashline` binary.
- The test suite requires `jq` to be installed (for constructing test JSON; the hooks themselves do not use jq).
- See `HASHLINE_HOOKS.md` in the hashline repo for full documentation.
