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

## Current Priorities (2026-02-26)

### Blocking

- **[BLOCKER] Fix JSON anchor encoding for special keys** — `hashline json-read` emits anchors like `$.a.b` for a literal key "a.b", but `json-apply` splits on `.`, so the edit fails. Switch to bracket notation (e.g. `$["a.b"]`), handle keys containing dots/brackets/quotes, and add regression tests covering library and CLI paths.

### High Priority

- **[HIGH] Make `--emit-updated` previews reliable** — capture the earliest modified line even when only `replace` edits run, share the same logic with `json-apply`, and add tests so replace-only flows produce useful context.
- **[HIGH] Add CLI/integration coverage for the JSON workflow** — exercise `json-read`, `json-apply`, hash mismatch diagnostics, the dotted-key fix above, and `--emit-updated`; run through the CLI rather than only unit tests.
- **[HIGH] Refresh docs and prompts** — update README, CLI help, `AGENTS.md`, `HASHLINE_TEMPLATE.md`, and `HASHLINE_HOOKS.md` to emphasise `--input` usage, document JSON mismatch output, and remove statements that are now false (e.g. "`--start-line` not implemented").
- **[HIGH] Finish the CLAUDE.md pruning + hook onboarding pass** — keep only what hooks cannot enforce, verify the skill/template instructions reference the new file locations, and ensure json-aware hooks are documented.

### Medium Priority

- **[MED] Replace `/tmp/...` usage in unit tests** — e.g. `src/util.rs` still relies on hard-coded `/tmp/` paths; switch to `tempfile::NamedTempFile` to keep the suite portable.
- **[MED] Consolidate file-reading normalization** — `src/main.rs` duplicates the same CRLF stripping/trailing newline trimming in five commands; extract a shared helper in `util.rs`.
- **[MED] Clarify `InsertAtPathOp` semantics** — split object vs array insertion or fail loudly when both `key` and `index` are provided; add coverage once the API is explicit.
- **[MED] Instrument hashline usage** — lightweight logging (read/apply counts, mismatch rates, `--emit-updated` adoption) to inform future prioritisation.

### Low Priority

- **[LOW] Revisit the large JSON fixture** — either wire it into a benchmark/fuzz target or remove it to keep the repo lean.
- **[LOW] Homebrew tap automation** — only tackle once core features stabilise; keep instructions but mark as backlog.
- **[LOW] Tidy residual session comments** — remove `// (fix N)` breadcrumbs in `src/json.rs` and replace with meaningful section headers.

### Recently Completed

- JSON engine was ported to Rust (`apply_json_edits`, canonical hashing, CLI plumbing).
- Hook scripts now track both text and JSON commands with regression tests in `contrib/hooks/tests/test_hooks.sh`.
- Fuzz tests cover core text workflows (hashing, formatting, parsing, edit application).

## Notes

- Always reinstall (`cargo install --path .`) after touching `src/` so the `hashline` binary used by hooks/tests matches the workspace.
- Prefer `hashline apply --input <file>` to avoid heredoc guardrails; `--emit-updated` is optional but should become the default verification path once the preview work above lands.

