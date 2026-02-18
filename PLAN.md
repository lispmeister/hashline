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
- Streaming variants for large files (chunk by line count and byte size)

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
  - **Core** (longer term): submit to `homebrew-core` once the project has enough stars/usage
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

3. **Streaming**: Punted. Current implementation is fully in-memory. Not a priority — files that models edit are rarely large enough to matter, and stdout is already line-buffered. If needed later, add `--chunk-lines N` / `--chunk-bytes N` to `hashline read` and a streaming `format_hashlines` variant that writes to `impl Write` instead of returning a `String`.

5. **Heuristic fidelity**: The TS implementation has ~6 different heuristic recovery mechanisms (merge detection, indent restoration, wrap restoration, etc.). These are valuable but complex. Should we port all of them in Phase 1, or start with a minimal set (hash prefix stripping, indent restoration) and add more based on real-world failure modes?
