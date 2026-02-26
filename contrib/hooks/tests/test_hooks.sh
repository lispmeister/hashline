#!/bin/bash
# Unit tests for .claude/hooks/check_before_apply.sh and track_hashline.sh
#
# Tests feed synthetic PreToolUse / PostToolUse JSON to each script via stdin
# and assert exit codes and session state.
#
# Run from the project root:
#   bash .claude/hooks/tests/test_hooks.sh
#
# Design note on PPID:
#   Hook scripts key their session file on $PPID (the parent process PID).
#   Pipes create an intermediate subprocess, so PPID inside a piped `bash script`
#   does NOT equal $$ of this test script.  We avoid pipes and use temp files for
#   stdin so that hooks always run as direct children of this script, keeping
#   PPID == $$ throughout.

HOOKS="$(cd "$(dirname "$0")/.." && pwd)"
CHECK="$HOOKS/check_before_apply.sh"
TRACK="$HOOKS/track_hashline.sh"

# Session file the hooks will use (keyed by PPID = this script's $$ for direct children)
SESSION="/tmp/hashline_session_$$"
STDIN_TMP="/tmp/hashline_test_stdin_$$"

pass=0; fail=0

# ── helpers ──────────────────────────────────────────────────────────────────

cleanup() { rm -f "$SESSION" "${SESSION}.tmp" "$STDIN_TMP"; }
trap cleanup EXIT

reset_session() { rm -f "$SESSION" "${SESSION}.tmp"; }

set_session() { printf '%s\n' "$@" > "$SESSION"; }

# Build PreToolUse JSON
pre_input() { jq -n --arg cmd "$1" '{"tool_input":{"command":$cmd}}'; }

# Build PostToolUse JSON
post_input() {
    local cmd="$1" is_error="${2:-false}"
    jq -n --arg cmd "$cmd" --argjson err "$is_error" \
        '{"tool_input":{"command":$cmd},"tool_response":{"isError":$err}}'
}

# Run a hook as a direct child (stdin from temp file, not a pipe) so PPID == $$
run_hook() {
    local script="$1" input="$2"
    printf '%s' "$input" > "$STDIN_TMP"
    bash "$script" < "$STDIN_TMP"
}

