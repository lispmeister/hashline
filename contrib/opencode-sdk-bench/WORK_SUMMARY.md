# Benchmark Work Summary (2026-03-04)

This file replaces the earlier implementation-complete narrative.

## What Is Finished

- Planning roadmap is finalized in `PLAN.md`.
- Methodology and claim boundaries are defined in `METHODOLOGY.md`.
- Fixture strategy (default + holdout) is documented in `FIXTURES.md`.
- Baseline snapshot process and first dated baseline are documented.
- Scorecard and baseline publish scripts are documented in README and methodology docs.

## What Is Partially Implemented

- Protocol compliance instrumentation in CLI/runner.
- Disturbance/randomization plumbing.
- Dashboard/API surfacing for protocol/corruption metrics.

## Current Blockers

The current branch has unstable edits that must be stabilized before new benchmark claims:
- `contrib/opencode-sdk-bench/src/cli.ts`
- `contrib/opencode-sdk-bench/src/runner.ts`
- `contrib/opencode-sdk-bench/web/app.js`

## Why This Matters

Until stabilization is complete, benchmark outputs are useful for historical context only, not for new product-level claims.

## Next Execution Order

1. Stabilize build/runtime for CLI runner and dashboard.
2. Verify end-to-end run flow with a compact smoke matrix.
3. Re-run baseline with protocol/safety metrics fully active.
4. Publish updated scorecard and dated baseline snapshot.

