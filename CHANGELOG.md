# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),

## [0.1.6] - 2026-02-23

### Changed

- `HASHLINE_TEMPLATE.md`: placement guidance added (paste at top of CLAUDE.md/AGENTS.md before any other content); `insert_after` example anchors to a non-blank line; heredoc direction fixed (`hashline apply << 'EOF'`); Rules explicitly name `hashline read` and `hashline apply`
- `README.md`: same heredoc fix in the usage example; step 2 of Agent Integration updated with top-of-file placement guidance
- `CLAUDE.md`: synced with updated template; `## Keeping the binary current` moved to end of file so editing instructions appear first

## [0.1.5] - 2026-02-18

### Fixed

- Benchmark CI threshold raised from 15% to 25% to tolerate shared runner variance
- `compute_line_hash` benchmark iterations increased 50 → 200 to reduce measurement noise

## [0.1.4] - 2026-02-18

### Added

- Performance regression test rig: `bench --json` flag, `benchmarks/compare.py`, per-release result files, and `bench-release.yml` workflow
- `BENCHMARKS.md`: Rust vs Bun throughput comparison, hash parity table, 100-edit scenario
- Rust toolchain pinned to 1.93.0 via `rust-toolchain.toml`

### Fixed

- Clippy `manual_is_multiple_of` lint in bench binary
- Added `clippy` and `rustfmt` components to `rust-toolchain.toml` for CI compatibility
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.3] - 2026-02-18

### Added

- Per-subcommand man pages (`hashline-read.1`, `hashline-apply.1`, `hashline-hash.1`) bundled in release tarballs
- Man pages include AGENT WORKFLOW, EDIT OPERATIONS reference, and per-subcommand EXAMPLES
- Homebrew install now includes all man pages
- README: Homebrew upgrade instructions

### Fixed

- `cargo install --path .` reminder added to CLAUDE.md to prevent stale binary issues

## [0.1.2] - 2026-02-18

### Added

- `hashline read --lines N` flag to limit output to N lines (partial reads)
- `replace` edit operation: exact substring replacement, runs after anchor edits
  - Errors on ambiguous match (multiple occurrences) and not-found
  - `apply_replace_edits()` is now a public library function
- Property-based fuzz tests via `proptest` (12 tests covering hash, parse, format, apply)
- Hash compatibility test suite verifying byte-for-byte output parity with `Bun.hash.xxHash32`
- 3 additional heuristic integration tests (hashline prefix stripping, diff-plus stripping, indent restoration)

### Fixed

- `--start-line` was only changing line number labels, not actually selecting lines from the file

### Changed

- `--start-line` and `--lines` now reject values of 0 or above `u32::MAX` (4,294,967,295)

## [0.1.1] - 2026-02-17

### Added

- CLI now supports `--help` and `--version` flags via clap
- Man page generation via `clap_mangen` (included in release tarballs)
- CI workflow for automated testing (build, test, clippy, fmt)

### Changed

- Install script now installs man page to `~/.local/share/man/man1/`

## [0.1.0] - 2026-02-17

### Added

- CLI with `read`, `apply`, and `hash` subcommands
- Edit operations: `set_line`, `replace_lines`, `insert_after`
- Content-addressable line hashing using xxHash32 (2-char hex)
- Hash mismatch detection for stale file edits
- Line relocation via unique hash lookup when line numbers shift
- Heuristics: indentation restoration, boundary echo stripping, wrapped line restoration, confusable hyphen normalization
- 88 tests: 24 unit, 53 integration, 11 LLM comparison fixtures
- Performance benchmarks (`cargo run --release --bin bench`)
- Cross-platform release binaries (macOS, Linux, Windows) via GitHub Actions
- Install script with SHA256 checksum verification

[0.1.2]: https://github.com/lispmeister/hashline/releases/tag/v0.1.2
[0.1.1]: https://github.com/lispmeister/hashline/releases/tag/v0.1.1
[0.1.0]: https://github.com/lispmeister/hashline/releases/tag/v0.1.0