ok()   { printf 'PASS  %s\n' "$1"; pass=$((pass+1)); }
bad()  { printf 'FAIL  %s\n' "$1"; [ $# -gt 1 ] && printf '      %s\n' "${@:2}"; fail=$((fail+1)); }

expect() {
    local name="$1" input="$2" script="$3" expected_exit="$4" stderr_pat="${5:-}"
    local actual_exit=0 actual_stderr STDERR_TMP; STDERR_TMP=$(mktemp)
    run_hook "$script" "$input" > /dev/null 2>"$STDERR_TMP" || actual_exit=$?
    actual_stderr=$(cat "$STDERR_TMP"); rm -f "$STDERR_TMP"

    if [ "$actual_exit" -ne "$expected_exit" ]; then
        bad "$name" "expected exit $expected_exit, got $actual_exit" "stderr: $actual_stderr"
        return
    fi
    if [ -n "$stderr_pat" ] && ! printf '%s' "$actual_stderr" | grep -qF "$stderr_pat"; then
        bad "$name" "expected stderr to contain: $stderr_pat" "actual: $actual_stderr"
        return
    fi
    ok "$name"
}

track() { run_hook "$TRACK" "$1" > /dev/null 2>&1; }

session_has()   { grep -qxF "$1" "$SESSION" 2>/dev/null; }
session_lacks() { ! grep -qxF "$1" "$SESSION" 2>/dev/null; }

assert() {
    local desc="$1" cond="$2"
    if eval "$cond"; then
        ok "$desc"
    else
        bad "$desc" "condition: $cond" "session: $(cat "$SESSION" 2>/dev/null | tr '\n' '|')"
    fi
}

# ── check_before_apply.sh ─────────────────────────────────────────────────────

printf '\n=== check_before_apply.sh ===\n\n'

reset_session
expect "non-apply command is allowed" \
    "$(pre_input 'cargo build')" "$CHECK" 0

reset_session
expect "apply without prior read is blocked" \
    "$(pre_input 'hashline apply << '"'"'EOF'"'"'
{"path": "/tmp/hashline_test_file.rs", "edits": []}
EOF')" "$CHECK" 2 "has not been read"

reset_session
set_session "read:/tmp/hashline_test_file.rs"
expect "apply after read is allowed" \
    "$(pre_input 'hashline apply << '"'"'EOF'"'"'
{"path": "/tmp/hashline_test_file.rs", "edits": []}
EOF')" "$CHECK" 0

reset_session
set_session "stale:/tmp/hashline_test_file.rs"
expect "apply on stale file is blocked" \
    "$(pre_input 'hashline apply << '"'"'EOF'"'"'
{"path": "/tmp/hashline_test_file.rs", "edits": []}
EOF')" "$CHECK" 2 "stale"

# --input variant: path extracted from JSON file
reset_session
TMPJSON=$(mktemp /tmp/test_edits_XXXXXX.json)
printf '{"path": "/tmp/hashline_test_file.rs", "edits": []}' > "$TMPJSON"
set_session "read:/tmp/hashline_test_file.rs"
expect "--input variant allowed when file is read" \
    "$(pre_input "hashline apply --input $TMPJSON")" "$CHECK" 0
rm -f "$TMPJSON"

# Relative path in apply JSON → resolved against PWD, matched against absolute in session
reset_session
ABS_FILE="$PWD/relative_test_dummy.rs"
set_session "read:$ABS_FILE"
expect "relative path in apply JSON resolved to match absolute in session" \
    "$(pre_input 'hashline apply << '"'"'EOF'"'"'
{"path": "relative_test_dummy.rs", "edits": []}
EOF')" "$CHECK" 0

reset_session
expect "json-apply without prior read is blocked" \
    "$(pre_input 'hashline json-apply << '"'"'EOF'"'"'
{"path": "/tmp/hashline_test_file.json", "edits": []}
EOF')" "$CHECK" 2 "has not been read"

reset_session
set_session "read:/tmp/hashline_test_file.json"
expect "json-apply after read is allowed" \
    "$(pre_input 'hashline json-apply << '"'"'EOF'"'"'
{"path": "/tmp/hashline_test_file.json", "edits": []}
EOF')" "$CHECK" 0

reset_session
set_session "stale:/tmp/hashline_test_file.json"
expect "json-apply on stale file is blocked" \
    "$(pre_input 'hashline json-apply << '"'"'EOF'"'"'
{"path": "/tmp/hashline_test_file.json", "edits": []}
EOF')" "$CHECK" 2 "stale"


# ── track_hashline.sh ─────────────────────────────────────────────────────────

printf '\n=== track_hashline.sh ===\n\n'

reset_session
track "$(post_input "hashline read /tmp/hashline_test_file.rs")"
assert "successful read marks file as read" "session_has 'read:/tmp/hashline_test_file.rs'"

reset_session
track "$(post_input "hashline read /tmp/hashline_test_file.rs" true)"
assert "failed read does not mark file" "session_lacks 'read:/tmp/hashline_test_file.rs'"

reset_session
track "$(post_input "hashline read --start-line 10 --lines 20 /tmp/hashline_test_file.rs")"
assert "partial read marks file as read" "session_has 'read:/tmp/hashline_test_file.rs'"

reset_session
track "$(post_input "hashline json-read /tmp/hashline_test_file.json")"
assert "json-read marks file as read" "session_has 'read:/tmp/hashline_test_file.json'"

reset_session
track "$(post_input "hashline json-read /tmp/hashline_test_file.json" true)"
assert "failed json-read does not mark file" "session_lacks 'read:/tmp/hashline_test_file.json'"


reset_session
track "$(post_input "cargo build")"
assert "non-hashline command does not affect session" \
    "session_lacks 'read:/tmp/hashline_test_file.rs'"

# apply (no --emit-updated) → read → stale
reset_session
set_session "read:/tmp/hashline_test_file.rs"
track "$(post_input 'hashline apply << '"'"'EOF'"'"'
{"path": "/tmp/hashline_test_file.rs", "edits": []}
EOF')"
assert "apply marks file stale" "session_has 'stale:/tmp/hashline_test_file.rs'"
assert "apply removes prior read entry" "session_lacks 'read:/tmp/hashline_test_file.rs'"

# apply --emit-updated → stays fresh
reset_session
set_session "read:/tmp/hashline_test_file.rs"
track "$(post_input 'hashline apply --emit-updated << '"'"'EOF'"'"'
{"path": "/tmp/hashline_test_file.rs", "edits": []}
EOF')"
assert "--emit-updated apply keeps file as read" "session_has 'read:/tmp/hashline_test_file.rs'"
assert "--emit-updated apply does not mark stale" "session_lacks 'stale:/tmp/hashline_test_file.rs'"

# json-apply (no --emit-updated) → read → stale
reset_session
set_session "read:/tmp/hashline_test_file.json"
track "$(post_input 'hashline json-apply << '"'"'EOF'"'"'
{"path": "/tmp/hashline_test_file.json", "edits": []}
EOF')"
assert "json-apply marks file stale" "session_has 'stale:/tmp/hashline_test_file.json'"
assert "json-apply removes prior read entry" "session_lacks 'read:/tmp/hashline_test_file.json'"

# json-apply --emit-updated → stays fresh
reset_session
set_session "read:/tmp/hashline_test_file.json"
track "$(post_input 'hashline json-apply --emit-updated << '"'"'EOF'"'"'
{"path": "/tmp/hashline_test_file.json", "edits": []}
EOF')"
assert "--emit-updated json-apply keeps file as read" "session_has 'read:/tmp/hashline_test_file.json'"
assert "--emit-updated json-apply does not mark stale" "session_lacks 'stale:/tmp/hashline_test_file.json'"


# failed apply → session unchanged
reset_session
set_session "read:/tmp/hashline_test_file.rs"
track "$(post_input 'hashline apply --input /tmp/hashline_test_file.rs' true)"
assert "failed apply does not mark stale" "session_lacks 'stale:/tmp/hashline_test_file.rs'"
assert "failed apply preserves read entry" "session_has 'read:/tmp/hashline_test_file.rs'"

# re-read after stale → back to fresh
reset_session
set_session "stale:/tmp/hashline_test_file.rs"
track "$(post_input "hashline read /tmp/hashline_test_file.rs")"
assert "re-read after stale transitions back to read" "session_has 'read:/tmp/hashline_test_file.rs'"
assert "re-read after stale removes stale entry" "session_lacks 'stale:/tmp/hashline_test_file.rs'"

# Regression: "Bash(hashline read:*)" permission string in Python heredoc → no false positive
reset_session
track "$(post_input 'python3 - << '"'"'PYEOF'"'"'
import json
new_settings = {"allow": ["Bash(hashline read:*)", "Bash(hashline apply:*)"]}
print(json.dumps(new_settings))
PYEOF')"
assert "permission string in Python heredoc is not a false-positive read" \
    "session_lacks 'read:/tmp/hashline_test_file.rs'"
spurious=$(grep '^read:' "$SESSION" 2>/dev/null || true)
if [ -z "$spurious" ]; then
    ok "no spurious read entries from Python heredoc"
else
    bad "spurious read entries from Python heredoc" "$spurious"
fi

# Regression: "hashline read" text embedded inside an apply payload → no false positive
reset_session
track "$(post_input 'hashline apply << '"'"'EOF'"'"'
{"path": "/tmp/hashline_test_file.rs", "edits": [
  {"replace": {"old_text": "hashline read src/lib.rs", "new_text": "use hashline read"}}
]}
EOF')"
assert "hashline read text inside apply payload is not a false-positive read" \
    "session_lacks 'read:/tmp/hashline_test_file.rs'"
assert "apply with embedded 'hashline read' text still marks file stale" \
    "session_has 'stale:/tmp/hashline_test_file.rs'"

# ── summary ───────────────────────────────────────────────────────────────────

printf '\n%d passed, %d failed\n' "$pass" "$fail"
[ "$fail" -eq 0 ]
