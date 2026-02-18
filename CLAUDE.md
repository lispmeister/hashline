# Editing files

For all code edits, use the hashline CLI via Bash instead of the built-in Edit tool:

- **Read**: `hashline read <file>` — returns LINE:HASH|content format
- **Partial read**: `hashline read --start-line N --lines M <file>` — read M lines starting at line N
- **Edit**: `echo '{"path":"<file>","edits":[...]}' | hashline apply`
- After every edit, re-read before editing the same file again (hashes changed)
- On hash mismatch errors (exit code 1), copy the updated LINE:HASH refs from stderr and retry
- Each edit call validates all anchors against the original file state before mutating
- Edits are applied atomically — if any anchor fails validation, no changes are made

## Edit operations

**`set_line`** — replace one line:
```json
{"set_line": {"anchor": "4:01", "new_text": "    println!(\"goodbye\");"}}
```

**`replace_lines`** — replace a range (or delete with `"new_text": ""`):
```json
{"replace_lines": {"start_anchor": "3:7f", "end_anchor": "5:0e", "new_text": "fn main() {}"}}
```

**`insert_after`** — add lines after an anchor:
```json
{"insert_after": {"anchor": "2:b2", "text": "use std::fs;"}}
```
