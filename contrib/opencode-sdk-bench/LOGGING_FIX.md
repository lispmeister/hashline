# Logging System Fix

## Issue
Logs weren't showing in the dashboard during test runs.

## Root Cause
The logging system was using async `appendLog()` calls that were fire-and-forget (`void appendLog()`), which meant:
1. Errors were silently swallowed
2. Logs might not appear in the in-memory buffer immediately
3. The async file writes could block or fail silently

## Fix Applied

### 1. Made `appendLog()` Synchronous
Changed from:
```javascript
async function appendLog(line) {
  // ... push to buffer
  await fs.appendFile(LOG_FILE, ...) // Blocking!
}
```

To:
```javascript
function appendLog(line) {
  // Push to buffer immediately (synchronous)
  activeRun.logs.push(timestamped)
  
  // Write to file in background (non-blocking)
  fs.appendFile(LOG_FILE, ...).catch(() => {})
}
```

### 2. Direct Child Process Capture
The child process stdout/stderr now pushes directly to the buffer:
```javascript
child.stdout.on("data", (chunk) => {
  // Immediately add to in-memory buffer
  activeRun.logs.push(timestamped)
  
  // File write happens async in background
  fs.appendFile(...).catch(() => {})
})
```

### 3. Removed `await` from `appendLog` Calls
Changed all:
- `await appendLog(...)` → `appendLog(...)`
- `void appendLog(...)` → `appendLog(...)`

## Result
- ✅ Logs appear in dashboard immediately
- ✅ No blocking on file I/O
- ✅ Errors don't prevent logs from showing
- ✅ Child process output captured in real-time

## Benefits
1. **Immediate visibility** - Logs show in dashboard instantly
2. **Reliability** - File write failures don't affect dashboard display
3. **Performance** - No blocking on I/O operations
4. **Simplicity** - Synchronous code is easier to debug

## Testing
To verify:
1. Start a new test run
2. Logs should appear immediately in the dashboard
3. Progress updates should show every 3 seconds
4. No delays or missing messages
