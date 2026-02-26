1: # PLAN.md — Hashline Rust Implementation
2: 
3: ## Problem Statement
4: 
5: AI coding assistants suffer from a **harness problem**: the interface between model output and workspace edits is where most practical failures occur. Current approaches all have fundamental weaknesses:
6: 
7: - **Patch/diff format** — strict formatting rules that models frequently violate (50%+ failure rate for some models)
8: - **String replacement** — requires verbatim character-perfect reproduction of existing code, including whitespace; multiple matches cause rejection
9: - **Neural merge** — requires fine-tuning separate large models just for edit application
10: 
11: **Hashline** solves this by giving each line a short content-addressable hash tag (`LINE:HASH`). Models reference lines by hash anchors instead of reproducing content verbatim. Hash mismatches after file changes prevent silent corruption. Benchmark results show 10x improvement for weaker models and ~20% token reduction across all models.
12: 
13: ## How Hashline Works
14: 
15: ### Encoding
16: 
17: Each line gets a prefix: `LINENUM:HASH|CONTENT`
18: 
19: ```
20: 1:a3|function hello() {
21: 2:f1|  return "world";
22: 3:0e|}
23: ```
24: 
25: - Line numbers are 1-indexed
26: - Hash is 2 hex characters derived from `xxHash32(whitespace_normalized_line) % 256`
27: - The `|` separates the anchor from the content
28: 
29: ### Edit Operations
30: 
31: Four operations, all referencing anchors from the original file state:
32: 
33: 1. **`set_line`** — replace a single line: `{ anchor: "2:f1", new_text: "  return 42;" }`
34: 2. **`replace_lines`** — replace a contiguous range: `{ start_anchor: "1:a3", end_anchor: "3:0e", new_text: "..." }`
35: 3. **`insert_after`** — add lines after an anchor: `{ anchor: "3:0e", text: "// end" }`
36: 4. **`replace`** — substring fuzzy match fallback (no hashes needed)
37: 
38: ### Key Properties
39: 
40: - **Atomicity**: All edits validate against the original file state before any mutation
41: - **Bottom-up application**: Edits sorted by descending line number so splices don't invalidate indices
42: - **Staleness detection**: Hash mismatch → error with updated `LINE:HASH` refs so model can retry
43: - **Heuristic recovery**: Strips accidental hashline prefixes from model output, restores indentation, detects line merges, normalizes confusable Unicode hyphens
44: 
45: ## Source Code Location
46: 
47: The reference implementation lives in [can1357/oh-my-pi](https://github.com/can1357/oh-my-pi):
48: 
49: | File | Purpose |
50: |------|---------|
51: | `packages/coding-agent/src/patch/hashline.ts` | Core implementation (992 lines) — hashing, formatting, parsing, edit application, heuristics |
52: | `packages/coding-agent/src/prompts/tools/hashline.md` | Tool prompt given to models describing the edit format |
53: | `packages/coding-agent/test/core/hashline.test.ts` | Comprehensive test suite |
54: | `packages/react-edit-benchmark/src/verify.ts` | Benchmark verification engine |
55: | `packages/react-edit-benchmark/src/formatter.ts` | Prettier-based formatting for benchmark |
56: 
57: ## Implementation Plan
58: 
59: ### Phase 1: Core Rust Library (`hashline-core`)
60: 
61: A Rust crate implementing the hashline algorithm.
62: 
63: #### Task 1.1: Hash computation
64: - Implement `compute_line_hash(line: &str) -> String`
65: - Normalize: strip all whitespace, compute xxHash32, mod 256, format as 2-char hex
66: - Strip trailing `\r` before normalization
67: 
68: #### Task 1.2: Hashline formatting
69: - `format_hashlines(content: &str, start_line: usize) -> String`
70: - Streaming variants: out of scope
71: 
72: #### Task 1.3: Line reference parsing
73: - `parse_line_ref(ref_str: &str) -> Result<LineRef>`
74: - Handle display-format suffixes (`5:ab|content`), legacy format (`5:ab  content`), `>>>` prefixes
75: - Validation: line >= 1, hash is 1-16 hex/alphanumeric chars
76: 
77: #### Task 1.4: Edit application engine
78: - Parse edit operations (set_line, replace_lines, insert_after)
79: - Pre-validate all hashes before any mutation
80: - Hash relocation: if a ref's hash doesn't match at its line number but is unique in the file, relocate
81: - Sort edits bottom-up (descending line number)
82: - Deduplicate identical edits
83: - Apply splices
84: 
85: #### Task 1.5: Heuristic recovery layer
86: - Strip accidental hashline prefixes from model output
87: - Strip diff `+` prefixes
88: - Restore leading indentation from template lines
89: - Detect single-line merges (model merges adjacent continuation lines)
90: - Undo pure formatting rewraps (model reflows one logical line into multiple)
91: - Normalize confusable Unicode hyphens (en-dash, em-dash, etc. → ASCII hyphen)
92: - Strip boundary echo (model echoes lines above/below the edit range)
93: - Strip insert-after anchor echo
94: 
95: #### Task 1.6: Error reporting
96: - `HashlineMismatchError` with context lines (2 above/below), `>>>` markers on changed lines
97: - Show correct `LINE:HASH` so model can retry with updated refs
98: - No-op detection (replacement identical to current content)
99: 
100: ### Phase 2: CLI Binary (`hashline`)
101: 
102: A standalone binary that can be invoked by Claude Code (or any tool harness).
103: 
104: #### Task 2.1: `hashline read <file>`
105: - Read file, output hashline-formatted content to stdout
106: - Options: `--start-line N`, `--lines N` (range), `--chunk-lines N`, `--chunk-bytes N`
107: 
108: #### Task 2.5: Implement `--start-line` and `--lines` for `hashline read`
109: - The `--start-line N` and `--lines N` options from Task 2.1 were never implemented
110: - After editing a large file, agents need to verify just the changed region without re-reading the entire file
111: - Without this, agents must fall back to the built-in Read tool for partial reads, breaking the hashline workflow
112: - Accept `hashline read --start-line 130 --lines 25 <file>` to output only that range with correct LINE:HASH anchors
113: 
114: #### Task 2.2: `hashline apply <file>`
115: - Read edit operations from stdin (JSON)
116: - Apply to file, write result back
117: - Output: modified content or structured error (hash mismatches with updated refs)
118: - Exit codes: 0 = success, 1 = hash mismatch (stderr has updated refs), 2 = other error
119: 
120: #### Task 2.3: `hashline hash <file>`
121: - Output just the line hashes (for tooling/debugging)
122: 
123: #### Task 2.4: JSON schema for edits
124: - Define the edit operation JSON format matching the TypeScript types
125: - Document the schema in the binary's help output
126: 
127: #### Task 2.6: Homebrew formula
128: - Create a Homebrew formula so users can install via `brew install hashline`
129: - Options:
130:   - **Tap** (recommended for now): create a `homebrew-hashline` tap repo under the same GitHub org, add formula that downloads the release tarball and verifies SHA256
131:   - **Core**: out of scope
132: - Formula should install the binary and the man pages (`man/hashline*.1`)
133: - Update README install section with `brew install lispmeister/hashline/hashline`
134: - CI should not be blocked on this — it's a distribution convenience, not a correctness requirement
135: 
136: ### Phase 3: Integration with Claude Code
137: 
138: A plain CLI tool invoked via Bash — no MCP server, no fork, no new tool registration needed.
139: 
140: #### Task 3.1: CLAUDE.md instructions
141: Add to the project (or global `~/.claude/CLAUDE.md`) instructions that tell Claude to use hashline instead of the built-in Edit tool:
142: 
143: ```markdown
144: # Editing files
145: For all code edits, use the hashline CLI instead of the built-in Edit tool:
146: - Read: `hashline read <file>` (returns LINE:HASH|content format)
147: - Edit: `echo '{"path":"<file>","edits":[...]}' | hashline apply`
148: - After every edit, re-read before editing the same file again (hashes changed)
149: - On hash mismatch errors, use the updated LINE:HASH refs from stderr and retry
150: ```
151: 
152: #### Task 3.2: Tool prompt
153: - Adapt the existing `hashline.md` prompt (from the TS repo) into the CLAUDE.md instructions
154: - Include the edit operation formats, scope rules, and recovery procedures
155: - Keep it concise — Claude follows CLAUDE.md reliably
156: 
157: #### Task 3.3: Workflow (all via Bash)
158: 1. Claude runs `hashline read src/foo.rs` → stdout returns `LINE:HASH|content`
159: 2. Claude collects `LINE:HASH` anchors for lines it wants to change
160: 3. Claude pipes JSON edits to `hashline apply` → file is modified in place
161: 4. On hash mismatch: exit code 1, stderr has updated refs with `>>>` markers, Claude retries
162: 5. After edit: Claude re-reads before editing the same file again
163: 
164: ### Phase 4: Testing & Validation
165: 
166: #### Task 4.1: Port the TypeScript test suite to Rust
167: - All hash computation tests
168: - All formatting tests
169: - All parse/validate tests
170: - All edit application tests including heuristic edge cases
171: 
172: #### Task 4.2: Fuzz testing — DONE
173: - Property-based fuzzing via `proptest` in `tests/fuzz.rs` (12 tests, runs on stable Rust)
174: - Covers: `compute_line_hash` (no panics, 2-hex-char invariant, whitespace invariant, index ignored)
175: - Covers: `parse_line_ref` (no panics, valid refs always round-trip)
176: - Covers: `format_hashlines` (no panics, line count, sequential numbering, hash verification)
177: - Covers: `apply_hashline_edits` (no panics on bad anchors, correct anchors always succeed, empty edits are no-op)
178: 
179: #### Task 4.3: Benchmark against the TypeScript implementation
180: - Ensure hash outputs are identical for the same inputs
181: - Performance comparison on large files
182: 
183: ## Open Questions
184: 
185: 1. **Hash compatibility**: Resolved. Bun uses `xxHash32(normalized, seed=0) % 256`. Our Rust implementation (`xxhash_rust::xxh32::xxh32`) with seed 0 produces identical output. Verified via 10 test vectors in `tests/integration.rs::hash_compat_bun_vectors`.
186: 
187: 2. **`replace` operation**: Resolved. Implemented exact substring `replace` via `apply_replace_edits()`. Runs in a separate pass after anchor edits, matching the TS architecture. Errors on ambiguity (multiple matches) and not-found. Fuzzy matching (Levenshtein) is explicitly out of scope — hashline's anchor system makes it unnecessary.
188: 
189: 3. **Streaming**: Out of scope. Removed from task list.
190: 
191: 5. **Heuristic fidelity**: The TS implementation has ~6 different heuristic recovery mechanisms (merge detection, indent restoration, wrap restoration, etc.). These are valuable but complex. Should we port all of them in Phase 1, or start with a minimal set (hash prefix stripping, indent restoration) and add more based on real-world failure modes?\n192: 
193: ---\n194: 
195: ## Field Usage Observations (2026-02-23)\n196: 
197: Heavy real-world use during OpenClaw containerization refactor. Multiple sessions, 7 parallel sub-agents, ~10 files edited across infrastructure and TypeScript source.\n198: 
199: ### Operation Frequency
200: 
201: | Operation | Count | Notes |
202: |-----------|-------|-------|\n203: | `set_line` | ~25 | Most common. Reliable, no issues. |\n204: | `replace` | ~10 | Used when anchors are awkward (multi-line blocks, blank line insertion). Escape hatch. |\n205: | `insert_after` | ~8 | Works but has blank-line limitation (see below). |\n206: | `replace_lines` | ~5 | Works well for range replacements. |\n207: | `read --start-line --lines` | ~15 | Essential for large files. Saves significant context window. |\n208: | `hash` | 0 | Never needed in practice. |\n209: 
210: ### Issues Encountered
211: 
212: #### 1. `insert_after` rejects empty `text` (Medium)
213: 
214: Cannot insert a blank line. `{"insert_after": {"anchor": "5:0e", "text": ""}}` returns an error. Workaround: use `replace` to embed `\n\n` in surrounding content. This is unintuitive and documented in CLAUDE.md as a recipe.
215: 
216: **Suggested fix:** Either allow empty `text` (inserting a single blank line) or add a dedicated `insert_blank_after` operation.
217: 
218: #### 2. Heredoc content triggers external shell guards (Medium — HIGH IMPACT)
219: 
220: The `hashline apply << 'EOF'` heredoc pattern means the entire JSON payload is visible to shell-level security hooks. When the payload contained dangerous-looking strings as *documentation text* (e.g. describing a shell injection vulnerability), the `dcg` pre-execution hook blocked the command entirely. This happened twice in the same session — once writing PLAN.md review findings, and again writing *this very section* to PLAN.md. Both times required falling back to the built-in Edit tool.
221: 
222: **This is the #1 blocker for general skill adoption.** Any project that discusses security, documents dangerous commands, or includes code examples with shell metacharacters will hit this.
223: 
224: **Suggested fix options:**
225: - A. Accept input from a file instead of stdin: `hashline apply --input edits.json`
226: - B. Accept base64-encoded input: `hashline apply --base64 <encoded>`
227: - C. Accept input from a named pipe or fd
228: 
229: Option A is simplest and avoids all heredoc escaping issues. The workflow becomes:
230: 1. Claude writes the JSON to a temp file via the Write tool (which has no shell guard issues)
231: 2. Claude runs `hashline apply --input /tmp/edits.json`
232: 3. No heredoc, no shell guard scanning of content
233: 
234: #### 3. Must re-read between every apply (Low)
235: 
236: Forgetting to re-read after an apply causes hash mismatches. Happened twice. Recovery is smooth (stderr shows updated anchors), but it's an extra round-trip.
237: 
238: **Suggested fix:** On successful apply, output the updated `LINE:HASH` anchors for the changed region to stdout. This way the agent has fresh anchors without a separate read call. Could be opt-in: `hashline apply --emit-updated`.
239: 
240: #### 4. Permission pattern matching (Low, worked around)
241: 
242: Claude Code's permission allowlist matches on the first token of a Bash command. The original CLAUDE.md pattern `cat << 'EOF' | hashline apply` matched on `cat`, requiring `"Bash(cat:*)"` in settings. Fixed by switching to `hashline apply << 'EOF'` which matches `"Bash(hashline:*)"`. This is now documented.
243: 
244: ### Sub-Agent Performance
245: 
246: 7 parallel sub-agents used hashline simultaneously on different files. All succeeded without intervention. The anchor-based system made it easy to give precise edit instructions in agent prompts ("change line 35:b2 to..."). Hash mismatches were handled autonomously by agents.
247: 
248: ### What Works Exceptionally Well
249: 
250: 1. **Atomic batch edits** — all-or-nothing per file prevents partial corruption
251: 2. **Deterministic anchors** — make edit instructions to sub-agents unambiguous
252: 3. **`replace` as escape hatch** — handles cases where anchor ops are clumsy
253: 4. **Partial reads** — `--start-line` + `--lines` save huge context on 500+ line files
254: 5. **Hash mismatch recovery** — stderr output with `>>>` markers is immediately actionable
255: 6. **Exit code convention** — 0/1/2 is clean and easy to branch on
256: 
257: ### Skill Readiness Assessment
258: 
259: **Ready:**
260: - CLAUDE.md instructions are mature and battle-tested across multiple sessions
261: - All four operations documented with examples and edge cases
262: - Error recovery (hash mismatch, exit codes) is well documented
263: - Permission configuration (`"Bash(hashline:*)"` + heredoc pattern) is solved
264: - Sub-agents can use it without extra guidance
265: - Partial read support is documented and heavily used
266: 
267: **Not ready — blockers:**
268: - **Issue #2 is a hard blocker.** Any project discussing security will hit dcg false positives. Must implement `--input file` option before shipping as a skill.
269: - Fix issue #1 (blank line insertion) — allow empty text or add operation
270: - Consider issue #3 (`--emit-updated`) to reduce round-trips
271: - Need usage data from at least one more project to validate generality
272: 
273: 
274: ---
275: 
276: ## Session 2026-02-26 — Hooks, Skill, and jq Integration
277: 
278: ### Task H1: Prune CLAUDE.md
279: Remove the edit-workflow instructions from CLAUDE.md that are now mechanically enforced by hooks. Keep only what hooks cannot enforce (command reference, exit codes, error recovery). Goal: shrink prompt footprint without losing essential guidance.
280: 
281: ### Task H2: Hooks testing
282: Design a test strategy for the four hooks in settings.local.json:
283: - PreToolUse/Edit — hard block
284: - PreToolUse/NotebookEdit — hard block
285: - PreToolUse/Bash → check_before_apply.sh — blocks apply without prior read
286: - PostToolUse/Bash → track_hashline.sh — tracks reads/applies/staleness
287: 
288: Options to evaluate:
289: 1. Manual smoke tests: intentionally trigger each hook, verify exit code and stderr message
290: 2. Shell-level unit tests for the two scripts (feed synthetic JSON via stdin, assert exit codes)
291: 3. BATS (Bash Automated Testing System) test suite in .claude/hooks/tests/
292: 
293: Bugs discovered during first use: (a) path normalization — absolute vs relative paths
294: produced different session-file keys; fixed by resolve_path(). (b) regex false positive —
295:  hashline .* read  matched permission strings inside Python heredocs; fixed with head -1 and
296: anchored ^\s*hashline\s+read .
297: 
298: ### Task H3: Hooks template / onboarding guide
299: Create reusable copy-paste instructions so users can add hashline hooks to their own projects:
300: - HASHLINE_HOOKS.md (or section in README) explaining the hook architecture
301: - Template settings.json / settings.local.json snippet with the four hook registrations
302: - Notes on PPID-based session tracking and its limitations (PID reuse, worktrees)
303: - Notes on what hooks cannot enforce (batching all edits into one apply call)
304: - Notes on the two known edge cases and their fixes (path normalization, regex anchoring)
305: 
306: ### Task H4: Claude Code skill evaluation
307: Evaluate whether hashline should be packaged as a Claude Code skill alongside hooks:
308: - Hooks are runtime enforcement (always-on, per-project)
309: - A skill is a reusable prompt expansion invoked on-demand
310: - Proposed complementary model: skill = one-shot bootstrapper (installs CLAUDE.md section,
311:   adds permissions, copies hook scripts, registers hooks in settings.local.json);
312:   hooks = ongoing enforcement after setup
313: - Open question: what is the natural trigger phrase / invocation context for the skill?
314: 
315: ### Task H5: jq-assisted JSON editing
316: Investigate using jq to reduce friction when JSON files are the edit target:
317: - Root problem: JSON files are valid hashline targets but editing them creates triple-layer
318:   escaping (JSON content inside JSON new_text inside shell heredoc). The --input flag
319:   already solves the heredoc layer; the remaining friction is constructing the payload.
320: - Proposal A: document a jq-based helper for generating hashline apply payloads for JSON files
321: - Proposal B: a hashline apply-json sub-command that accepts jq-path edits instead of line
322:   anchors (e.g. {"jq_set": {"path": ".hooks.PreToolUse", "value": [...]}})
323: - Trade-offs: jq is a soft dependency; anchor-based ops work fine on JSON if the model
324:   reads first; the bigger win is tooling/templates that generate payloads programmatically
325: 
326: ### Task H6: Track hashline usage across Claude Code projects
327: Instrument hashline usage tracking across all Claude Code projects to gather real-world
328: statistics on tool adoption, operation frequency, error rates, and failure modes.
329: - What to track: operation counts (read, apply, hash), operation types (set_line,
330:   replace_lines, insert_after, replace), error rates (hash mismatch, bad JSON, file not
331:   found), recovery success rate, --emit-updated vs re-read ratio, --input vs heredoc ratio
332: - Where to store: append-only log file per project or global (~/.claude/hashline_usage.log)
333: - How to collect: lightweight wrapper or hook that logs each hashline invocation
334: - Goal: data-driven prioritization of improvements (e.g. if replace is used 40% of the
335:   time, invest in making it more robust; if hash mismatches are rare, deprioritize
336:   heuristic recovery)
337: - Deferred: implementation TBD in a future session
338: 
339: ---
340: 
341: ## Session 2026-02-26 — JSON-Aware Feature Fixes
342: 
343: Code review of the `json-aware` branch found several critical bugs and gaps in the initial implementation. Tasks below track the fixes.
344: 
345: ### Task J-impl: Fix json.rs core engine
346: 
347: **Status: DONE**
348: 
349: Six bugs in `src/json.rs` and `src/main.rs` must be fixed before the feature ships:
350: 
351: 1. **Fake JSONPath traversal** — `query_path`, `set_path`, `insert_at_path`, `delete_path` only handle `$` and `$.toplevelkey`. Nested paths (`$.a.b.c`) and array indices (`$.arr[0]`) silently error. `insert_at_path` ignores its `_path` argument entirely and mutates the root. Must implement a real recursive path walker supporting: `$`, `$.key`, `$.a.b.c` (arbitrary depth), `$.arr[N]` (array index), and mixed (`$.users[0].name`).
352: 
353: 2. **Canonical hashing is not canonical** — `serde_json::to_string()` does not sort object keys. Two semantically identical objects with different insertion order hash differently. Must sort keys recursively before serializing for the hash.
354: 
355: 3. **Hash mismatch exits 2 instead of 1** — The contract (exit 0 = success, 1 = hash mismatch, 2 = other error) is broken. Mismatch currently exits 2 with no updated anchor output, so agents can't retry. Must introduce a typed error variant that distinguishes `HashMismatch` from `OtherError`, exit 1 on mismatch, and emit updated `JSONPATH:NEW_HASH` anchors to stderr.
356: 
357: 4. **Indentation wrong for nested objects/arrays** — `format_json_with_anchors` indents every level with a flat `  ` prefix from the recursive call's root, not relative to the parent. Nested content is misaligned.
358: 
359: 5. **`JsonParams` struct in a match arm** — Should live in `json.rs` or a shared module, not inside a `match` arm in `main.rs`.
360: 
361: 6. **`println!("Applied successfully.")` is inconsistent** — The existing `apply` command prints nothing on success. This stdout noise will confuse agents parsing output.
362: 
363: ### Task J-tests: Add real tests for the JSON engine
364: 
365: **Status: DONE**
366: 
367: The current 4 tests cover parsing setup only. No tests for `apply_json_edits` or any edit operation. The fixture files in `tests/fixtures/json/` exist but are unused.
368: 
369: Add `tests/json_integration.rs` covering:
370: - `set_path` on top-level, nested, and array-indexed paths
371: - `insert_at_path` into object (with key) and array (without key) at correct path
372: - `delete_path` on top-level and nested paths
373: - Canonical hash consistency: same logical value, different insertion order → same hash
374: - Hash mismatch: stale anchor returns typed error (not panic)
375: - Atomicity: first edit valid, second edit stale → no mutations applied
376: - Round-trip: json-read anchor for a key → use that anchor in apply → verify updated value
377: - Use `tests/fixtures/json/small.json` and `medium.json` as input fixtures
378: 
379: ### Task J-cli-docs: Fix CLI help indentation regression
380: 
381: **Status: DONE**
382: 
383: In `src/cli.rs`, the `after_long_help` agent workflow section lost its leading whitespace when the JSON workflow was added — the indented block became flush-left. The `hash` subcommand `long_about` gained 8 spurious leading spaces. Fixed both.\n384: \n385: ---\n386: \n387: ## Session 2026-02-26 — Second Review Findings\n388: \n389: Thorough code review after the initial fixes found two critical bugs, several medium issues, and documentation drift. Prioritized below.

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
10. **Task RF-10: Sync release collateral with version bumps (LOW)** — Ensure README install snippets and future release notes automatically track the crate version whenever `Cargo.toml` changes.\n390: \n391: ### Task J2-1: Hook scripts don\'t handle json-read / json-apply (CRITICAL)\n392: \n393: **Status: TODO**\n394: \n395: `check_before_apply.sh` line 16 only matches `hashline apply`:\n396: ```bash\n397: grep -qE \'^[[:space:]]*hashline[[:space:]]+apply\\b\'\n398: ```\n399: It never matches `hashline json-apply`. Same for `track_hashline.sh` lines 70–71 — only looks for `hashline read` and `hashline apply`.\n400: \n401: Result:\n402: - `json-apply` without a prior `json-read` is never blocked\n403: - `json-read` is never recorded in the session file\n404: - After `json-apply`, the file is never marked stale\n405: \n406: `HASHLINE_HOOKS.md` lines 9, 11, 12 explicitly claim these hooks handle `json-read` and `json-apply`. That\'s a documentation lie until the scripts are updated.\n407: \n408: Fix: extend both regex patterns in both scripts to match `hashline json-read` / `hashline json-apply`. Update the test suite in `contrib/hooks/tests/test_hooks.sh` with synthetic JSON read/apply scenarios.\n409: \n410: ### Task J2-2: Multi-edit on overlapping paths breaks atomicity (CRITICAL)\n411: \n412: **Status: TODO**\n413: \n414: `apply_json_edits` validates all anchors in pass 1 against the *original* AST, then applies all edits in pass 2 sequentially. If edit 1 deletes `$.scripts` and edit 2 sets `$.scripts.test`, the delete mutates the AST, then the set fails with \"Key not found\" — but the delete already happened. The AST is now in a half-mutated state. Atomicity is broken.\n415: \n416: More subtly: two `set_path` edits on the same key both validate against the original hash (both pass), then the second overwrites the first silently. The agent intended both edits to apply, but the second was never validated against the post-first-edit state.\n417: \n418: Options:\n419: - A. **Clone-on-validate**: clone the AST before pass 2, apply edits to the clone, swap on success. On any error in pass 2, the original AST is untouched. Simple, correct, slight memory cost.\n420: - B. **Detect overlapping paths**: reject edit batches where any path is a prefix of another. Prevents the dangerous cases but is overly restrictive.\n421: - C. **Sequential validate-then-apply per edit**: validate edit N against the current (possibly mutated) AST, then apply. This is correct but changes the semantics — each edit sees the result of prior edits, so anchors need to reflect post-edit state.\n422: \n423: Recommendation: Option A — clone before apply, swap on success. It\'s the same pattern the text-file engine uses (validate all, then splice). The clone cost is negligible for any JSON file an agent would edit.\n424: \n425: ### Task J2-3: Formatter doesn\'t escape JSON keys (HIGH)\n426: \n427: **Status: TODO**\n428: \n429: `format_json_with_anchors_inner` line 468:\n430: ```rust\n431: \"{}  \\\"{}\\\": {}\"\n432: ```\n433: The key is interpolated raw. A JSON key containing `\"` (e.g. `he said \"hello\"`) produces broken output. Fix: use `serde_json::to_string(k)` which handles escaping, instead of manual `\\\"{}\\\"` wrapping.\n434: \n435: ### Task J2-4: Remove dead `jsonpath-rust` dependency (MEDIUM)\n436: \n437: **Status: TODO**\n438: \n439: `jsonpath-rust = \"0.3\"` was added to `Cargo.toml` but is never imported or used anywhere. The path parser was hand-written. Remove the dependency and clean up `Cargo.lock` (should shrink by ~154 lines).\n440: \n441: ### Task J2-5: Unify error types in public API (MEDIUM)\n442: \n443: **Status: TODO**\n444: \n445: `parse_json_ast` returns `Result<Value, Box<dyn std::error::Error>>` while `apply_json_edits` returns `Result<(), JsonError>`. Two different error types for the same module. `parse_json_ast` should return `Result<Value, JsonError>` for a consistent API surface. This simplifies error handling in `main.rs` where both functions are called in sequence.\n446: \n447: ### Task J2-6: Fix README curl URLs after contrib/ relocation (MEDIUM)\n448: \n449: **Status: TODO**\n450: \n451: Two curl URLs in `README.md` still point to `.claude/`:\n452: - Line 226: `https://raw.githubusercontent.com/.../main/.claude/skills/hashline-setup/SKILL.md`\n453: - Line 269: `bash .claude/hooks/tests/test_hooks.sh`\n454: \n455: Both should reference `contrib/` to match the actual file locations after the move.\n456: \n457: Also: `HASHLINE_HOOKS.md` line 31 references `.claude/skills/hashline-setup/SKILL.md` in prose (not a curl URL) — should be `contrib/skills/hashline-setup/SKILL.md`.\n458: \n459: ### Task J2-7: Sync AGENTS.md with HASHLINE_TEMPLATE.md (LOW)\n460: \n461: **Status: TODO**\n462: \n463: `AGENTS.md` error recovery section (line ~121) only shows the text-file `>>>` format. `HASHLINE_TEMPLATE.md` was updated to include the JSON mismatch format. These two files have near-identical content but are now diverged. Either:\n464: - A. Make `AGENTS.md` a copy of `HASHLINE_TEMPLATE.md` (they serve the same purpose)\n465: - B. Add the JSON error recovery section to `AGENTS.md`\n466: - C. Delete one and symlink / reference the other\n467: \n468: ### Task J2-8: Remove `large.json` fixture or add tests for it (LOW)\n469: \n470: **Status: TODO**\n471: \n472: `tests/fixtures/json/large.json` (960 lines) was added per the spec\'s plan for performance tests. No test uses it. Either:\n473: - A. Delete it (dead weight)\n474: - B. Add a benchmark or test that actually exercises it (the spec suggested parse/serialize < 100ms)\n475: \n476: ### Task J2-9: Clean up `// (fix N)` comments in json.rs (LOW)\n477: \n478: **Status: TODO**\n479: \n480: Lines 8, 65, 76, 217, 457 in `src/json.rs` have implementation-session comments like `// Error type (fix 3)` and `// Path segment parser (fix 1)`. These are scaffolding from the fix session, not meaningful documentation. Replace with descriptive section headers or remove entirely.\n481: \n482: ### Task J2-10: Cosmetic: `to_string_pretty` for primitives, whitespace-strip on compact input (LOW)\n483: \n484: **Status: TODO**\n485: \n486: 1. `format_json_with_anchors_inner` line 500 uses `serde_json::to_string_pretty(value)` for primitives. `to_string` produces identical output for non-structured values and is semantically correct (the formatter controls its own layout).\n487: \n488: 2. `compute_json_anchor` routes canonical JSON through `compute_line_hash(0, &canonical)` which strips whitespace. Canonical JSON is already compact (no whitespace to strip). A direct `xxh32(canonical.as_bytes(), 0) % 256` would be clearer. However, changing the hash computation would invalidate all existing anchors — so this should only be done if no real-world anchors exist yet (i.e., before any release that ships JSON support). If anchors are already in the wild, leave it alone and document the quirk.\n\n\n---\n\n## Session 2026-02-26 — Third Review Findings\n\n### Task J3-1: Hand-rolled JSONPath parsing is brittle and undocumented (HIGH)\n\n**Status: TODO**\n\n**Reasoning:** The `parse_path_segments` function (json.rs:99-155) is a manual state machine for parsing JSONPath. While currently functional, it's brittle against variations in JSONPath syntax, potential future escape sequences, or malformed input not covered by existing tests. Reimplementing this complex logic is error-prone and a maintenance burden compared to using a well-tested library.\n\n**Proposed Fix:** Replace the custom `parse_path_segments` with a mature, battle-tested JSONPath parsing library. If a suitable one is unavailable or deemed too heavy, thoroughly document the *exact* supported JSONPath subset and edge cases, and add extensive fuzzing specifically for path parsing.\n\n### Task J3-2: `canonical_json` performance for large/deep JSON (MEDIUM)\n\n**Status: TODO**\n\n**Reasoning:** The recursive nature of `canonical_json` (json.rs:235-258) with its frequent string allocations (`Vec<String>`, `format!`) can lead to significant performance degradation and high memory usage for very large or deeply nested JSON structures. While correctness is achieved, the current implementation might not meet implicit performance requirements for large files.\n\n**Proposed Fix:** Investigate alternative canonicalization strategies that minimize intermediate string allocations, potentially operating more directly on `serde_json::Value` or by using a custom serializer that sorts keys during a single pass. Benchmark against large JSON fixtures (once they are used).\n\n### Task J3-3: `format_json_with_anchors_inner` string building performance (MEDIUM)\n\n**Status: TODO**\n\n**Reasoning:** Similar to `canonical_json`, the `format_json_with_anchors_inner` function (json.rs:458-502) extensively uses `push_str` and `format!`, leading to numerous intermediate string allocations and reallocations of the `result` string. This will negatively impact performance and memory usage when formatting large JSON files for `json-read` output.\n\n**Proposed Fix:** Refactor string building to use `std::fmt::Write` trait with a `String` buffer or pre-allocate `result` string with an estimated capacity to reduce reallocations. Consider a custom `serde_json::Serializer` that can inject comments directly during serialization, eliminating the need for manual recursive formatting.\n\n### Task J3-4: `InsertAtPathOp` API ambiguity (LOW)\n\n**Status: TODO**\n\n**Reasoning:** The `InsertAtPathOp` struct (json.rs:288-293) contains both `key: Option<String>` and `index: Option<usize>`. The documentation notes that `index` is "Ignored when `key` is set." This creates an ambiguous API where callers must understand precedence rules, leading to potential confusion or misuse. It forces the `insert_at_path` function to contain branching logic for mutually exclusive options.\n\n**Proposed Fix:** Refactor `JsonEdit` to have distinct operations for inserting into objects (e.g., `InsertObjectEntry { path, key, value }`) and inserting into arrays (e.g., `InsertArrayElement { path, index, value }`). This would make the API explicit and remove the ambiguity.\n\n### Task J3-5: Repetitive file reading/newline normalization (MEDIUM)\n\n**Status: TODO**\n\n**Reasoning:** The pattern of reading a file into a string, replacing `\r\n` with `\n`, and stripping a trailing `\n` is repeated almost identically in `read`, `apply`, `hash`, `json-read`, and `json-apply` commands within `src/main.rs`. This duplication is a maintenance burden; a bug fix or enhancement to file reading logic would need to be applied in five separate places.\n\n**Proposed Fix:** Extract this common file reading and normalization logic into a single, reusable helper function (e.g., `read_file_content_and_normalize(path: &Path) -> Result<String, io::Error>`) within `src/lib.rs` or `src/util.rs` (if created) and use it across all commands.\n\n### Task J3-6: `after_long_help` is a hardcoded, brittle markdown string (MEDIUM)\n\n**Status: TODO**\n\n**Reasoning:** The extensive markdown content in `Cli.after_long_help` (cli.rs:15-46) is embedded directly as a raw string literal. This is difficult to read, painful to maintain (especially with `\` escapes), and susceptible to formatting regressions with `clap` updates. It violates the principle of separation of concerns by mixing presentation logic with code structure.\n\n**Proposed Fix:** Extract the help text into an external Markdown file (e.g., `cli_help.md`) and load it at compile time using `include_str!`. This significantly improves readability and maintainability of the help text. Ensure any dynamic elements (like version numbers) are correctly injected.\n\n### Task J3-7: CLI `usize` ranges for `start_line` and `lines` (LOW)\n\n**Status: TODO**\n\n**Reasoning:** The `RangedU64ValueParser::<usize>::new().range(1..=(u32::MAX as u64))` for `start_line` and `lines` (cli.rs:75, 78) arbitrarily caps the maximum value at `u32::MAX` (2^32 - 1). While this is a large number, `usize` can represent much larger values on 64-bit systems. There's no clear justification for this specific limit, and it could be misleading or unnecessarily restrictive for extremely large files.\n\n**Proposed Fix:** Re-evaluate the maximum logical line number or length of files `hashline` is expected to handle. If `u32::MAX` is sufficient, add a comment explaining why. Otherwise, use `usize::MAX` or a more appropriate, justified upper bound for the range. Alternatively, remove the explicit `u32::MAX` cast if the `usize` type already implicitly handles the platform's maximum.\n\n### Task J3-8: `large.json` fixture is unused (LOW)\n\n**Status: TODO**\n\n**Reasoning:** The `tests/fixtures/json/large.json` (960 lines) was added with the intention of being used for performance benchmarks or tests, as stated in the spec. However, no existing test in the suite actually loads or processes this file.\n\n**Proposed Fix:** Either: A) Implement a performance benchmark (e.g., in `tests/benchmark.rs` or `benches/large_json_bench.rs`) that utilizes `large.json` to verify the `json-read`/`json-apply` commands meet performance criteria (e.g., parse/serialize < 100ms). Or, B) If performance testing with this fixture is deferred indefinitely, remove the `large.json` file as it's dead weight in the codebase.\n\n### Task J3-9: Cosmetic `// (fix N)` comments in `json.rs` (LOW)\n\n**Status: TODO**\n\n**Reasoning:** Several comments in `src/json.rs` (e.g., lines 8, 65, 76, 217, 457) like `// Error type (fix 3)` are artifacts from the initial implementation session. While helpful during development, they are not part of the long-term code documentation and clutter the file with transient information.\n\n**Proposed Fix:** Replace these development-specific comments with more descriptive, long-term section headers or remove them entirely to improve code readability and maintainability.\n\n### Task J3-10: `compute_json_anchor` whitespace stripping in `compute_line_hash` (LOW)\n\n**Status: TODO**\n\n**Reasoning:** `compute_json_anchor` (json.rs:259-263) calls `canonical_json` to get a compact JSON string, then passes this string to `compute_line_hash(0, &canonical)`. The `compute_line_hash` function (hash.rs:19) explicitly strips *all* whitespace. Canonical JSON, by definition, is already compact and contains no meaningful whitespace. Therefore, the whitespace stripping in `compute_line_hash` is redundant for canonical JSON, making the hash calculation path slightly less clear and potentially adding a tiny, unnecessary processing step.\n\n**Proposed Fix:** If no real-world anchors for JSON files have been deployed yet (i.e., before the first release of JSON-aware features), modify `compute_json_anchor` to directly call `xxhash_rust::xxh32::xxh32(canonical.as_bytes(), 0) % HASH_MOD` (as defined in hash.rs) instead of routing through `compute_line_hash`. This would make the intent clearer and avoid redundant processing. If anchors *are* already in the wild, this change would break compatibility, so the current approach should be retained, and a comment should be added to `compute_json_anchor` explaining that `compute_line_hash`'s whitespace stripping is technically redundant but necessary for hash compatibility.
