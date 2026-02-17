# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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

[0.1.1]: https://github.com/lispmeister/hashline/releases/tag/v0.1.1
[0.1.0]: https://github.com/lispmeister/hashline/releases/tag/v0.1.0
