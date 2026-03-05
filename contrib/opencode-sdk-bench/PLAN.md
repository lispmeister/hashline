# OpenCode Benchmark Rig - Plan (2026-03-04)
## Goal

Produce defensible evidence about Hashline utility under realistic editing conditions, across:
- correctness (did the right change happen),
- safety (did anything else get corrupted),
- protocol integrity (was hashline mode actually followed),
- efficiency (time/cost per verified success).

## Current Status

This plan is now finalized at the planning/docs level.

- Planning complete: benchmark phases, metrics, acceptance criteria, and reporting expectations are defined.
- Documentation complete: methodology, fixture strategy, and baseline caveats are written down.
- Implementation incomplete: BB-01/02/03 work is partially implemented but currently unstable in `src/cli.ts`, `src/runner.ts`, and `web/app.js`.

## Benchmark Program

### Phase 1 - Measurement Integrity (P0)
#### BB-01: Enforce mode compliance in runner
Status: `in_progress`
- For `hashline` mode, require observed `hashline read/json-read` before apply and apply via `hashline apply/json-apply`.
- Emit protocol failure reasons per attempt.
- Separate `task_pass`, `protocol_pass`, and `overall_pass`.

#### BB-02: Add compliance/safety summaries to dashboard and API
Status: `in_progress`
- Add summary fields for protocol and corruption metrics.
- Add filtering by mode/model/scenario family.

#### BB-03: Dashboard run-flow robustness
Status: `in_progress`
- Keep run controls robust when model selection or run state changes.
- Surface explicit blocked reasons when run cannot start.

### Phase 2 - Realistic Scenario Expansion (P1)

#### BB-04: Scenario family coverage
Status: `partially_done`
- Expanded default fixtures were added (stale-context, refactor/rename, large-file, confusables, JSON migration, ambiguity traps).
- Continue to 12-20 total scenarios with broader coverage and balancing.
#### BB-05: Disturbance mode
Status: `in_progress`
- Add optional disturbance between read/apply to test fail-closed behavior and recovery quality.
#### BB-06: Holdout and randomized variants
Status: `partially_done`
- Holdout fixture set exists.
- Randomization/plumbing still needs stabilization and verification.

### Phase 3 - Verification Quality (P1)
#### BB-07: Per-scenario validators
Status: `partially_done`
- Validator hooks exist and initial fixture-local validators were added.
- Expand validator coverage to all medium/hard scenarios.
#### BB-08: Silent corruption detection
Status: `in_progress`
- Corruption labeling/reporting is designed and partially wired.
- Needs full validation after runner stabilization.

### Phase 4 - Reporting Users Can Trust (P2)

#### BB-09: Methodology publication
Status: `done`
- `METHODOLOGY.md` defines metrics, interpretation bounds, and claim limits.
#### BB-10: Utility scorecard
Status: `done`
- `scripts/scorecard.mjs` exists for weighted utility summaries.
#### BB-11: Reproducible baseline snapshots
Status: `done`
- Dated baseline and publish script exist (`BASELINE_2026-03-04.md`, `scripts/publish-baseline.mjs`).

## Evidence Gates

Treat README-level product claims as blocked until all are true:
- Protocol-compliant execution is hard-enforced and visible in reports.
- Disturbance + holdout runs are included in baseline sweeps.
- Silent corruption metrics are captured and non-trivial.
- Repeated runs show stable ranges (confidence-aware reporting).

## Immediate Next Execution Order

1. Stabilize `src/cli.ts`, `src/runner.ts`, and `web/app.js`.
2. Re-enable end-to-end run smoke checks (CLI + dashboard + API + DB).
3. Run compact matrix (1 model x 2 modes x small x repeat=1) as regression check.
4. Run expanded baseline and publish scorecard + dated snapshot.

