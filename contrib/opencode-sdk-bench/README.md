# OpenCode SDK Benchmark Rig

Dashboard-first benchmark rig for measuring Hashline impact on LLM editing tasks.

## Quick Start

**IMPORTANT**: If you're in an OpenCode session, exit it first (`exit` or Ctrl+D) to avoid nested permission conflicts.
```bash
cd contrib/opencode-sdk-bench
npm install
npm run dashboard
```

Open the URL printed by the server (default: `http://localhost:4177`).

## Workflow
1. Open the dashboard in your browser
2. Select a model from the dropdown (109 models available with pricing info)
3. Choose modes to test: `hashline`, `raw_replace`, and/or `patch`
4. Click `RUN` to start the benchmark
5. Watch live progress, logs, and cost accumulation
6. View results in `Previous Runs` tab with aggregate statistics


## Features

- **Live monitoring**: Real-time progress bar, log streaming (2s refresh), and cost tracking
- **6 challenging test cases**: Ambiguous lines, deep nesting, whitespace sensitivity, name collisions
- **Cost tracking**: Per-run costs aggregated from OpenCode SDK responses
- **Aggregate stats**: Pass rates by mode and model across all historical runs
- **Export**: Download results as CSV or JSON
- **Database admin**: Rebuild or drop SQLite database from raw JSON files

## Clear Test Data

To reset all test results:

```bash
./clear-data.sh
```

This removes the SQLite database and all JSON result files.
## Data Storage

- **Git-tracked**: `runs/*.json` - Full test results with all attempt details
- **Git-ignored**: `runs/bench.sqlite*` - SQLite database for fast queries
- **Live files**: `runs/progress.json`, `runs/active.log` - Current run state
- **Working directory**: `runs/work/` - Temporary test files during execution

## Tabs

- `Run`: start tests, monitor in-flight stats/log output, stop active run
- `Previous Runs`: run summaries ordered newest-first from sqlite
- `Admin`: drop sqlite DB or rebuild DB from raw JSON files

## API Endpoints

- `GET /api/models`
- `POST /api/run`
- `POST /api/run/stop`
- `GET /api/status`
- `GET /api/progress`
- `GET /api/log`
- `GET /api/runs`
- `POST /api/admin/drop`
- `POST /api/admin/rebuild`

## Troubleshooting

**Dashboard not updating after code changes?**
- Hard refresh: `Cmd+Shift+R` (Mac) or `Ctrl+Shift+F5` (Windows/Linux)

**Permission errors when running?**
- Exit any OpenCode sessions before starting the dashboard
- The benchmark needs direct file system access

**Models not showing in dropdown?**
- Ensure OpenCode CLI is installed: `npm install -g opencode-cli`
- Check `opencode models --verbose` returns valid JSON

## Documentation

- [START.md](START.md) - Detailed startup guide
- [FIXTURES.md](FIXTURES.md) - Test case descriptions
- [DESIGN.md](DESIGN.md) - System architecture
- [TROUBLESHOOTING.md](TROUBLESHOOTING.md) - Common issues and solutions
