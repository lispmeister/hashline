# PLAN.md — Hashline Product + Implementation Plan

## Mission

Hashline makes AI code editing reliable by replacing fragile text matching with anchor-based edits (`LINE:HASH` and `JSONPATH:HASH`) plus safety checks that fail closed on stale context.

## Current State (2026-03-04)

### What is implemented and working

- Core CLI commands: `read`, `apply`, `hash`, `json-read`, `json-apply`, `hook pre`, `hook post`
- Atomic text editing with anchor validation: `set_line`, `replace_lines`, `insert_after`, `replace`
- Atomic JSON-aware editing: `set_path`, `insert_at_path`, `delete_path`
- Hash mismatch diagnostics with updated anchors and non-zero exit codes
- Heuristic recovery for common model output artifacts in text edits
- Built-in Claude Code hook subcommands (no external hook scripts required)
- Hook integration tests in `contrib/hooks/tests/test_hooks.sh`
- Install paths: Homebrew, release binary install script, source install

### Recently delivered milestones

- v0.1.14: moved hook behavior into native `hashline hook pre/post`
- v0.1.13: improved agent integration docs and setup flow
- v0.1.12: shipped JSON-aware workflows and broader docs/test coverage

## Current Reality Check (Galaxy Brain Review)

The core engine is strong, but setup and agent integration still feel more manual than they should. The next phase should prioritize turnkey onboarding, parser robustness, and documentation consistency across Claude and non-Claude agents.

## Priorities

### P0 — Trust + Correctness + Setup Reliability

#### GB-01: Fix `hashline hash` duplicate output bug
- **Problem:** `hashline hash` currently prints each line twice.
- **Tasks:**
  1. Remove duplicate output loop in `src/main.rs`.
  2. Add integration coverage that asserts one output line per input line.
  3. Add regression note to changelog.
- **Acceptance criteria:** single-pass output only; regression test fails before fix and passes after.

#### GB-02: Harden hook command parsing (`--input`, `-i`, quoting, spaces)
- **Problem:** hook extraction is regex/split based and fragile for real shell commands.
- **Tasks:**
  1. Support both `--input` and `-i` in `extract_input_flag`.
  2. Handle quoted file paths and spaces robustly.
  3. Support command prefixes like env assignments before `hashline`.
  4. Add parser-focused unit tests for pre/post hook parsing edge cases.
- **Acceptance criteria:** hook enforcement works for quoted paths, spaced paths, and `-i` form.

#### GB-03: Add strict enforcement mode for unresolved apply target
- **Problem:** when path extraction fails, pre-hook currently allows command through.
- **Tasks:**
  1. Add strict mode env flag (e.g., `HASHLINE_HOOK_STRICT=1`).
  2. In strict mode, block apply/json-apply if target path cannot be determined.
  3. Keep permissive mode default for backward compatibility.
  4. Add tests for strict vs permissive behavior.
- **Acceptance criteria:** deterministic fail-closed behavior available for high-safety users.

#### GB-04: Improve hook error guidance by command type
- **Problem:** block messages recommend `hashline read` even for JSON apply flows.
- **Tasks:**
  1. Detect `apply` vs `json-apply` and tailor remediation message.
  2. Include concrete next command in stderr output.
  3. Add message assertions in hook tests.
- **Acceptance criteria:** blocked JSON apply recommends `hashline json-read`.

#### GB-05: Fix PLAN/docs drift and obvious template defects
- **Problem:** docs have inconsistencies and stale references.
- **Tasks:**
  1. Remove stray empty fenced block in `HASHLINE_TEMPLATE.md`.
  2. Ensure template rule text includes both `hashline read` and `hashline json-read`.
  3. Align README hook test path references with repo reality.
  4. Bring changelog current with released versions.
  5. Correct JSON spec limitation text to match bracket-notation support.
- **Acceptance criteria:** no conflicting setup instructions across README/template/spec/changelog.

### P1 — One-Command Onboarding (Claude-first, extensible)

#### GB-06: Add `hashline setup claude`
- **Goal:** replace manual multi-step setup with one deterministic command.
- **Tasks:**
  1. Add CLI subcommand to detect/create `.claude/settings.local.json`.
  2. Merge permissions and hooks idempotently.
  3. Inject hashline template content at top of `CLAUDE.md` idempotently.
  4. Optionally run hook self-test and report pass/fail.
  5. Print concise next steps and restart hint.
- **Acceptance criteria:** fresh repo can be made hook-enforced in one command.

#### GB-07: Add `hashline doctor`
- **Goal:** make install and enforcement state observable.
- **Checks:**
  1. Binary/version sanity.
  2. Hook registration presence.
  3. `Bash(hashline:*)` permission presence.
  4. Optional simulated enforcement check (apply before read should block).
  5. Template presence in agent instructions.
- **Acceptance criteria:** clear pass/fail report with actionable remediation.

#### GB-08: Version-pinned setup assets
- **Problem:** setup content fetched from `main` can drift from installed binary.
- **Tasks:**
  1. Prefer fetching docs/templates/tests pinned to installed version tag.
  2. Keep `main` fallback only when versioned artifact unavailable.
  3. Print provenance in setup output.
- **Acceptance criteria:** reproducible setup aligned with installed binary behavior.

### P2 — Multi-Agent Expansion (Beyond Claude)

#### GB-09: Agent adapter architecture
- **Goal:** support enforcement/installation per agent capabilities.
- **Tasks:**
  1. Define adapter interface (Claude, Cursor, Windsurf, Generic).
  2. Implement `hashline setup --agent <name>` scaffold.
  3. For each adapter, document enforceable vs advisory guarantees.
- **Acceptance criteria:** users can run setup with explicit agent target and predictable outcomes.

#### GB-10: Compatibility matrix + agent-specific templates
- **Tasks:**
  1. Publish capability matrix in README (edit-blocking, read-before-apply enforcement, post-tool state tracking).
  2. Add agent-specific instruction templates where needed.
  3. Add token-efficient/strict variants where appropriate.
- **Acceptance criteria:** non-Claude users get first-class guidance, not generic caveats.

## Execution Plan

### Phase 1 (immediate)
- Deliver GB-01 through GB-05 in a stabilization release.
- Update tests + docs in same release to eliminate user-facing drift.

### Phase 2
- Deliver GB-06 and GB-07 as the onboarding release.
- Position `hashline setup claude` + `hashline doctor` as default getting-started flow.

### Phase 3
- Deliver GB-08 through GB-10 for multi-agent adoption.
- Expand ecosystem docs and integration examples.

## Risks and Mitigations

- Parser hardening may introduce false negatives in unusual shell invocations.
  - **Mitigation:** comprehensive fixture tests for command-shape variants.
- One-command setup may overstep user preferences in existing settings files.
  - **Mitigation:** idempotent merge, explicit dry-run/preview mode, backup on write.
- Cross-agent support may promise more than some agents can enforce.
  - **Mitigation:** capability matrix with explicit “enforced” vs “advisory” labeling.

## Notes

- After changes under `src/`, reinstall before using `hashline` in the same environment:
  - `cargo install --path .`
- Prefer `--input` + `--emit-updated` flows in examples and templates to reduce round-trips and heredoc fragility.

