# Benchmark Design

## Objective

Measure editing reliability and cost by running the same fixture suite against a selected model across multiple harness modes:

- `hashline` - Anchor-based editing with hash validation
- `raw_replace` - Direct string replacement
- `patch` - Patch-style editing

Only the harness mode changes; tasks and fixtures stay fixed.

## Orchestration Model

- Dashboard is the single control plane (runs on port 4177)
- User starts dashboard manually (`npm run dashboard`)
- User starts test runs from the `Run` tab only
- Server spawns benchmark CLI as child process and streams logs/stats to UI
- OpenCode SDK manages model communication and session tracking

## Minimal Test Suite (Cost-Optimized)

**6 carefully designed challenges** replace the original 27 tests (78% cost reduction):

| Test | Format | Lines | Challenge |
|------|--------|-------|-----------|
| `md-ambiguous-lines` | Markdown | 24 | Multiple "Mode:" lines - requires context awareness |
| `json-deep-nest` | JSON | 30 | Deep nesting (5 levels) with duplicate "ssl" keys |
| `rust-whitespace` | Rust | 35 | Pure indentation fix (4→8 spaces, no content change) |
| `ts-similar-names` | TypeScript | 36 | Class method vs standalone function (same name) |
| `json-array-puzzle` | JSON | 27 | Arrays with duplicate "build" names in different scopes |
| `md-json-embedded` | Markdown+JSON | 28 | Multiple JSON blocks, must fix correct timeout value |

Each fixture directory contains:
- `original.<ext>` - Correct file state
- `mutated.<ext>` - File with intentional bug
- `task.md` - Natural language fix instruction

**Design Principles**: Every test has a "gotcha" that catches imprecise targeting:
- Ambiguity (similar lines/keys)
- Deep nesting (navigation errors)
- Whitespace sensitivity
- Scope awareness (class vs function)
- Multi-format boundaries (code fences)

See `FIXTURES.md` for detailed test case descriptions and `TEST_COMPARISON.md` for old vs new comparison.

## Data Flow

1. **Execution**: CLI runner (`src/cli.ts`) orchestrates test runs via OpenCode SDK
2. **Progress**: Live updates written to `runs/progress.json` (includes cost accumulation)
3. **Persistence**: Full report saved to `runs/<timestamp>-<mode>-<model>.json`
4. **Database**: Server ingests JSON → SQLite (`runs/bench.sqlite`) with cost tracking
5. **UI**: Dashboard polls APIs for status/progress/logs and displays aggregate stats

## Dashboard Features

### Run Tab
- **Model Selection**: Searchable list with pricing (FREE vs $X/M in/out)
- **Configuration**: Select modes (hashline/raw_replace/patch), sizes, repeats
- **Live Status**: Running state, progress bar with completion percentage
- **Live Cost**: Real-time cost accumulation displayed in progress bar
- **Log Stream**: Tail of benchmark output (auto-refresh every 2s)
- **Controls**: RUN/STOP buttons, auto-refresh toggle

### Previous Runs Tab
- **Aggregate Statistics**: Pass rates grouped by mode and by model with total costs
- **Run History**: Sortable table with file, mode, model, attempts, passes, errors, retries, cost, elapsed time
- **Drill-Down**: Click any row to view per-case results in modal
- **Export**: Download runs as CSV or JSON for external analysis

### Admin Tab
- **Rebuild DB**: Drop and recreate SQLite from JSON files (recalculates all costs)
- **Drop DB**: Clear SQLite database only

## Cost Tracking

### Model Pricing
- Loaded at startup via `opencode models --verbose`
- Parses cost structure: `{ input: X, output: Y, cache: { read: Z, write: W } }`
- FREE models (input=0, output=0) highlighted in green

### Live Cost Accumulation
- Each attempt captures `response.data.info?.cost` from OpenCode SDK
- CLI accumulates total cost in `ProgressState.totalCost`
- Progress file updates every attempt with running total
- Server polls progress every 3s and logs cost in format: `cost=$X.XXXX`
- Dashboard shows live cost in progress bar label

