# Benchmarks

Performance results are recorded per release and stored in `results/`. `baseline.json` is always a copy of the most recent release result and is what CI compares against.

## Running locally

```sh
# Human-readable Markdown tables (default)
cargo run --release --bin bench

# Structured JSON (for comparison)
cargo run --release --bin bench -- --json > /tmp/current.json

# Compare against baseline
python benchmarks/compare.py benchmarks/baseline.json /tmp/current.json
python benchmarks/compare.py benchmarks/baseline.json /tmp/current.json --threshold 10
```

## How CI uses this

On every push to `main` and every PR, the `bench` job in `ci.yml`:
1. Builds the bench binary in release mode
2. Runs `bench --json` → `/tmp/current.json`
3. Runs `compare.py` against `benchmarks/baseline.json`
4. Fails if any metric regressed by more than 15%

On every release tag (`v*`), the `bench-release.yml` workflow:
1. Runs benchmarks on `ubuntu-latest`
2. Writes `benchmarks/results/<tag>.json`
3. Copies it to `benchmarks/baseline.json`
4. Commits both files back to `main`

This keeps the baseline in sync with each release and makes history browsable in the repo.

## Bootstrapping a new baseline

Trigger the `Benchmark Release` workflow manually via GitHub Actions → workflow_dispatch, passing the tag name (e.g. `v0.1.3`). It will run on `ubuntu-latest` and commit the result.

> **Note:** The baseline in this repo was seeded on Apple Silicon (local), not `ubuntu-latest`. The first `workflow_dispatch` run on `v0.1.3` will replace it with a proper linux/x86_64 baseline. Until then, the CI regression check is advisory only.

## Schema

Each result file contains:

```json
{
  "version": "0.1.3",
  "commit": "c073ba4",
  "timestamp": "2026-02-18T07:05:53Z",
  "runner": "ubuntu-latest",
  "results": [
    {
      "benchmark": "format_hashlines",
      "file_lines": 1000,
      "metric": "us_per_iter",
      "value": 334.6
    },
    {
      "benchmark": "apply_batched",
      "file_lines": 1000,
      "edit_count": 100,
      "metric": "us_per_iter",
      "value": 566.7
    }
  ]
}
```

`metric` is always `us_per_iter` (microseconds per iteration). `edit_count` is omitted for benchmarks where it does not apply.
