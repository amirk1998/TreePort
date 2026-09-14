# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Documentation for the modular project architecture and recommended
  development/testing workflow.

### Changed

- Continued the refactor from a monolithic `main.rs` into focused modules for
  CLI configuration, filesystem scanning, analysis, duplicate detection,
  platform support, output coordination, and report generation.
- Added an explicit library crate entry point through `src/lib.rs` while keeping
  `src/main.rs` as the thin binary entry point.
- Split report generation into dedicated tree, summary, detailed-table,
  Markdown, JSON, CSV, and file-list modules.
- Isolated Windows console/FFI behavior behind the platform module.
- Updated package metadata for the modular Cargo project and Rust 2024 edition.

### Fixed

- Fixed the `--sort` parser to match the borrowed `&str` returned by the CLI
  argument helper instead of calling the unstable `str::as_str()` API.
- Removed unused imports introduced during the modular refactor.

### Notes

- The core project remains dependency-free and uses only the Rust standard
  library.
- Symlinks are detected through link metadata and are not followed during
  recursive scanning.
- Duplicate detection remains intentionally dependency-free and uses exact
  size grouping followed by streamed content hashing.

## [0.2.0] - 2026-09-14

Modular architecture release following the initial public `dirscan` →
**Treeport** transition.

### Added

- Modular source layout under `src/` with dedicated responsibilities for CLI,
  domain models, scanning, analysis, duplicate detection, platform handling,
  output coordination, and reports.
- Library crate target via `src/lib.rs`.
- Expanded project documentation covering architecture, testing, exports,
  Unicode/RTL handling, and development workflow.

### Changed

- Moved application orchestration out of the monolithic `main.rs`.
- Split terminal rendering and file exporters into separate modules.
- Updated Cargo package metadata and moved the project to Rust 2024 edition.

### Fixed

- Corrected the `--sort` parser compatibility issue with stable Rust.
- Cleaned unused imports from the refactored modules.

### Preserved behavior

- Existing CLI concepts and export formats remain available.
- `.treeportignore` behavior remains name-based rather than full path-based.
- Unicode/RTL handling remains a first-class feature.
- Symlinks are detected but never followed.

## [0.1.0] - 2026-08-07

Initial public release, renamed from `dirscan` to **Treeport**.

### Added

- Tree view of a directory with per-folder totals, item counts, and per-type icons.
- Summary report with totals, category/extension breakdowns, largest files and
  directories, oldest/newest files, empty files/directories, hidden entries,
  symlinks, and maximum depth.
- Full Unicode and right-to-left script support for Persian, Arabic, Hebrew,
  and other UTF-8 filenames.
- Windows UTF-8 console handling and ANSI color support without external crates.
- Markdown, JSON, CSV, detailed table, and file-list export modes.
- `--file-list <file>` — numbered file-only CSV inventory with file name and
  path relative to the scan root, with optional `--include-ext` narrowing.
- Duplicate file detection using size grouping and streamed content hashing.
- Exact-name, glob, extension, date, depth, and minimum-display-size filters.
- `.treeportignore` support with `--ignore-file` and `--no-ignore-file`.
- Live throttled scan progress with `--no-progress`.
- Name/size sorting, ASCII mode, color controls, quiet mode, and full-table mode.
- Cross-platform support for Linux, macOS, and Windows.
- Zero external dependencies — Rust standard library only.

[Unreleased]: https://github.com/amirk1998/treeport/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/amirk1998/treeport/releases/tag/v0.2.0
[0.1.0]: https://github.com/amirk1998/treeport/releases/tag/v0.1.0
