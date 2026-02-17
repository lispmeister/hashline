# Hashline Template

Copy the section below into your project's CLAUDE.md or AGENTS.md.

---

# Editing Files

**NEVER edit a file you haven't read with `hashline read` in this conversation.**
For all code edits, use the hashline CLI via Bash instead of the built-in Edit tool.
For creating new files, use the Write tool. For deleting files, use `rm`.

- `hashline read <file>` — returns `LINE:HASH|content` format
- `cat << 'EOF' | hashline apply` — apply edits (use heredoc to avoid shell escaping issues)
- After every edit, re-read before editing the same file again (hashes change)
- Each apply call validates all anchors before mutating — if any fail, no changes are made

## Reading

`hashline read src/main.rs` returns:
```
1:a3|use std::io;
2:05|
3:7f|fn main() {
4:01|    println!("hello");
5:0e|}
```

The `3:7f` prefix is the anchor — line 3, hash `7f`. Use these anchors in edits.

## Editing

Always use a heredoc to pipe JSON. Batch multiple edits into one `edits` array:

```bash
cat << 'EOF' | hashline apply
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

- **`set_line`** — replace one line: `{"set_line": {"anchor": "4:01", "new_text": "new content"}}`
- **`replace_lines`** — replace a range: `{"replace_lines": {"start_anchor": "3:7f", "end_anchor": "5:0e", "new_text": "fn main() {}"}}`
- **`insert_after`** — add lines after anchor: `{"insert_after": {"anchor": "2:05", "text": "use std::fs;"}}`

For `replace_lines`, use `"new_text": ""` to delete the range. Use `\n` in strings for multi-line content.

## Error Recovery

On hash mismatch (exit code 1), stderr shows the current state:
```
Hash mismatch at line 4: expected 01, got b7. Current content — 4:b7|    println!("world");
```

Copy the updated anchor (`4:b7`) into your edit and retry.
