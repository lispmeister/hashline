#!/bin/bash
# PreToolUse hook: enforce "hashline read before hashline apply".
#
# Reads the session tracking file written by track_hashline.sh (PostToolUse).
# Blocks apply if the target file is absent from the session OR marked stale
# (i.e. the file was applied to but not yet re-read, so anchors are invalid).
#
# Exit codes:
#   0  — allow (not a hashline apply, or file is freshly read)
#   2  — block (file missing or stale)

input=$(cat)
cmd=$(printf '%s' "$input" | jq -r '.tool_input.command // ""')

# Fast path: not a hashline apply command
printf '%s' "$cmd" | head -1 | grep -qE '^[[:space:]]*hashline[[:space:]]+apply\b' || exit 0

session="/tmp/hashline_session_${PPID}"

resolve_path() {
    local p="$1"
    [[ "$p" == /* ]] && printf '%s' "$p" || printf '%s/%s' "$PWD" "$p"
}

# ── extract target file path ──────────────────────────────────────────────────

file=""

# --input variant
if printf '%s' "$cmd" | grep -qF -- '--input'; then
    ifile=$(printf '%s' "$cmd" | sed -n 's/.*--input[[:space:]]\+\([^[:space:]]\+\).*/\1/p')
    if [ -n "$ifile" ] && [ -f "$ifile" ]; then
        file=$(jq -r '.path // ""' "$ifile" 2>/dev/null) || file=""
    fi
fi

# Heredoc / inline JSON variant
if [ -z "$file" ]; then
    file=$(printf '%s' "$cmd" | sed -n 's/.*"path"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -1) || file=""
fi

# Can't determine file — let hashline itself catch any anchor mismatch
[ -z "$file" ] && exit 0
file=$(resolve_path "$file")

# ── check session state ───────────────────────────────────────────────────────

if grep -qxF "read:$file" "$session" 2>/dev/null; then
    exit 0  # freshly read — anchors are valid
fi

if grep -qxF "stale:$file" "$session" 2>/dev/null; then
    printf 'BLOCKED: "%s" was modified by hashline apply but not re-read.\n' "$file" >&2
    printf 'Anchors are stale. Run:\n  hashline read %s\nbefore applying edits.\n' "$file" >&2
    exit 2
fi

# File not in session at all
printf 'BLOCKED: "%s" has not been read with `hashline read` in this session.\n' "$file" >&2
printf 'Run:\n  hashline read %s\nbefore applying edits.\n' "$file" >&2
exit 2
