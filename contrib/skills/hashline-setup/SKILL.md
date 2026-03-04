---
name: hashline-setup
description: Install hashline Claude Code hooks in the current project. Enforces read-before-apply, blocks the Edit tool, and tracks per-session file state. Use when setting up a new project to use hashline.
argument-hint: [settings-file: settings.json or settings.local.json (default)]
disable-model-invocation: true
---

Set up hashline Claude Code integration in the current project by using hashline's built-in setup command.

## Step 1 — Verify hashline is installed

```bash
hashline --version
```

If this fails, stop and tell the user to install hashline first.

## Step 2 — Resolve settings target

If `$ARGUMENTS` is `settings.json`, use `--settings-file .claude/settings.json`.
Otherwise, use the default (`.claude/settings.local.json`).

## Step 3 — Run setup

Default:

```bash
hashline setup --agent claude --run-tests
```

If `settings.json` was requested:

```bash
hashline setup --agent claude --settings-file .claude/settings.json --run-tests
```

This command is idempotent and will:
- Merge required permissions/hooks into the target settings file
- Inject hashline instructions into `CLAUDE.md` (without duplicating)
- Run hook tests (`contrib/hooks/tests/test_hooks.sh`)

## Step 4 — Verify with doctor

```bash
hashline doctor --agent claude --simulate
```

If doctor reports failures, surface them to the user and stop.

## Step 5 — Report
Tell the user:
- Which settings file was used
- That `CLAUDE.md` contains hashline instructions
- That they may need to restart Claude Code for settings reload
- That `hashline apply` without prior read should now be blocked by hooks

## Notes

- Uses embedded setup assets from the installed hashline version (no remote downloads required).
- Hook runtime has no jq dependency; test harness still uses jq.
- For manual details, see `HASHLINE_HOOKS.md`.

