# Dashboard Status (2026-03-04)

## Status

Dashboard planning and API contract docs are up to date, but the current frontend/runtime implementation is not yet stable after recent benchmark-metric expansion.

## Known Unstable Areas

- `contrib/opencode-sdk-bench/web/app.js`
- `contrib/opencode-sdk-bench/src/cli.ts`
- `contrib/opencode-sdk-bench/src/runner.ts`

## What Is Already Defined

- Run configuration should support fixture-set, randomization, disturbance, and protocol-enforcement flags.
- Run summaries should include task/protocol/overall pass and corruption counters.
- Run listing should support mode/model/family filters.

## Validation Needed Before Calling Dashboard Healthy

1. `npm run build` passes in `contrib/opencode-sdk-bench`.
2. `node --check web/app.js` passes.
3. `/api/run` accepts new run options and starts successfully.
4. `/api/runs` filter parameters return expected subsets.
5. UI run flow works end to end (start, progress, completion, history updates).