### Persistence
- Database schema includes `total_cost REAL DEFAULT 0` column
- `summarizeReport()` aggregates cost from all attempts in JSON report
- Aggregate stats show total cost by mode and by model
- Export includes cost data in CSV/JSON format

## API Contract

- `GET /api/models`: Providers/models with pricing from `opencode models --verbose`
- `POST /api/run { model, modes[], sizes[], repeats }`: Start benchmark run
- `POST /api/run/stop`: Send SIGTERM to active child process
- `GET /api/status`: Current run state (running, completedJobs, totalJobs, lastError)
- `GET /api/progress`: Latest progress snapshot (attempts, passes, errors, **totalCost**)
- `GET /api/log?tail=N`: Log tail (default 120 lines, max 400)
- `GET /api/runs`: SQLite-backed run history with **totalCost** per run
- `GET /api/run/detail?file=<name>`: Full JSON report for drill-down modal
- `POST /api/admin/drop`: Drop SQLite database
- `POST /api/admin/rebuild`: Rebuild SQLite from JSON files (returns ingested count)

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│  Dashboard (web/index.html + web/app.js)               │
│  - Model selector with pricing                          │
│  - Live progress + cost tracking                        │
│  - Aggregate statistics                                 │
│  - Run history with drill-down                          │
└─────────────────────┬───────────────────────────────────┘
                      │ HTTP API (port 4177)
┌─────────────────────▼───────────────────────────────────┐
│  Server (server.js)                                     │
│  - Spawns CLI as child process                          │
│  - Parses opencode models --verbose for pricing         │
│  - Streams logs via appendLog()                         │
│  - Polls progress.json every 3s                         │
│  - Manages SQLite with cost columns                     │
└─────────────────────┬───────────────────────────────────┘
                      │ Child process spawn
┌─────────────────────▼───────────────────────────────────┐
│  CLI Runner (src/cli.ts)                                │
│  - Calls runBenchmark() with hooks                      │
│  - Accumulates totalCost from attempts                  │
│  - Writes progress.json with live cost                  │
│  - Outputs final report path to stdout                  │
└─────────────────────┬───────────────────────────────────┘
                      │ Calls
┌─────────────────────▼───────────────────────────────────┐
│  Benchmark Runner (src/runner.ts)                       │
│  - Lists fixtures from fixtures/ (flat structure)       │
│  - Creates work dirs for each attempt                   │
│  - Calls OpenCode SDK with mode-specific instructions   │
│  - Evaluates results (exact match)                      │
│  - Captures cost from response.data.info?.cost          │
│  - Returns RunReport with all attempt data              │
└─────────────────────┬───────────────────────────────────┘
                      │ Uses
┌─────────────────────▼───────────────────────────────────┐
│  OpenCode SDK (@opencode-ai/sdk)                        │
│  - Manages model communication                          │
│  - Creates sessions                                     │
│  - Sends prompts with mode instructions                 │
│  - Returns response with token/cost metadata            │
└─────────────────────────────────────────────────────────┘
```

## Metrics Captured

Per attempt:
- `passed`: boolean (exact file match)
- `reason`: "exact_match" | "content_differs"
- `durationMs`: execution time
- `retries`: hashline retry count (from event stream)
- `errorCount`: total errors (from event stream)
- `tokenInput/Output/Reasoning/Total`: token usage
- `cost`: USD cost for this attempt

Per run (aggregated):
- `totalAttempts`: number of test cases × repeats
- `passCount`: successful exact matches
- `errorCount`: sum of all errors
- `retryCount`: sum of all retries
- `totalCost`: sum of all attempt costs (NEW)
- `elapsedTimeMs`: total run duration

## Cost Optimization Impact

**Before**: 27 tests (9 cases × 3 sizes) @ ~$0.0047/attempt = **$0.127/run**  
**After**: 6 tests (focused challenges) @ ~$0.0047/attempt = **$0.028/run**  
**Savings**: 78% reduction (~$0.099 per run)

For 100 runs comparing 3 modes across 5 models:
- Old cost: 100 × 3 × 5 × $0.127 = **$190.50**
- New cost: 100 × 3 × 5 × $0.028 = **$42.00**
- **Total savings: $148.50** (78%)
