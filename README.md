# Hashline

[![CI](https://github.com/lispmeister/hashline/actions/workflows/ci.yml/badge.svg)](https://github.com/lispmeister/hashline/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/lispmeister/hashline)](https://github.com/lispmeister/hashline/releases/latest)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![Changelog](https://img.shields.io/badge/changelog-Keep%20a%20Changelog-orange)](CHANGELOG.md)

**AI coding agents fail at edits, not at reasoning.** Hashline fixes the interface.

Instead of asking models to reproduce exact text or generate fragile diffs, Hashline tags each line with a short content hash. Models reference lines by `LINE:HASH` anchor — if the file changes, the hash changes, and stale edits are rejected before any corruption occurs.

Based on the [Hashline concept by Can Bölük](https://blog.can.ac/2026/02/12/the-harness-problem/).

## Results

| Model | Without Hashline | With Hashline |
|-------|-----------------|---------------|
| Grok Code Fast | 6.7% | 68.3% (**10x**) |
| All models | baseline | ~20% fewer output tokens |

The improvement is largest for weaker models — Hashline makes cheap models viable for real editing tasks.

## How It Works

```
1:a3|function hello() {
2:f1|  return "world";
3:0e|}
```

Each line gets a `LINE:HASH` prefix. To edit line 2, the model uses anchor `2:f1` — not the text itself. If the file has changed since the model last read it, the hash won't match and the edit is rejected with the correct updated anchors. No silent corruption.

**Heuristics handle real-world model output:** Hashline automatically strips accidentally echoed prefixes, restores dropped indentation, detects when a model merges adjacent lines, undoes formatting rewraps, and normalizes confusable Unicode characters. The model doesn't need to be perfect — Hashline recovers from common mistakes.

## Install

### Homebrew (macOS and Linux)
```sh
brew install lispmeister/hashline/hashline
```

Installs the `hashline` binary and man pages (`man hashline`, `man hashline-read`, etc.).

To upgrade:

```sh
brew upgrade hashline
```

### From release binary

```sh
curl -fsSL https://raw.githubusercontent.com/lispmeister/hashline/main/install.sh | sh
```

Options:

```sh
# Custom install directory (default: ~/.local/bin)
curl -fsSL https://raw.githubusercontent.com/lispmeister/hashline/main/install.sh | sh -s -- --prefix /usr/local/bin

# Specific version
curl -fsSL https://raw.githubusercontent.com/lispmeister/hashline/main/install.sh | sh -s -- --version v0.1.10
```

Pre-built binaries for macOS (Apple Silicon, Intel), Linux (x86_64, ARM64), and Windows (x86_64). Or download from [Releases](https://github.com/lispmeister/hashline/releases).

### From source

```sh
cargo install --path .
```

## Usage

### The full loop

```bash
# 1. Read — get LINE:HASH anchors
hashline read src/main.rs

# 2. Edit — reference anchors, batch changes, atomic apply (save as edits.json)
```json
{
  "path": "src/main.rs",
  "edits": [
    {"set_line": {"anchor": "4:01", "new_text": "    println!(\"goodbye\");"}},
    {"insert_after": {"anchor": "5:0e", "text": "// end of main"}}
  ]
}
```

```bash
hashline apply --emit-updated --input edits.json
```

# 3. Verify — re-read just the changed region (useful if you skipped --emit-updated)
```bash
hashline read --start-line 4 --lines 3 src/main.rs
```


### Edit operations

**`set_line`** — replace one line:
```json
{"set_line": {"anchor": "4:01", "new_text": "    println!(\"goodbye\");"}}
```

**`replace_lines`** — replace a range (use `"new_text": ""` to delete):
```json
{"replace_lines": {"start_anchor": "3:7f", "end_anchor": "5:0e", "new_text": "fn main() {}"}}
```

**`insert_after`** — add lines after an anchor (use `"text": ""` to insert a blank line):
```json
{"insert_after": {"anchor": "2:b2", "text": "use std::fs;"}}
```

**`replace`** — exact substring replacement, no anchor needed:
```json
{"replace": {"old_text": "old string", "new_text": "new string"}}
```

Errors if the text is not found or matches more than one location. Runs after all anchor edits.

### Error handling

On hash mismatch (exit code 1), stderr shows the current file state with `>>>` marking changed lines:

```
1 line has changed since last read. Use the updated LINE:HASH references shown below (>>> marks changed lines).

    3:7f|fn main() {
>>> 4:c9|    println!("changed");
    5:0e|}
```

Copy the updated anchor (`4:c9`) and retry. No need to re-read the whole file.

**Exit codes:** 0 = success, 1 = hash mismatch (retry with updated anchors), 2 = other error.

### Partial reads

After editing a large file, verify just the changed region:

```sh
hashline read --start-line 130 --lines 25 src/main.rs
```

### Hash a file (debugging)

```sh
hashline hash src/main.rs
```

### JSON-aware editing

For JSON files, Hashline supports semantic editing using JSONPath-based anchors:

```bash
# 1. Read JSON with anchors
hashline json-read package.json

# Output example:
{
  // $.name:cd
  "name": "my-project",
  // $.version:a7
  "version": "1.0.0",
  // $.dependencies:27
  "dependencies": {
  // $.dependencies.express:39
  "express": "^4.17.1"
  }
}
Anchors that include dots, spaces, or brackets are emitted with bracket notation (for example `$["a.b"]["c d"]`). Use the same form when constructing JSON edits.


# 2. Apply semantic JSON edits (save as json-edits.json)
```json
{
  "path": "package.json",
  "edits": [
    {"set_path": {"anchor": "$.version:a7", "value": "1.2.0"}},
    {"set_path": {"anchor": "$.dependencies:27", "value": {"express": "^4.17.1", "lodash": "^4.17.0"}}}
  ]
}
```

```bash
hashline json-apply --emit-updated --input json-edits.json
```

### JSON edit operations

**`set_path`** — set a value at JSONPath:
```json
{"set_path": {"anchor": "$.version:a7", "value": "1.2.0"}}
```

**`insert_at_path`** — insert into object/array:
```json
{"insert_at_path": {"anchor": "$.dependencies:27", "key": "lodash", "value": "^4.17.0"}}
```
Provide either `key` (object insertion) or `index` (array insertion); specifying both returns an error.


**`delete_path`** — remove a value:
```json
{"delete_path": {"anchor": "$.scripts.test:3b"}}
```

## Agent Integration

Hashline works with any AI coding agent that accepts system-prompt instructions: Claude Code, Cursor, Windsurf, and others.

### Claude Code (recommended)

**1. Install the skill** (one-time, global — available in all your projects):

```sh
mkdir -p ~/.claude/skills/hashline-setup
curl -fsSL https://raw.githubusercontent.com/lispmeister/hashline/main/.claude/skills/hashline-setup/SKILL.md \
    -o ~/.claude/skills/hashline-setup/SKILL.md
```

**2. Run it in any project:**

```
/hashline-setup
```

The skill installs the hook scripts, registers them in `.claude/settings.local.json`, and runs the test suite to verify. See [`HASHLINE_HOOKS.md`](HASHLINE_HOOKS.md) for what the hooks do and how to install them manually.


See also [`HASHLINE_TEMPLATE.md`](HASHLINE_TEMPLATE.md) for instructions for other agents.

### Other agents (Cursor, Windsurf, etc.)

1. Install the `hashline` binary
2. Paste the instructions from [`HASHLINE_TEMPLATE.md`](HASHLINE_TEMPLATE.md) (below the `---`) at the **top** of your project's `AGENTS.md` or equivalent rules file — before any other content. Agents weight earlier instructions more heavily; placing these first ensures `hashline` takes precedence over default edit tools.

The template covers the full workflow: reading files, applying edits (heredoc or `--input` file), batching multiple edits, recovering from hash mismatches, using `--emit-updated` to reduce round-trips, and when to use `replace` vs anchor ops.

## Usage Logging

Hashline appends a one-line CSV record to `~/.local/state/hashline/usage.log` on macOS/Linux (or `%APPDATA%\hashline\usage.log` on Windows) after each command. Set `HASHLINE_USAGE_LOG` to override the location, or export `HASHLINE_DISABLE_USAGE_LOG=1` to skip logging entirely.


## Why Not Diffs or String Replacement?

| Approach | Failure mode |
|----------|-------------|
| **Patch/diff** | Strict formatting rules; 50%+ failure rate for weaker models |
| **String replacement** | Requires character-perfect reproduction including whitespace |
| **Neural merge** | Requires fine-tuning a separate 70B model |
| **Hashline** | Model references anchors; heuristics recover from output artifacts |

The key insight from Can Bölük's original research: models don't fail because they can't reason about code — they fail because the *edit harness* is too brittle. Hashline makes the harness robust.

## Testing

```sh
# Run all tests (unit + integration + fuzz + comparison fixtures) - 145 tests total
cargo test

# Run only the LLM comparison fixtures (hashline vs raw search-replace)
cargo test --test comparison -- --nocapture

# Run performance benchmarks (100 / 1K / 10K line files)
cargo run --release --bin bench

# Run Claude Code hook tests (bash, requires jq)
bash .claude/hooks/tests/test_hooks.sh
```

The comparison suite applies each of 10 fixture scenarios two ways — hashline anchors vs naive string replacement — and prints a pass/fail table showing where hashline succeeds and raw mode fails.

## License

MIT
