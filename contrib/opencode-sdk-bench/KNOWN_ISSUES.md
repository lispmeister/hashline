# Known Issues

## RUN Button - OpenCode SDK Permission Prompts

### Issue
When you click RUN in the dashboard, the test starts but then hangs waiting for OpenCode SDK permission prompts.

### Root Cause
The OpenCode SDK runs in interactive mode and asks for permission to access directories like `/tmp/*`. This blocks the benchmark runner which expects non-interactive execution.

### Symptoms
- Dashboard shows "Running" status
- `runs/active.log` shows the CLI started
- No `progress.json` file is created
- OpenCode log (`~/.local/share/opencode/log/`) shows `permission.asked` events
- Test never completes

### Current Workaround

**Option 1: Grant permissions ahead of time**
1. Run `opencode` CLI manually once
2. Grant all requested permissions
3. The SDK should remember the permissions

**Option 2: Configure non-interactive mode (if available)**
Check OpenCode SDK documentation for:
- Environment variables to disable prompts
- Configuration file for pre-approved permissions
- Non-interactive mode flags

**Option 3: Use a different model provider**
Test with Anthropic or Google models directly (not through OpenCode/Zen):
```bash
# This may work if you have direct API keys configured
npm run run -- --mode hashline --sizes small --model anthropic/claude-3-5-haiku --repeats 1
```

### Temporary Fix Applied

The server now kills stale OpenCode processes before starting a new run to prevent port conflicts:
```javascript
execSync("pkill -f 'opencode serve' 2>/dev/null || true")
```

However, this doesn't solve the permission prompt issue.

### Needed Fix

The benchmark runner needs to:
1. Configure OpenCode SDK in non-interactive mode
2. Pre-grant permissions programmatically
3. Or use a permissions configuration file

Check `@opencode-ai/sdk` documentation for:
- `OPENCODE_NO_PROMPTS` environment variable
- Permission configuration options
- Headless/CI mode

### Status

**RUN button works correctly** - the issue is in OpenCode SDK configuration, not the dashboard.

The test DOES start, but waits for user input that never comes in automated mode.
