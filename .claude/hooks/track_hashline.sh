#!/bin/bash
# PostToolUse hook: track hashline read/apply calls for session state.
#
# Session file: /tmp/hashline_session_<PPID>
# Entries:
#   read:<file>   — file was read; anchors are fresh
#   stale:<file>  — file was applied but not yet re-read; anchors are stale
#
# PPID is the Claude Code process PID — consistent across all hooks in the session.

input=$(cat)
cmd=$(printf '%s' "$input" | jq -r '.tool_input.command // ""')
is_error=$(printf '%s' "$input" | jq -r '.tool_response.isError // false')

session="/tmp/hashline_session_${PPID}"

# ── helpers ──────────────────────────────────────────────────────────────────

extract_read_file() {
    # File is the last non-flag, non-numeric argument after 'hashline read'
    printf '%s' "$cmd" \
        | sed -E 's/.*hashline[[:space:]]+read[[:space:]]*//' \
        | tr ' \t' '\n' \
        | grep -Ev '^-|^[0-9]+$|^$' \
        | tail -1
}

extract_apply_file() {
    local f=""
    # --input variant: hashline apply --input /tmp/edits.json
    if printf '%s' "$cmd" | grep -qF -- '--input'; then
        local ifile
        ifile=$(printf '%s' "$cmd" | sed -n 's/.*--input[[:space:]]\+\([^[:space:]]\+\).*/\1/p')
        if [ -n "$ifile" ] && [ -f "$ifile" ]; then
            f=$(jq -r '.path // ""' "$ifile" 2>/dev/null) || f=""
        fi
    fi
    # Heredoc / inline JSON variant: "path": "src/main.rs"
    if [ -z "$f" ]; then
        f=$(printf '%s' "$cmd" | sed -n 's/.*"path"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -1) || f=""
    fi
    printf '%s' "$f"
}

resolve_path() {
    local p="$1"
    [[ "$p" == /* ]] && printf '%s' "$p" || printf '%s/%s' "$PWD" "$p"
}

mark_read() {
    local file="$1"
    (grep -vxF "read:$file" "$session" 2>/dev/null || true) \
        | grep -vxF "stale:$file" > "${session}.tmp" 2>/dev/null || true
    printf 'read:%s\n' "$file" >> "${session}.tmp"
    mv "${session}.tmp" "$session"
}

mark_stale() {
    local file="$1"
    (grep -vxF "read:$file" "$session" 2>/dev/null || true) \
        | grep -vxF "stale:$file" > "${session}.tmp" 2>/dev/null || true
    printf 'stale:%s\n' "$file" >> "${session}.tmp"
    mv "${session}.tmp" "$session"
}

# ── main ─────────────────────────────────────────────────────────────────────

is_read=false
is_apply=false
printf '%s' "$cmd" | head -1 | grep -qE '^[[:space:]]*hashline[[:space:]]+read\b'  && is_read=true  || true
printf '%s' "$cmd" | head -1 | grep -qE '^[[:space:]]*hashline[[:space:]]+apply\b' && is_apply=true || true

if $is_read && [ "$is_error" = "false" ]; then
    file=$(extract_read_file)
    [ -n "$file" ] && file=$(resolve_path "$file") && mark_read "$file"

elif $is_apply && [ "$is_error" = "false" ]; then
    file=$(extract_apply_file)
    if [ -n "$file" ]; then
        file=$(resolve_path "$file")
        # --emit-updated provides fresh anchors in the output → still fresh
        if printf '%s' "$cmd" | grep -qF -- '--emit-updated'; then
            mark_read "$file"
        else
            mark_stale "$file"
        fi
    fi
fi

exit 0
