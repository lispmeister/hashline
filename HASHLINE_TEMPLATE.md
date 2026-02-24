# Hashline Template

Paste at the **top** of your project's `CLAUDE.md`, `AGENTS.md`, or equivalent — before any other content. Agents weight earlier instructions more heavily; placing these first ensures `hashline` takes precedence over the agent's default edit tools.

---

# Editing Files

**NEVER edit a file you haven't read with `hashline read` in this conversation.**
For all code edits, use the hashline CLI via Bash instead of the built-in Edit tool.
For creating new files, use the Write tool. For deleting files, use `rm`.

## Reading

```bash
hashline read src/main.rs
```

Returns:
```
1:a3|use std::io;
2:05|
3:7f|fn main() {
4:01|    println!("hello");
5:0e|}
```

Each line has a `LINE:HASH` anchor. Use these anchors — not line numbers alone — in edits.

**Partial read** (after editing, verify just the changed region):
```bash
hashline read --start-line 10 --lines 20 src/main.rs
```

## Editing

Always use a heredoc. Batch all changes to a file into one `edits` array - edits are atomic (all succeed or none apply):

```bash
hashline apply << 'EOF'
{
  "path": "src/main.rs",
  "edits": [
    {"set_line": {"anchor": "4:01", "new_text": "    println!(\"goodbye\");"}},
    {"insert_after": {"anchor": "5:0e", "text": "fn helper() {\n    todo!()\n}"}}
  ]
}
EOF
```

Alternatively, write the JSON to a temp file and use `--input` (avoids heredoc shell guard issues with dangerous-looking content):

```bash
hashline apply --input /tmp/edits.json
```

Use `--emit-updated` to get fresh `LINE:HASH` anchors for the changed region without a separate re-read:

```bash
hashline apply --emit-updated << 'EOF'
...
EOF
```

### Operations

**`set_line`** — replace one line:
```json
{"set_line": {"anchor": "4:01", "new_text": "    println!(\"goodbye\");"}}
```

**`replace_lines`** — replace a range (use `"new_text": ""` to delete):
```json
{"replace_lines": {"start_anchor": "3:7f", "end_anchor": "5:0e", "new_text": "fn main() {}"}}
```

**`insert_after`** — insert lines after an anchor (use `"text": ""` to insert a blank line):
```json
{"insert_after": {"anchor": "1:a3", "text": "use std::fs;"}}
```

**`replace`** — exact substring replacement, no anchor needed (use when anchor ops are awkward, e.g. replacing a unique multi-line block). Runs after all anchor edits. Errors if text is not found or matches multiple locations:
```json
{"replace": {"old_text": "old string", "new_text": "new string"}}
```

Use `\n` in strings for multi-line content.

## Exit Codes

- **0** — success
- **1** — hash mismatch (file changed since last read); stderr has updated anchors — copy them and retry
- **2** — other error (bad JSON, file not found, etc.); do not retry without fixing the input

## Error Recovery

On hash mismatch, stderr shows the current file state with `>>>` marking changed lines:

```
1 line has changed since last read. Use the updated LINE:HASH references shown below (>>> marks changed lines).

    3:7f|fn main() {
>>> 4:c9|    println!("changed");
    5:0e|}
```

Copy the updated anchor (`4:c9`) into your edit and retry. Do not re-read the whole file — just update the anchor.

## Rules

- Re-read a file with `hashline read` before editing it again (hashes change after every apply), or use `--emit-updated` to get fresh anchors in the apply output
- Batch all edits to one file into a single `hashline apply` call
- Prefer anchor ops (`set_line`, `replace_lines`, `insert_after`) over `replace` — they are safer and more precise
- Never guess a hash — always read first
