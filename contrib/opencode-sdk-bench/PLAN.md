# OpenCode Benchmark Dashboard - Design Review & Plan

## Overview

Single-page dashboard for running LLM editing benchmarks comparing hashline vs raw_replace vs patch modes. Built with vanilla JS frontend + Node.js backend with SQLite persistence.

## Architecture

- **Backend**: `server.js` - HTTP server on port 4177, spawns benchmark CLI as child process
- **Frontend**: Static HTML/JS in `web/`, polls APIs every 2 seconds
- **Storage**: SQLite for run history, JSON files for raw results

## What Works

- Model catalog loads from `opencode models` CLI
- Searchable model selector with radio buttons
- Run configuration (modes, sizes, repeats)
- Progress bar with live updates
- Log display with word-wrap
- Start/Stop controls
- Admin DB rebuild/drop

## Current Issues

| Issue | Root Cause |
|-------|------------|
| "Providers and Models" empty | Server wasn't running when tested; works when server is up |
| "Previous Runs" shows no stats | `renderRuns()` works but may fail silently if DB empty or tab switching issue |

## Tasks

### High Priority

- [x] Fix model list not loading on page load (timing/server startup issue)
- [x] Fix Previous Runs tab not displaying run statistics table

### Medium Priority

- [x] Add aggregate statistics view (pass rate by mode, model comparison)
- [x] Add detailed run drill-down (click row to see per-case results)

### Low Priority

- [x] Add export results as CSV/JSON from Previous Runs
- [x] Add auto-refresh toggle for live status during runs

## Recent Updates (2026-02-27)

### Cost Optimization - Minimal Test Suite

Replaced 27 trivial tests (9 cases × 3 sizes) with **6 carefully designed puzzle-style challenges**:

1. **md-ambiguous-lines** - Multiple similar "Mode:" lines requiring context awareness
2. **json-deep-nest** - Deep nesting with duplicate "ssl" keys at different levels
3. **rust-whitespace** - Pure indentation fix (4→8 spaces) with no content change
4. **ts-similar-names** - Distinguish class method from standalone function with same name
5. **json-array-puzzle** - Arrays with duplicate "build" names in different scopes
6. **md-json-embedded** - Markdown with multiple JSON blocks, fix correct timeout value

**Cost Impact**: 78% reduction in API calls (~$0.099 savings per run)  
**Coverage**: Same format diversity (MD, JSON, Rust, TS) with higher quality challenges

See `FIXTURES.md` for detailed test case descriptions.

## Completed Features (2026-02-27)

All planned dashboard features have been implemented:

1. **Model loading with retry** - Automatically retries if server isn't ready at page load
2. **Error handling** - Better connection error messages for all API calls
3. **Aggregate statistics** - Shows pass rates grouped by mode and by model
4. **Run drill-down** - Click any run row to see detailed per-case results in a modal
5. **Export functionality** - Export runs as CSV or JSON for external analysis
6. **Auto-refresh toggle** - User can pause/resume the 2-second polling
7. **Model pricing display** - Shows cost per model, highlights FREE models
8. **Live cost tracking** - Real-time cost accumulation during benchmark runs
9. **Cost persistence** - Total costs saved with run statistics in SQLite and JSON
