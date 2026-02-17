# Hashline

A content-addressable line editing tool for AI coding assistants. Instead of requiring models to reproduce exact text for edits, Hashline tags each line with a short hash anchor — models reference lines by `LINE:HASH` instead of matching verbatim content.

Based on the [Hashline concept by Can Bölük](https://blog.can.ac/2026/02/12/the-harness-problem/), which identifies the "harness problem" — the interface between model output and workspace edits is where most practical failures occur, not in the model's reasoning.

## Why

Current edit approaches fail in predictable ways:

| Approach | Failure mode |
|----------|-------------|
| **Patch/diff** (Codex) | Strict formatting rules; 50%+ failure rate for some models |
| **String replacement** (Claude Code) | Requires character-perfect reproduction including whitespace |
| **Neural merge** (Cursor) | Requires fine-tuning separate 70B models |

Hashline sidesteps all of these. Each line gets a 2-character hex hash derived from its content:

```
1:a3|function hello() {
2:f1|  return "world";
3:0e|}
```

Models edit by referencing anchors (`2:f1`) rather than reproducing text. Benchmarks from the original article show:

- **10x improvement** for weaker models (Grok Code Fast: 6.7% → 68.3%)
- **~20% fewer output tokens** across all models
- **Staleness detection** — hash mismatches catch edits to changed files before corruption

## Install

### From release binary

```sh
curl -fsSL https://raw.githubusercontent.com/lispmeister/hashline/main/install.sh | sh
```

Or download directly from [Releases](https://github.com/lispmeister/hashline/releases) and place the binary on your `PATH`.

### Homebrew (planned)

```sh
brew install lispmeister/tap/hashline
```

### From source

```sh
cargo install --path .
```

## Usage

### Read a file with hashline annotations

```sh
hashline read src/main.rs
```

Output:
```
1:4a|use std::io;
2:b2|
3:7f|fn main() {
4:01|    println!("hello");
5:0e|}
```

### Apply edits

Pipe JSON edits to stdin:

```sh
echo '{"path":"src/main.rs","edits":[
  {"set_line":{"anchor":"4:01","new_text":"    println!(\"goodbye\");"}}
]}' | hashline apply
```

### Edit operations

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

### Error handling

On hash mismatch (file changed since last read), exit code 1 and stderr shows updated refs:

```
1 line has changed since last read. Use the updated LINE:HASH references shown below (>>> marks changed lines).

    3:7f|fn main() {
>>> 4:c9|    println!("changed");
    5:0e|}
```

Retry with the updated anchor (`4:c9` instead of `4:01`).

### Hash a file (debugging)

```sh
hashline hash src/main.rs
```

## Claude Code Integration

Add to your project's `CLAUDE.md` (or globally to `~/.claude/CLAUDE.md`):

```markdown
# Editing files

For all code edits, use the hashline CLI via Bash instead of the built-in Edit tool:

- **Read**: `hashline read <file>` — returns LINE:HASH|content format
- **Edit**: `echo '{"path":"<file>","edits":[...]}' | hashline apply`
- After every edit, re-read before editing the same file again (hashes changed)
- On hash mismatch errors (exit code 1), copy the updated LINE:HASH refs from stderr and retry
- Each edit call validates all anchors against the original file state before mutating
- Edits are applied atomically — if any anchor fails validation, no changes are made
```

## Install Script

The `install.sh` script detects your platform and installs the appropriate binary:

```sh
# Default install to ~/.local/bin
curl -fsSL https://raw.githubusercontent.com/lispmeister/hashline/main/install.sh | sh

# Custom install directory
curl -fsSL https://raw.githubusercontent.com/lispmeister/hashline/main/install.sh | sh -s -- --prefix /usr/local
```

## License

MIT
