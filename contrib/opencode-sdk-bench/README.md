# OpenCode SDK Benchmark Rig
Dashboard-first benchmark rig for measuring Hashline impact on LLM editing tasks.

## Current State (2026-03-04)

- Planning and methodology docs are finalized.
- Fixture strategy is expanded (default + holdout sets).
- Baseline snapshot flow is documented.
- Core implementation for protocol/safety metrics is still being stabilized in `src/cli.ts`, `src/runner.ts`, and `web/app.js`.

Use this repo for benchmark planning, fixture design, and reporting artifacts first. Treat implementation status as in-progress until the stabilization pass is complete.

## Quick Start

```bash
cd contrib/opencode-sdk-bench
npm install
npm run dashboard
```

Open the URL printed by the server (default: `http://localhost:4177`).

## Run Workflow

1. Open dashboard.
2. Select model and modes.
3. Start run and monitor progress/logs.
4. Inspect summaries and previous runs.
5. Export JSON/CSV when needed.

## Benchmark Scope

- Modes: `hashline`, `raw_replace`, `patch`
- Fixture sets: `default`, `holdout`, `all`
- Optional stressors: randomized fixture order and disturbance mode
- Reporting focus: task pass, protocol pass, overall pass, corruption, retries, duration, and cost

## Reporting Commands

```bash
npm run scorecard
npm run baseline:publish
```

- `scorecard`: produces weighted utility scoring from run artifacts.
- `baseline:publish`: writes a dated markdown baseline snapshot.

## Data Storage

- Git-tracked run artifacts: `runs/*.json`
- SQLite cache: `runs/bench.sqlite*`
- Live run state: `runs/progress.json`, `runs/active.log`
- Temp workspaces: `runs/work/`

## API Endpoints
- `GET /api/models`
- `POST /api/run`
- `POST /api/run/stop`
- `GET /api/status`
- `GET /api/progress`
- `GET /api/log`
- `GET /api/runs?mode=&model=&family=`
- `POST /api/admin/drop`
- `POST /api/admin/rebuild`
## Documentation

- `PLAN.md` - execution roadmap and status by benchmark block
- `METHODOLOGY.md` - metric definitions, evidence gates, and claim limits
- `FIXTURES.md` - fixture families, sets, and validator expectations
- `BASELINE_2026-03-04.md` - initial pre-compliance baseline snapshot
- `DESIGN.md` - architecture and component boundaries

