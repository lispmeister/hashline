# PLAN.md — Hashline Rust Implementation

## Problem Statement

AI coding assistants suffer from a **harness problem**: the interface between model output and workspace edits is where most practical failures occur. Current approaches all have fundamental weaknesses:

- **Patch/diff format** — strict formatting rules that models frequently violate (50%+ failure rate for some models)
- **String replacement** — requires verbatim character-perfect reproduction of existing code, including whitespace; multiple matches cause rejection
- **Neural merge** — requires fine-tuning separate large models just for edit application

**Hashline** solves this by giving each line a short content-addressable hash tag (`LINE:HASH`). Models reference lines by hash anchors instead of reproducing content verbatim. Hash mismatches after file changes prevent silent corruption. Benchmark results show 10x improvement for weaker models and ~20% token reduction across all models.

## How Hashline Works

### Encoding

Each line gets a prefix: `LINENUM:HASH|CONTENT`

```
1:a3|function hello() {
2:f1|  return "world";
3:0e|}
```

- Line numbers are 1-indexed
- Hash is 2 hex characters derived from `xxHash32(whitespace_normalized_line) % 256`
- The `|` separates the anchor from the content

### Edit Operations

Four operations, all referencing anchors from the original file state:

1. **`set_line`** — replace a single line: `{ anchor: "2:f1", new_text: "  return 42;" }`
2. **`replace_lines`** — replace a contiguous range: `{ start_anchor: "1:a3", end_anchor: "3:0e", new_text: "..." }`
3. **`insert_after`** — add lines after an anchor: `{ anchor: "3:0e", text: "// end" }`
4. **`replace`** — substring fuzzy match fallback (no hashes needed)

### Key Properties

- **Atomicity**: All edits validate against the original file state before any mutation
- **Bottom-up application**: Edits sorted by descending line number so splices don't invalidate indices
- **Staleness detection**: Hash mismatch → error with updated `LINE:HASH` refs so model can retry
- **Heuristic recovery**: Strips accidental hashline prefixes from model output, restores indentation, detects line merges, normalizes confusable Unicode hyphens

## Source Code Location

