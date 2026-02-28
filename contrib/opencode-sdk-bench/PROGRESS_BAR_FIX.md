# Progress Bar Fix

## Issue
The progress bar wasn't moving during test runs.

## Root Cause
The progress bar was calculating percentage based on **modes completed** (0/1 = 0%) instead of **test cases completed** (5/6 = 83%).

Since each run only executes one mode at a time, the bar stayed at 0% until the entire run finished, then jumped to 100%.

## Fix Applied
Changed `renderProgressBar()` to calculate progress based on:
- Primary: `completedAttempts / totalAttempts` (test case progress)
- Fallback: `completedJobs / totalJobs` (mode progress, for backward compatibility)

## Result
Progress bar now updates as each test case completes:
- 1/6 tests = 17%
- 2/6 tests = 33%
- 3/6 tests = 50%
- 4/6 tests = 67%
- 5/6 tests = 83%
- 6/6 tests = 100%

## To See the Fix
**Hard refresh your browser**: Cmd+Shift+R (Mac) or Ctrl+Shift+R (Windows/Linux)

The progress bar will now show real-time updates as tests complete!
