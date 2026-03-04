# PLAN.md — Hashline Rust Implementation

## Problem Statement

AI coding assistants suffer from a harness problem: the interface between model output and workspace edits is where most practical failures occur. Traditional diff, raw string replacement, and neural merge approaches are brittle, forcing models to reproduce exact text or rely on heavyweight tooling.

Hashline tags each line with a short content-addressable anchor (`LINE:HASH`). Agents reference anchors instead of reproducing content, so stale edits are detected before any mutation. Benchmarks show ~10x improvement for weaker models and ~20% token savings across the board.

## How Hashline Works

### Encoding

Each line is rendered as `LINENUM:HASH|CONTENT`, where:

- Line numbers are 1-indexed.
- Hashes are 2-char lowercase hex derived from `xxHash32(normalized_line) % 256`.
- The pipe separates the anchor from the content.

Example:

```
1:a3|function hello() {
2:f1|  return "world";
3:0e|}
```

### Edit Operations

Agents batch edits in JSON, referencing anchors collected via `hashline read`:

1. `set_line` — replace a single line.
2. `replace_lines` — replace or delete a range of lines.
3. `insert_after` — insert lines after an anchor.
4. `replace` — exact substring replacement (escape hatch when anchors are awkward).

All edits validate against the original file before mutating disk.

### Key Properties

- Atomic batch apply (all-or-nothing per file).
- Bottom-up splice ordering keeps indices stable.
- Hash mismatch detection returns updated anchors with context.
- Heuristics strip accidental prefixes, restore indentation, handle merged lines, and normalize confusable characters.

## Current Priorities (2026-03-04)

### High Priority — Agent Integration Improvements

**Problem:** The install flow has several gaps that make it hard for agents (and humans) to get hashline working end-to-end with Claude Code.

**Analysis:**

1. **SKILL.md downloads hooks from wrong URLs** — Step 2 references `.claude/hooks/` on GitHub but the files live at `contrib/hooks/`. The skill fails on install.
2. **Dead code in hook scripts** — Both `check_before_apply.sh:63` and `track_hashline.sh:92` have `rm -f /tmp/hashline_session_*` after an unconditional `exit`. Never executes.
3. **No CLAUDE.md template injection** — The skill installs hooks that block the Edit tool but never injects the HASHLINE_TEMPLATE.md content into CLAUDE.md. The agent is blocked from editing but has no instructions on how to use hashline instead.
4. **README doesn't mention CLAUDE.md setup** — Users who install via the skill don't know that CLAUDE.md instructions are also needed/provided.

**Tasks:**

- [x] Fix SKILL.md hook download URLs (`contrib/hooks/`, not `.claude/hooks/`)
- [x] Remove dead code from both hook scripts
- [x] Add CLAUDE.md template injection step to SKILL.md
- [x] Update README agent integration section to mention CLAUDE.md setup

### Medium Priority

- Move hook logic into `hashline hook pre` / `hashline hook post` subcommands (eliminates bash scripts, jq dependency, absolute path requirement). Track in a separate PR.

### Low Priority
- _None._

### Recently Completed (2026-03-04)

- Agent integration improvements: fixed SKILL.md URLs, removed dead code from hooks, added CLAUDE.md template injection, updated README.

### Recently Completed (2026-02-26)

- Fixed JSON anchor encoding for special keys (bracket notation) with new unit and CLI coverage.
- Made `--emit-updated` previews reliable for replace-only edits and plumbed the logic through the CLI.
- Added CLI/integration coverage for JSON workflows, including mismatch diagnostics and special-key round trips.
- Refreshed README/AGENTS/HASHLINE_TEMPLATE docs to highlight `--input`, `--emit-updated`, and bracket notation.
- Consolidated file reading via `util::read_normalized` and switched tests to `NamedTempFile`.
- Updated CLI help, cli_help.md, and HASHLINE_HOOKS.md to push the `--emit-updated --input` workflow and bracket-notation anchors.
- Added CLI usage instrumentation with opt-out env vars and documented log locations.
- Enforced exclusive key/index handling for `insert_at_path` and documented the rule across templates.
- Introduced tests/fixtures/json/large.json with regression coverage for deep anchors.
- Documented the Homebrew tap automation plan in contrib/HOMEBREW_AUTOMATION.md.




## Notes

- Always reinstall (`cargo install --path .`) after touching `src/` so the `hashline` binary used by hooks/tests matches the workspace.
- Prefer `hashline apply --input <file>` to avoid heredoc guardrails; `--emit-updated` is optional but should become the default verification path once the preview work above lands.

