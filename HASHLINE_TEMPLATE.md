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

Batch all changes to a file into one `edits` array - edits are atomic (all succeed or none apply).

Prefer writing the payload to disk and invoking `hashline apply --emit-updated --input` (avoids heredoc guardrails and returns fresh anchors automatically):

```bash
hashline apply --emit-updated --input edits.json
```

Example payload (`edits.json`):

```json
{
  "path": "src/main.rs",
  "edits": [
    {"set_line": {"anchor": "4:01", "new_text": "    println!(\"goodbye\");"}},
    {"insert_after": {"anchor": "5:0e", "text": "fn helper() {\n    todo!()\n}"}}
  ]
}
```

Fallback heredoc (fine for simple payloads):

```bash
hashline apply <<'EOF'
{
  "path": "src/main.rs",
  "edits": [
    {"set_line": {"anchor": "4:01", "new_text": "    println!(\"goodbye\");"}},
    {"insert_after": {"anchor": "5:0e", "text": "fn helper() {\n    todo!()\n}"}}
  ]
}
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

## JSON Files

For JSON files, use the JSON-aware commands for semantic editing:

```bash
hashline json-read package.json
```

Output example:

```jsonc
{
  // $.name:cd
  "name": "my-project",
  // $.version:a7
  "version": "1.0.0"
}
```

Anchors that include dots, spaces, or brackets use bracket notation (e.g. `$["a.b"]["c d"]`). Use the same representation when constructing JSON edits.

Prepare edits (`json-edits.json`):
```json
{
  "path": "package.json",
  "edits": [
    {"set_path": {"anchor": "$.version:a7", "value": "1.2.0"}}
  ]
}
```

```bash
hashline json-apply --emit-updated --input json-edits.json
```

```

```

### JSON Operations

**`set_path`** — set value at JSONPath:
```json
{"set_path": {"anchor": "$.version:a7", "value": "1.2.0"}}
```

**`insert_at_path`** — insert into object/array:
```json
{"insert_at_path": {"anchor": "$.dependencies:a1", "key": "lodash", "value": "^4.17.0"}}
```
Provide either `key` (object insertion) or `index` (array insertion); specifying both returns an error.


**`delete_path`** — remove value:
```json
{"delete_path": {"anchor": "$.scripts.test:3b"}}
```

## Exit Codes

- **0** — success
- **1** — hash mismatch (file changed since last read); stderr has updated anchors — copy them and retry
- **2** — other error (bad JSON, file not found, etc.); do not retry without fixing the input

## Error Recovery

**Text files** — stderr shows context lines with `>>>` marking changed lines:

```
1 line has changed since last read. Use the updated LINE:HASH references shown below (>>> marks changed lines).

    3:7f|fn main() {
>>> 4:c9|    println!("changed");
    5:0e|}
```

Copy the updated anchor (`4:c9`) into your edit and retry. Do not re-read the whole file — just update the anchor.

**JSON files** — stderr shows the changed path with `>>>` and then the full re-anchored file:

```
1 anchor has changed since last read. Updated references (>>> marks changed values):

>>> $.version:c9
{
  // $.name:cd
  "name": "my-project",
  // $.version:c9
  "version": "1.1.0"
}
```

Copy the updated anchor (`$.version:c9`) from the `>>>` line into your edit and retry.

## Rules

- Re-read a file with `hashline read` before editing it again (hashes change after every apply), or use `--emit-updated` to get fresh anchors in the apply output
- Batch all edits to one file into a single `hashline apply` call
- Prefer anchor ops (`set_line`, `replace_lines`, `insert_after`) over `replace` — they are safer and more precise
- Never guess a hash — always read first
