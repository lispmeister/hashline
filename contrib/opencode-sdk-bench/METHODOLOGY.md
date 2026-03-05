# Benchmark Methodology
This benchmark evaluates editing utility across four dimensions:
1. Correctness
2. Safety
3. Efficiency
4. Review burden
## Core Attempt Metrics

- `task_passed`: scenario validator/expected-content check passed.
- `protocol_passed`: mode-specific protocol check passed (especially hashline compliance).
- `overall_passed`: both task and protocol passed (or task-only if protocol enforcement is disabled).
- `corruption_detected`: unexpected modifications outside allowed mutation scope.
- `retry_count` and error classes: used to compute recovery behavior.
- `duration_ms` and `attempt_cost`: used for efficiency comparisons.

## Aggregate Reporting Metrics

- `verified_success_rate`: task pass rate.
- `protocol_pass_rate`: protocol pass rate.
- `both_pass_rate`: overall pass rate.
- `silent_corruption_rate`: corruption frequency.
- `recovery_rate`: eventual success after retries/errors.
- `time_to_verified_success_ms`.
- `cost_per_verified_success`.

## Evidence Gates (Claim Discipline)

Do not publish strong product-level claims unless runs include:
- protocol-enforced execution,
- disturbance-enabled stress runs,
- holdout fixture coverage,
- repeated runs with stable ranges/confidence awareness.

## Claim Boundaries

- Small-fixture happy-path results are useful for regression checks, not broad utility claims.
- Raw pass rate alone is insufficient when protocol/safety metrics fail.
- Throughput claims must be separated from safety-under-stress claims.

## Reporting Workflow

```bash
npm run scorecard
npm run baseline:publish
```

- `scorecard` generates weighted utility summaries.
- `baseline:publish` creates dated markdown snapshots linked to run artifacts.

