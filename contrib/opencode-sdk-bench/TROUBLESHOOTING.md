# Dashboard Troubleshooting

## Quick Fixes

### Issue: "Providers and Models" section is empty

**Cause**: JavaScript syntax error was preventing the page from loading properly.  
**Fixed**: Corrected missing `if (!model) {` statement in `startRun()` function.

**Verify the fix**:
```bash
# Check server is running
curl http://localhost:4177/api/models

# Should return JSON with providers array
```

### Issue: "Previous Runs" tab is blank

**Cause**: Same JavaScript error preventing page initialization.  
**Solution**: Refresh browser (Cmd+Shift+R or Ctrl+Shift+R to clear cache)

### Issue: "Admin" tab doesn't work

**Cause**: Same JavaScript error.  
**Solution**: Hard refresh browser after server restart.

## Startup Checklist

1. **Start the server**:
   ```bash
   cd contrib/opencode-sdk-bench
   npm run dashboard
   ```

2. **Verify server is running**:
   ```bash
   curl http://localhost:4177/api/models
   # Should return JSON with providers
   ```

3. **Open browser**: http://localhost:4177

4. **Hard refresh**: Press Cmd+Shift+R (Mac) or Ctrl+Shift+R (Windows/Linux)

5. **Check browser console**: Open DevTools (F12) and look for errors

## Common Issues

### Models not loading

**Symptoms**: "Providers and Models" section shows "Loading models..." forever

**Solutions**:
1. Check if `opencode` CLI is installed:
   ```bash
   which opencode
   opencode models | head -5
   ```

2. Check server logs:
   ```bash
   # Server logs to stdout, check the terminal where you ran npm run dashboard
   ```

3. Manually test the API:
   ```bash
   curl http://localhost:4177/api/models | jq '.providers | length'
   # Should return a number > 0
   ```

### Previous runs show "No runs stored"

**Symptoms**: Runs table is empty even though you've run tests

**Solutions**:
1. Check if runs directory exists:
   ```bash
   ls -la contrib/opencode-sdk-bench/runs/
   ```

2. Check if JSON files exist:
   ```bash
   ls contrib/opencode-sdk-bench/runs/*.json
   ```

3. Rebuild database:
   - Go to Admin tab
   - Click "Drop + Rebuild DB from JSON"
   - Refresh page

### Port already in use

**Symptoms**: Error "Port 4177 is already in use"

**Solution**:
```bash
# Kill existing server
pkill -f "node server.js"

# Or use different port
PORT=4178 npm run dashboard
```

### Auto-refresh not working

**Symptoms**: Status doesn't update during runs

**Solutions**:
1. Check "Auto-refresh (2s)" checkbox is enabled
2. Open browser console (F12) and look for errors
3. Hard refresh browser (Cmd+Shift+R)

## API Endpoints Testing

Test all endpoints manually:

```bash
# Models (with pricing)
curl http://localhost:4177/api/models | jq '.providers[0]'

# Status
curl http://localhost:4177/api/status | jq

# Runs history
curl http://localhost:4177/api/runs | jq '.runs | length'

# Progress
curl http://localhost:4177/api/progress | jq

# Logs
curl http://localhost:4177/api/log?tail=10 | jq '.lines'

# Run detail
curl 'http://localhost:4177/api/run/detail?file=FILENAME.json' | jq '.report'
```

## Browser Developer Tools

Open DevTools (F12) and check:

1. **Console tab**: Look for JavaScript errors
2. **Network tab**: Check if API calls are succeeding (200 status)
3. **Application tab** > Local Storage: Clear if needed

## Server Logs

The server logs all operations to stdout. Look for:
- `http://localhost:4177` - Server started
- `[PROGRESS]` - Test run progress
- Error messages

## Complete Reset

If nothing works, reset everything:

```bash
# Stop server
pkill -f "node server.js"

# Remove database
rm contrib/opencode-sdk-bench/runs/bench.sqlite*

# Clear runs (optional - saves your test results)
# rm contrib/opencode-sdk-bench/runs/*.json

# Reinstall dependencies
cd contrib/opencode-sdk-bench
npm install

# Rebuild TypeScript
npm run build

# Start fresh
npm run dashboard
```

Then:
1. Open http://localhost:4177
2. Hard refresh (Cmd+Shift+R)
3. Check browser console for errors

## Verify Installation

```bash
cd contrib/opencode-sdk-bench

# Check dependencies
npm list --depth=0

# Check TypeScript build
npm run build

# Check fixtures exist
ls fixtures/
# Should show: json-array-puzzle, json-deep-nest, md-ambiguous-lines, etc.

# Check OpenCode CLI
opencode --version
opencode models | head -5
```

## Still Not Working?

1. Check the browser console (F12) for JavaScript errors
2. Check the server terminal for error messages
3. Test API endpoints with curl (see above)
4. Make sure you're on http://localhost:4177 (not https)
5. Try a different browser
6. Check firewall isn't blocking port 4177