The reference implementation lives in [can1357/oh-my-pi](https://github.com/can1357/oh-my-pi):

| File | Purpose |
|------|---------|
| `packages/coding-agent/src/patch/hashline.ts` | Core implementation (992 lines) — hashing, formatting, parsing, edit application, heuristics |
| `packages/coding-agent/src/prompts/tools/hashline.md` | Tool prompt given to models describing the edit format |
| `packages/coding-agent/test/core/hashline.test.ts` | Comprehensive test suite |
| `packages/react-edit-benchmark/src/verify.ts` | Benchmark verification engine |
| `packages/react-edit-benchmark/src/formatter.ts` | Prettier-based formatting for benchmark |

## Implementation Plan

### Phase 1: Core Rust Library (`hashline-core`)

A Rust crate implementing the hashline algorithm.

#### Task 1.1: Hash computation
- Implement `compute_line_hash(line: &str) -> String`
- Normalize: strip all whitespace, compute xxHash32, mod 256, format as 2-char hex
- Strip trailing `\r` before normalization

#### Task 1.2: Hashline formatting
- `format_hashlines(content: &str, start_line: usize) -> String`
- Streaming variants: out of scope

#### Task 1.3: Line reference parsing
- `parse_line_ref(ref_str: &str) -> Result<LineRef>`
- Handle display-format suffixes (`5:ab|content`), legacy format (`5:ab  content`), `>>>` prefixes
- Validation: line >= 1, hash is 1-16 hex/alphanumeric chars

#### Task 1.4: Edit application engine
- Parse edit operations (set_line, replace_lines, insert_after)
- Pre-validate all hashes before any mutation
- Hash relocation: if a ref's hash doesn't match at its line number but is unique in the file, relocate
- Sort edits bottom-up (descending line number)
- Deduplicate identical edits
- Apply splices

#### Task 1.5: Heuristic recovery layer
- Strip accidental hashline prefixes from model output
- Strip diff `+` prefixes
- Restore leading indentation from template lines
- Detect single-line merges (model merges adjacent continuation lines)
- Undo pure formatting rewraps (model reflows one logical line into multiple)
- Normalize confusable Unicode hyphens (en-dash, em-dash, etc. → ASCII hyphen)
- Strip boundary echo (model echoes lines above/below the edit range)
- Strip insert-after anchor echo

#### Task 1.6: Error reporting
- `HashlineMismatchError` with context lines (2 above/below), `>>>` markers on changed lines
- Show correct `LINE:HASH` so model can retry with updated refs
- No-op detection (replacement identical to current content)

### Phase 2: CLI Binary (`hashline`)

A standalone binary that can be invoked by Claude Code (or any tool harness).

#### Task 2.1: `hashline read <file>`
- Read file, output hashline-formatted content to stdout
- Options: `--start-line N`, `--lines N` (range), `--chunk-lines N`, `--chunk-bytes N`

#### Task 2.5: Implement `--start-line` and `--lines` for `hashline read`
- The `--start-line N` and `--lines N` options from Task 2.1 were never implemented
- After editing a large file, agents need to verify just the changed region without re-reading the entire file
- Without this, agents must fall back to the built-in Read tool for partial reads, breaking the hashline workflow
- Accept `hashline read --start-line 130 --lines 25 <file>` to output only that range with correct LINE:HASH anchors

#### Task 2.2: `hashline apply <file>`
- Read edit operations from stdin (JSON)
- Apply to file, write result back
- Output: modified content or structured error (hash mismatches with updated refs)
- Exit codes: 0 = success, 1 = hash mismatch (stderr has updated refs), 2 = other error

#### Task 2.3: `hashline hash <file>`
- Output just the line hashes (for tooling/debugging)

#### Task 2.4: JSON schema for edits
- Define the edit operation JSON format matching the TypeScript types
- Document the schema in the binary's help output

#### Task 2.6: Homebrew formula
- Create a Homebrew formula so users can install via `brew install hashline`
- Options:
  - **Tap** (recommended for now): create a `homebrew-hashline` tap repo under the same GitHub org, add formula that downloads the release tarball and verifies SHA256
  - **Core**: out of scope
- Formula should install the binary and the man pages (`man/hashline*.1`)
- Update README install section with `brew install lispmeister/hashline/hashline`
- CI should not be blocked on this — it's a distribution convenience, not a correctness requirement

### Phase 3: Integration with Claude Code

A plain CLI tool invoked via Bash — no MCP server, no fork, no new tool registration needed.

#### Task 3.1: CLAUDE.md instructions
Add to the project (or global `~/.claude/CLAUDE.md`) instructions that tell Claude to use hashline instead of the built-in Edit tool:

```markdown
# Editing files
For all code edits, use the hashline CLI instead of the built-in Edit tool:
- Read: `hashline read <file>` (returns LINE:HASH|content format)
- Edit: `echo '{"path":"<file>","edits":[...]}' | hashline apply`
- After every edit, re-read before editing the same file again (hashes changed)
- On hash mismatch errors, use the updated LINE:HASH refs from stderr and retry
```

#### Task 3.2: Tool prompt
- Adapt the existing `hashline.md` prompt (from the TS repo) into the CLAUDE.md instructions
- Include the edit operation formats, scope rules, and recovery procedures
- Keep it concise — Claude follows CLAUDE.md reliably

#### Task 3.3: Workflow (all via Bash)
1. Claude runs `hashline read src/foo.rs` → stdout returns `LINE:HASH|content`
2. Claude collects `LINE:HASH` anchors for lines it wants to change
3. Claude pipes JSON edits to `hashline apply` → file is modified in place
4. On hash mismatch: exit code 1, stderr has updated refs with `>>>` markers, Claude retries
5. After edit: Claude re-reads before editing the same file again

### Phase 4: Testing & Validation

#### Task 4.1: Port the TypeScript test suite to Rust
- All hash computation tests
- All formatting tests
- All parse/validate tests
- All edit application tests including heuristic edge cases

#### Task 4.2: Fuzz testing — DONE
- Property-based fuzzing via `proptest` in `tests/fuzz.rs` (12 tests, runs on stable Rust)
- Covers: `compute_line_hash` (no panics, 2-hex-char invariant, whitespace invariant, index ignored)
- Covers: `parse_line_ref` (no panics, valid refs always round-trip)
- Covers: `format_hashlines` (no panics, line count, sequential numbering, hash verification)
- Covers: `apply_hashline_edits` (no panics on bad anchors, correct anchors always succeed, empty edits are no-op)

#### Task 4.3: Benchmark against the TypeScript implementation
- Ensure hash outputs are identical for the same inputs
- Performance comparison on large files

## Open Questions

1. **Hash compatibility**: Resolved. Bun uses `xxHash32(normalized, seed=0) % 256`. Our Rust implementation (`xxhash_rust::xxh32::xxh32`) with seed 0 produces identical output. Verified via 10 test vectors in `tests/integration.rs::hash_compat_bun_vectors`.

2. **`replace` operation**: Resolved. Implemented exact substring `replace` via `apply_replace_edits()`. Runs in a separate pass after anchor edits, matching the TS architecture. Errors on ambiguity (multiple matches) and not-found. Fuzzy matching (Levenshtein) is explicitly out of scope — hashline's anchor system makes it unnecessary.

3. **Streaming**: Out of scope. Removed from task list.

5. **Heuristic fidelity**: The TS implementation has ~6 different heuristic recovery mechanisms (merge detection, indent restoration, wrap restoration, etc.). These are valuable but complex. Should we port all of them in Phase 1, or start with a minimal set (hash prefix stripping, indent restoration) and add more based on real-world failure modes?

---

## Field Usage Observations (2026-02-23)

Heavy real-world use during OpenClaw containerization refactor. Multiple sessions, 7 parallel sub-agents, ~10 files edited across infrastructure and TypeScript source.

### Operation Frequency

| Operation | Count | Notes |
|-----------|-------|-------|
| `set_line` | ~25 | Most common. Reliable, no issues. |
| `replace` | ~10 | Used when anchors are awkward (multi-line blocks, blank line insertion). Escape hatch. |
| `insert_after` | ~8 | Works but has blank-line limitation (see below). |
| `replace_lines` | ~5 | Works well for range replacements. |
| `read --start-line --lines` | ~15 | Essential for large files. Saves significant context window. |
| `hash` | 0 | Never needed in practice. |

### Issues Encountered

#### 1. `insert_after` rejects empty `text` (Medium)

Cannot insert a blank line. `{"insert_after": {"anchor": "5:0e", "text": ""}}` returns an error. Workaround: use `replace` to embed `\n\n` in surrounding content. This is unintuitive and documented in CLAUDE.md as a recipe.

**Suggested fix:** Either allow empty `text` (inserting a single blank line) or add a dedicated `insert_blank_after` operation.

#### 2. Heredoc content triggers external shell guards (Medium — HIGH IMPACT)

The `hashline apply << 'EOF'` heredoc pattern means the entire JSON payload is visible to shell-level security hooks. When the payload contained dangerous-looking strings as *documentation text* (e.g. describing a shell injection vulnerability), the `dcg` pre-execution hook blocked the command entirely. This happened twice in the same session — once writing PLAN.md review findings, and again writing *this very section* to PLAN.md. Both times required falling back to the built-in Edit tool.

**This is the #1 blocker for general skill adoption.** Any project that discusses security, documents dangerous commands, or includes code examples with shell metacharacters will hit this.

**Suggested fix options:**
- A. Accept input from a file instead of stdin: `hashline apply --input edits.json`
- B. Accept base64-encoded input: `hashline apply --base64 <encoded>`
- C. Accept input from a named pipe or fd

Option A is simplest and avoids all heredoc escaping issues. The workflow becomes:
1. Claude writes the JSON to a temp file via the Write tool (which has no shell guard issues)
2. Claude runs `hashline apply --input /tmp/edits.json`
3. No heredoc, no shell guard scanning of content

#### 3. Must re-read between every apply (Low)

Forgetting to re-read after an apply causes hash mismatches. Happened twice. Recovery is smooth (stderr shows updated anchors), but it's an extra round-trip.

**Suggested fix:** On successful apply, output the updated `LINE:HASH` anchors for the changed region to stdout. This way the agent has fresh anchors without a separate read call. Could be opt-in: `hashline apply --emit-updated`.

#### 4. Permission pattern matching (Low, worked around)

Claude Code's permission allowlist matches on the first token of a Bash command. The original CLAUDE.md pattern `cat << 'EOF' | hashline apply` matched on `cat`, requiring `"Bash(cat:*)"` in settings. Fixed by switching to `hashline apply << 'EOF'` which matches `"Bash(hashline:*)"`. This is now documented.

### Sub-Agent Performance

7 parallel sub-agents used hashline simultaneously on different files. All succeeded without intervention. The anchor-based system made it easy to give precise edit instructions in agent prompts ("change line 35:b2 to..."). Hash mismatches were handled autonomously by agents.

### What Works Exceptionally Well

1. **Atomic batch edits** — all-or-nothing per file prevents partial corruption
2. **Deterministic anchors** — make edit instructions to sub-agents unambiguous
3. **`replace` as escape hatch** — handles cases where anchor ops are clumsy
4. **Partial reads** — `--start-line` + `--lines` save huge context on 500+ line files
5. **Hash mismatch recovery** — stderr output with `>>>` markers is immediately actionable
6. **Exit code convention** — 0/1/2 is clean and easy to branch on

### Skill Readiness Assessment

**Ready:**
- CLAUDE.md instructions are mature and battle-tested across multiple sessions
- All four operations documented with examples and edge cases
- Error recovery (hash mismatch, exit codes) is well documented
- Permission configuration (`"Bash(hashline:*)"` + heredoc pattern) is solved
- Sub-agents can use it without extra guidance
- Partial read support is documented and heavily used

**Not ready — blockers:**
- **Issue #2 is a hard blocker.** Any project discussing security will hit dcg false positives. Must implement `--input file` option before shipping as a skill.
- Fix issue #1 (blank line insertion) — allow empty text or add operation
- Consider issue #3 (`--emit-updated`) to reduce round-trips
- Need usage data from at least one more project to validate generality


---

## Session 2026-02-26 — Hooks, Skill, and jq Integration

### Task H1: Prune CLAUDE.md
Remove the edit-workflow instructions from CLAUDE.md that are now mechanically enforced by hooks. Keep only what hooks cannot enforce (command reference, exit codes, error recovery). Goal: shrink prompt footprint without losing essential guidance.

### Task H2: Hooks testing
Design a test strategy for the four hooks in settings.local.json:
- PreToolUse/Edit — hard block
- PreToolUse/NotebookEdit — hard block
- PreToolUse/Bash → check_before_apply.sh — blocks apply without prior read
- PostToolUse/Bash → track_hashline.sh — tracks reads/applies/staleness

Options to evaluate:
1. Manual smoke tests: intentionally trigger each hook, verify exit code and stderr message
2. Shell-level unit tests for the two scripts (feed synthetic JSON via stdin, assert exit codes)
3. BATS (Bash Automated Testing System) test suite in .claude/hooks/tests/

Bugs discovered during first use: (a) path normalization — absolute vs relative paths
produced different session-file keys; fixed by resolve_path(). (b) regex false positive —
hashline.*read matched permission strings inside Python heredocs; fixed with head -1 and
anchored ^\s*hashline\s+read.

### Task H3: Hooks template / onboarding guide
Create reusable copy-paste instructions so users can add hashline hooks to their own projects:
- HASHLINE_HOOKS.md (or section in README) explaining the hook architecture
- Template settings.json / settings.local.json snippet with the four hook registrations
- Notes on PPID-based session tracking and its limitations (PID reuse, worktrees)
- Notes on what hooks cannot enforce (batching all edits into one apply call)
- Notes on the two known edge cases and their fixes (path normalization, regex anchoring)

### Task H4: Claude Code skill evaluation
Evaluate whether hashline should be packaged as a Claude Code skill alongside hooks:
- Hooks are runtime enforcement (always-on, per-project)
- A skill is a reusable prompt expansion invoked on-demand
- Proposed complementary model: skill = one-shot bootstrapper (installs CLAUDE.md section,
  adds permissions, copies hook scripts, registers hooks in settings.local.json);
  hooks = ongoing enforcement after setup
- Open question: what is the natural trigger phrase / invocation context for the skill?

### Task H5: jq-assisted JSON editing
Investigate using jq to reduce friction when JSON files are the edit target:
- Root problem: JSON files are valid hashline targets but editing them creates triple-layer
  escaping (JSON content inside JSON new_text inside shell heredoc). The --input flag
  already solves the heredoc layer; the remaining friction is constructing the payload.
- Proposal A: document a jq-based helper for generating hashline apply payloads for JSON files
- Proposal B: a hashline apply-json sub-command that accepts jq-path edits instead of line
  anchors (e.g. {"jq_set": {"path": ".hooks.PreToolUse", "value": [...]}})
- Trade-offs: jq is a soft dependency; anchor-based ops work fine on JSON if the model
  reads first; the bigger win is tooling/templates that generate payloads programmatically

### Task H6: Track hashline usage across Claude Code projects
Instrument hashline usage tracking across all Claude Code projects to gather real-world
statistics on tool adoption, operation frequency, error rates, and failure modes.
- What to track: operation counts (read, apply, hash), operation types (set_line,
  replace_lines, insert_after, replace), error rates (hash mismatch, bad JSON, file not
  found), recovery success rate, --emit-updated vs re-read ratio, --input vs heredoc ratio
- Where to store: append-only log file per project or global (~/.claude/hashline_usage.log)
- How to collect: lightweight wrapper or hook that logs each hashline invocation
- Goal: data-driven prioritization of improvements (e.g. if replace is used 40% of the
  time, invest in making it more robust; if hash mismatches are rare, deprioritize
  heuristic recovery)
- Deferred: implementation TBD in a future session

---

### Priority Queue — 2026-02-26 Review (ordered)

1. **Task RF-1: Restore JSON formatter output (CRITICAL)** — Rewrite `format_json_with_anchors_inner` so it uses real indentation without `{ }` placeholders, renders key/value pairs correctly, and add regression tests to guard the behavior.
2. **Task RF-2: Fix canonical hash unit fixtures (CRITICAL)** — Replace the double-escaped JSON literals in `json::tests::test_canonical_hash_sorted_keys` so the test matches real input and tighten assertions around canonical hashing.
3. **Task RF-3: Restore JSON anchor compatibility (CRITICAL)** — Update `parse_anchor` to accept 1–16 hex/alphanumeric hashes per the public contract and add coverage to prevent regressions.
4. **Task RF-4: Improve `json-apply` mismatch diagnostics (CRITICAL)** — Emit both expected and actual hashes, limit stderr output to the changed anchor, and keep the rest of the file off stderr so agents can parse the response.
5. **Task RF-5: Extend hook scripts to JSON commands (CRITICAL)** — Implement Task J2-1 (regex updates + hook tests) so `json-read`/`json-apply` are enforced and logged like text edits.
6. **Task RF-6: Align CLI success messaging (HIGH)** — Make `hashline apply` and `hashline json-apply` share the same success/no-op output contract (no extra chatter by default).
7. **Task RF-7: Avoid double cloning in `apply_json_edits` (MEDIUM)** — Validate anchors once, clone once, apply to the clone, and only swap on success to maintain atomicity without the extra copy.
8. **Task RF-8: Harden filesystem tests (MEDIUM)** — Replace the fixed `/tmp/...` paths in `json.rs` unit tests with `tempfile::NamedTempFile` to avoid collisions under parallel runs.
9. **Task RF-9: Clean up post-review lint (LOW)** — Drop the unused `fmt::Write` import and run `cargo fmt && cargo clippy` to keep the tree warning-free.
10. **Task RF-10: Sync release collateral with version bumps (LOW)** — Ensure README install snippets and future release notes automatically track the crate version whenever `Cargo.toml` changes.

---

## Session 2026-02-26 — JSON-Aware Feature Fixes

Code review of the `json-aware` branch found several critical bugs and gaps in the initial implementation. Tasks below track the fixes.

### Task J-impl: Fix json.rs core engine

**Status: DONE**

Six bugs in `src/json.rs` and `src/main.rs` must be fixed before the feature ships:

1. **Fake JSONPath traversal** — `query_path`, `set_path`, `insert_at_path`, `delete_path` only handle `$` and `$.toplevelkey`. Nested paths (`$.a.b.c`) and array indices (`$.arr[0]`) silently error. `insert_at_path` ignores its `_path` argument entirely and mutates the root. Must implement a real recursive path walker supporting: `$`, `$.key`, `$.a.b.c` (arbitrary depth), `$.arr[N]` (array index), and mixed (`$.users[0].name`).

2. **Canonical hashing is not canonical** — `serde_json::to_string()` does not sort object keys. Two semantically identical objects with different insertion order hash differently. Must sort keys recursively before serializing for the hash.

3. **Hash mismatch exits 2 instead of 1** — The contract (exit 0 = success, 1 = hash mismatch, 2 = other error) is broken. Mismatch currently exits 2 with no updated anchor output, so agents can't retry. Must introduce a typed error variant that distinguishes `HashMismatch` from `OtherError`, exit 1 on mismatch, and emit updated `JSONPATH:NEW_HASH` anchors to stderr.

4. **Indentation wrong for nested objects/arrays** — `format_json_with_anchors` indents every level with a flat `  ` prefix from the recursive call's root, not relative to the parent. Nested content is misaligned.

5. **`JsonParams` struct in a match arm** — Should live in `json.rs` or a shared module, not inside a `match` arm in `main.rs`.

6. **`println!("Applied successfully.")` is inconsistent** — The existing `apply` command prints nothing on success. This stdout noise will confuse agents parsing output.

### Task J-tests: Add real tests for the JSON engine

**Status: DONE**

The current 4 tests cover parsing setup only. No tests for `apply_json_edits` or any edit operation. The fixture files in `tests/fixtures/json/` exist but are unused.
Add `tests/json_integration.rs` covering:
- `set_path` on top-level, nested, and array-indexed paths
- `insert_at_path` into object (with key) and array (without key) at correct path
- `delete_path` on top-level and nested paths
- Canonical hash consistency: same logical value, different insertion order → same hash
- Hash mismatch: stale anchor returns typed error (not panic)
- Atomicity: first edit valid, second edit stale → no mutations applied
- Round-trip: json-read anchor for a key → use that anchor in apply → verify updated value
- Use `tests/fixtures/json/small.json` and `medium.json` as input fixtures
### Task J-cli-docs: Fix CLI help indentation regression

**Status: DONE**

In `src/cli.rs`, the `after_long_help` agent workflow section lost its leading whitespace when the JSON workflow was added — the indented block became flush-left. The `hash` subcommand `long_about` gained 8 spurious leading spaces. Fixed both.

---

## Session 2026-02-26 — Second Review Findings

Thorough code review after the initial fixes found two critical bugs, several medium issues, and documentation drift. Prioritized below.

### Task J2-1: Hook scripts don't handle json-read / json-apply (CRITICAL)

**Status: TODO**
`check_before_apply.sh` line 16 only matches `hashline apply`:
```bash
grep -qE '^[[:space:]]*hashline[[:space:]]+apply'
```
It never matches `hashline json-apply`. Same for `track_hashline.sh` lines 70–71 — only looks for `hashline read` and `hashline apply`.
- `json-apply` without a prior `json-read` is never blocked
- `json-read` is never recorded in the session file
- After `json-apply`, the file is never marked stale
`HASHLINE_HOOKS.md` lines 9, 11, 12 explicitly claim these hooks handle `json-read` and `json-apply`. That's a documentation lie until the scripts are updated.

Fix: extend both regex patterns in both scripts to match `hashline json-read` / `hashline json-apply`. Update the test suite in `contrib/hooks/tests/test_hooks.sh` with synthetic JSON read/apply scenarios.

### Task J2-2: Multi-edit on overlapping paths breaks atomicity (CRITICAL)

**Status: TODO**

`apply_json_edits` validates all anchors in pass 1 against the *original* AST, then applies all edits in pass 2 sequentially. If edit 1 deletes `$.scripts` and edit 2 sets `$.scripts.test`, the delete mutates the AST, then the set fails with "Key not found" — but the delete already happened. The AST is now in a half-mutated state. Atomicity is broken.

More subtly: two `set_path` edits on the same key both validate against the original hash (both pass), then the second overwrites the first silently. The agent intended both edits to apply, but the second was never validated against the post-first-edit state.
Options:
- A. **Clone-on-validate**: clone the AST before pass 2, apply edits to the clone, swap on success. On any error in pass 2, the original AST is untouched. Simple, correct, slight memory cost.
- B. **Detect overlapping paths**: reject edit batches where any path is a prefix of another. Prevents the dangerous cases but is overly restrictive.
- C. **Sequential validate-then-apply per edit**: validate edit N against the current (possibly mutated) AST, then apply. This is correct but changes the semantics — each edit sees the result of prior edits, so anchors need to reflect post-edit state.
Recommendation: Option A — clone before apply, swap on success. It's the same pattern the text-file engine uses (validate all, then splice). The clone cost is negligible for any JSON file an agent would edit.

### Task J2-3: Formatter doesn't escape JSON keys (HIGH)

**Status: TODO**
`format_json_with_anchors_inner` line 468:
```rust
"{}  "{}": {}"
```
The key is interpolated raw. A JSON key containing `"` (e.g. `he said "hello"`) produces broken output. Fix: use `serde_json::to_string(k)` which handles escaping, instead of manual `"{}"` wrapping.

### Task J2-4: Remove dead `jsonpath-rust` dependency (MEDIUM)

**Status: TODO**

`jsonpath-rust = "0.3"` was added to `Cargo.toml` but is never imported or used anywhere. The path parser was hand-written. Remove the dependency and clean up `Cargo.lock` (should shrink by ~154 lines).

### Task J2-5: Unify error types in public API (MEDIUM)

**Status: TODO**

`parse_json_ast` returns `Result<Value, Box<dyn std::error::Error>>` while `apply_json_edits` returns `Result<(), JsonError>`. Two different error types for the same module. `parse_json_ast` should return `Result<Value, JsonError>` for a consistent API surface. This simplifies error handling in `main.rs` where both functions are called in sequence.

### Task J2-6: Fix README curl URLs after contrib/ relocation (MEDIUM)

**Status: TODO**
Two curl URLs in `README.md` still point to `.claude/`:
- Line 226: `https://raw.githubusercontent.com/.../main/.claude/skills/hashline-setup/SKILL.md`
- Line 269: `bash .claude/hooks/tests/test_hooks.sh`
Both should reference `contrib/` to match the actual file locations after the move.

Also: `HASHLINE_HOOKS.md` line 31 references `.claude/skills/hashline-setup/SKILL.md` in prose (not a curl URL) — should be `contrib/skills/hashline-setup/SKILL.md`.

### Task J2-7: Sync AGENTS.md with HASHLINE_TEMPLATE.md (LOW)

**Status: TODO**
`AGENTS.md` error recovery section (line ~121) only shows the text-file `>>>` format. `HASHLINE_TEMPLATE.md` was updated to include the JSON mismatch format. These two files have near-identical content but are now diverged. Either:
- A. Make `AGENTS.md` a copy of `HASHLINE_TEMPLATE.md` (they serve the same purpose)
- B. Add the JSON error recovery section to `AGENTS.md`
- C. Delete one and symlink / reference the other
### Task J2-8: Remove `large.json` fixture or add tests for it (LOW)

**Status: TODO**
`tests/fixtures/json/large.json` (960 lines) was added per the spec's plan for performance tests. No test uses it. Either:
- A. Delete it (dead weight)
- B. Add a benchmark or test that actually exercises it (the spec suggested parse/serialize < 100ms)
### Task J2-9: Clean up `// (fix N)` comments in json.rs (LOW)

**Status: TODO**

Lines 8, 65, 76, 217, 457 in `src/json.rs` have implementation-session comments like `// Error type (fix 3)` and `// Path segment parser (fix 1)`. These are scaffolding from the fix session, not meaningful documentation. Replace with descriptive section headers or remove entirely.

### Task J2-10: Cosmetic: `to_string_pretty` for primitives, whitespace-strip on compact input (LOW)

**Status: TODO**

1. `format_json_with_anchors_inner` line 500 uses `serde_json::to_string_pretty(value)` for primitives. `to_string` produces identical output for non-structured values and is semantically correct (the formatter controls its own layout).

2. `compute_json_anchor` routes canonical JSON through `compute_line_hash(0, &canonical)` which strips whitespace. Canonical JSON is already compact (no whitespace to strip). A direct `xxh32(canonical.as_bytes(), 0) % 256` would be clearer. However, changing the hash computation would invalidate all existing anchors — so this should only be done if no real-world anchors exist yet (i.e., before any release that ships JSON support). If anchors are already in the wild, leave it alone and document the quirk.

