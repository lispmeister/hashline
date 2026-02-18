# Hashline

[![CI](https://github.com/lispmeister/hashline/actions/workflows/ci.yml/badge.svg)](https://github.com/lispmeister/hashline/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/lispmeister/hashline)](https://github.com/lispmeister/hashline/releases/latest)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![Changelog](https://img.shields.io/badge/changelog-Keep%20a%20Changelog-orange)](CHANGELOG.md)

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

Detects your platform, downloads the binary, and verifies the SHA256 checksum:

```sh
curl -fsSL https://raw.githubusercontent.com/lispmeister/hashline/main/install.sh | sh
```

Options:

```sh
# Custom install directory (default: ~/.local/bin)
curl -fsSL https://raw.githubusercontent.com/lispmeister/hashline/main/install.sh | sh -s -- --prefix /usr/local/bin

# Specific version
curl -fsSL https://raw.githubusercontent.com/lispmeister/hashline/main/install.sh | sh -s -- --version v0.1.0
```

Pre-built binaries are available for:
- macOS (Apple Silicon, Intel)
- Linux (x86_64, ARM64)
- Windows (x86_64)

Or download directly from [Releases](https://github.com/lispmeister/hashline/releases).

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

Read a specific range (useful for verifying edits without re-reading the whole file):

```sh
hashline read --start-line 3 --lines 2 src/main.rs
```

Output:
```
3:7f|fn main() {
4:01|    println!("hello");
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

## Agent Integration

Hashline works with any AI coding agent that accepts system-prompt instructions (Claude Code, Cursor, Windsurf, etc.).

To enable hashline in your project:

1. Install the `hashline` binary (see [Install](#install) above)
2. Open [`HASHLINE_TEMPLATE.md`](HASHLINE_TEMPLATE.md) and copy the section below the `---` line
3. Paste it into your project's `CLAUDE.md`, `AGENTS.md`, or equivalent agent instructions file

The template covers the full workflow: reading files, applying edits with heredoc syntax, batching multiple edits, and recovering from hash mismatches.

## Testing

```sh
# Run all tests (unit + integration + comparison fixtures)
cargo test

# Run only the LLM comparison fixtures (hashline vs raw search-replace)
cargo test --test comparison

# Run comparison fixtures with the summary table printed
cargo test --test comparison -- --nocapture

# Run a single fixture by name
cargo test --test comparison fixture_04_indentation_sensitive

# Run performance benchmarks (100 / 1K / 10K line files)
cargo run --release --bin bench
```

The comparison test suite loads 10 fixture scenarios from `tests/fixtures/` and applies each edit two ways: via hashline anchors and via naive string replacement. It prints a pass/fail summary table showing where hashline succeeds and raw mode fails (ambiguity, indentation, etc.).

## License

MIT
