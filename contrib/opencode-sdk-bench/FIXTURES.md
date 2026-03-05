# Benchmark Fixture Suite
The fixture corpus is split into two sets:
- `fixtures/` (default): public regression fixtures for routine runs.
- `fixtures-holdout/`: holdout fixtures for anti-overfitting checks.

## Status

- Core family coverage is defined and documented.
- New default fixtures and holdout fixtures were added.
- Additional medium/hard scenarios can still be added, but planning for family coverage is complete.

## Scenario Families

Current families include:
1. Ambiguity and nearby-target confusion
2. Nested JSON migrations
3. Whitespace-sensitive edits
4. Scope collisions and similar identifiers
5. Multi-format docs (Markdown with embedded JSON)
6. Stale-context and disturbance-sensitive edits
7. Large-file targeted edits
8. Confusable-identifier traps
9. Simple refactor/rename style edits

## Inventory

Default set:
- `md-ambiguous-lines`
- `json-deep-nest`
- `rust-whitespace`
- `ts-similar-names`
- `json-array-puzzle`
- `md-json-embedded`
- `stale-context-trap`
- `refactor-rename-endpoint`
- `large-file-settings`
- `confusable-identifiers`
- `json-migration-rules`
- `ambiguous-nearby-blocks`
Holdout set:
- `holdout-json-target`
- `holdout-ts-scope`
## Validator Expectations

Prefer fixture-local `validator.mjs` checks for medium/hard scenarios so we can detect:
- right text in wrong place,
- wrong symbol edited when names are similar,
- collateral modifications outside expected mutation window.

## Run-Time Options

- `--fixture-set default` for routine regressions
- `--fixture-set holdout` for anti-overfitting checks
- `--fixture-set all` for broad sweeps
- `--randomize --seed <n>` for deterministic shuffle order
- `--disturbance --disturbance-probability <0..1>` for stale-context stress testing

