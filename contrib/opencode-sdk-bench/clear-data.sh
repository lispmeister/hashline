#!/bin/bash
# Clear all old benchmark data

echo "Clearing old benchmark data..."

cd "$(dirname "$0")"

# Stop server
echo "Stopping server..."
pkill -f "node server.js" 2>/dev/null

# Clear database
echo "Removing database..."
rm -f runs/bench.sqlite*

# Clear test results
echo "Removing old test results..."
rm -f runs/*.json

# Clear logs
echo "Removing logs..."
rm -f runs/*.log

# Clear work directories
echo "Removing work directories..."
rm -rf runs/work

echo "✅ All old data cleared!"
echo ""
echo "Restart dashboard with: npm run dashboard"
