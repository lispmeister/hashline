# How to Start the Dashboard

## Important: Run Outside of OpenCode Session

The benchmark tests use OpenCode SDK, so they should be run **outside** of an active OpenCode session to avoid nested permission conflicts.

## Step 1: Exit OpenCode (if running)

Exit your current OpenCode session completely.

## Step 2: Open a Regular Terminal

Open a normal terminal window (not inside OpenCode).

## Step 3: Start the Dashboard

```bash
cd /Users/fix/projects/claude-code/hashline/contrib/opencode-sdk-bench
npm run dashboard
```

You should see:
```
http://localhost:4177
```

## Step 4: Open Browser

Navigate to: http://localhost:4177

## Step 5: Run a Test

1. In the "Providers and Models" section, search for a model (e.g., "big-pickle")
2. Select a model by clicking the radio button
3. Ensure at least one mode is checked (hashline, raw_replace, or patch)
4. Ensure at least one size is checked (small is recommended)
5. Click the **RUN** button
6. Watch the "Live Status" section for progress

## What Should Happen

- Status changes to "Running"
- Progress bar starts filling
- Logs appear showing test execution
- After completion, results appear in "Previous Runs" tab

## Troubleshooting

### Models not showing
- Hard refresh browser (Cmd+Shift+R or Ctrl+Shift+R)
- Check server is running: `curl http://localhost:4177/api/models`

### RUN button doesn't work
- Open browser console (F12) and check for JavaScript errors
- Verify a model is selected (radio button checked)
- Verify at least one mode and one size is checked

### Test hangs on "Running"
If testing from within an OpenCode session, you'll hit nested permission issues. **Exit OpenCode completely** and run from a normal terminal.

### Port already in use
```bash
# Kill existing server
pkill -f "node server.js"

# Or use different port
PORT=4178 npm run dashboard
```

## Quick Test (Command Line)

To test without the dashboard:

```bash
cd /Users/fix/projects/claude-code/hashline/contrib/opencode-sdk-bench

# Run a single test
npm run run -- --mode hashline --sizes small --repeats 1 --model opencode/big-pickle

# This should complete in 30-60 seconds and output a JSON file path
```

## Files Created

After a successful run:
- `runs/<timestamp>-<mode>-<model>.json` - Full test results
- `runs/bench.sqlite` - Database with run history
- `runs/work/` - Temporary test files

## Viewing Results

1. Click "Previous Runs" tab in dashboard
2. See aggregate statistics by mode and model
3. Click any run row to drill down into per-case results
4. Export as CSV or JSON for external analysis

## Cost Tracking

The dashboard shows live cost accumulation during runs and saves total cost with each run. With the new minimal test suite (6 tests), expect:
- FREE models: $0.00
- Paid models: ~$0.02-0.03 per run

## Next Steps

After verifying the dashboard works:
1. Test different models (search for "claude", "gpt", etc.)
2. Compare modes (hashline vs raw_replace vs patch)
3. Analyze results in "Previous Runs" tab
4. Export data for further analysis
