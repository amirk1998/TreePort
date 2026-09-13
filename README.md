# Treeport 🌳

**A fast, zero-dependency directory & file intelligence CLI written in Rust, with first-class Unicode and right-to-left (RTL) support.**

Treeport recursively scans a directory and produces a useful tree view, statistical summary, detailed inventory, duplicate analysis, and exportable reports. The project is implemented as a modular Rust application rather than a single source file, while keeping the runtime dependency-free and cross-platform.

## Features

- 🌲 **Tree view** — colored, icon-annotated hierarchy with per-directory totals and item counts.
- 📊 **Summary analysis** — totals, categories, extensions, largest files/directories, oldest/newest files, empty entries, hidden entries, symlinks, errors, and maximum depth.
- 🔍 **Duplicate detection** — groups candidate duplicate files by exact size and streamed content hashing.
- 🎛️ **Rich filtering** — exact names, glob patterns, extensions, modification dates, minimum display size, maximum depth, and `.treeportignore`.
- 📤 **Multiple outputs** — terminal tree/table plus Markdown, JSON, CSV, and file-only inventory CSV.
- 🌍 **Unicode / RTL support** — designed for Persian, Arabic, Hebrew, and other UTF-8 filenames.
- 🪟 **Windows-aware** — UTF-8 console setup and ANSI color support without external crates.
- ⚡ **Single filesystem traversal** — the scanner builds the tree, flat entry inventory, and core statistics together.
- 📦 **Zero third-party dependencies** — Rust standard library only.

## Architecture

The refactor separates the application into focused modules:

```text
src/
├── main.rs                 # Thin process entry point
├── lib.rs                  # Application orchestration / library entry
├── cli.rs                  # CLI parsing and runtime configuration
├── model.rs                # Core domain types
├── scanner.rs              # Filesystem traversal
├── analysis.rs             # Derived statistics and analysis
├── duplicate.rs            # Duplicate-file detection
├── output.rs               # Output coordination
├── util.rs                 # Shared parsing / formatting helpers
├── platform/
│   ├── mod.rs              # Platform abstraction
│   └── windows.rs          # Windows console support
└── report/
    ├── mod.rs
    ├── tree.rs             # Tree renderer
    ├── summary.rs          # Summary renderer
    ├── detailed.rs         # Detailed table renderer
    ├── markdown.rs         # Markdown exporter
    ├── json.rs             # JSON exporter
    ├── csv.rs              # CSV exporter
    └── file_list.rs        # File inventory exporter
```

The intended dependency direction is:

```text
CLI → Scanner → ScanResult
                  ├── Analysis
                  ├── Duplicate detection
                  └── Reports / Output
```

Rendering and export code consumes scan results; it does not perform filesystem traversal.

## Requirements

- Rust **1.85+**
- No third-party runtime dependencies
- Windows, Linux, and macOS

The package uses Rust 2024 edition. The `rust-version` field is the supported MSRV for this release.

## Installation

### Build from source

```bash
git clone https://github.com/amirk1998/treeport
cd treeport
cargo build --release
```

Binary:

```text
target/release/treeport
```

Windows:

```text
target\\release\\treeport.exe
```

### Development checks

```bash
cargo fmt -- --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release
```

## Quick start

```bash
# Scan the current directory
treeport .

# Scan a project and skip common noise directories
treeport ~/projects/my-app -e .git -e node_modules -e target

# Sort the tree by size and show the largest 20 items in the summary
treeport . --sort size --top 20

# Print only the summary
treeport . --max-depth 2 --quiet

# Find duplicate files
treeport . --find-duplicates

# Export reports in several formats
treeport . --markdown report.md --json report.json --csv report.csv

# Export a file-only inventory
treeport . --file-list files.csv

# Inventory only Markdown files
treeport . --include-ext md --file-list docs.csv
```

## Full CLI reference

```text
USAGE:
    treeport <path> [options]

OUTPUT / EXPORT:
    -o, --output <file>       Write the full detailed table report to <file>
    -m, --markdown <file>     Export a formatted Markdown report to <file>
    -j, --json <file>         Export a structured JSON report to <file>
    -c, --csv <file>          Export a flat CSV of every entry to <file>
    --file-list <file>        Export a numbered CSV inventory of files only
    -f, --full                Also print the full detailed per-entry table
    -q, --quiet               Skip the tree, print only the summary

DISPLAY:
    --no-color                Disable colored output
    --color                   Force colored output
    --ascii                   Use plain ASCII tree characters
    --utf8-bom                Prefix supported text/Markdown exports with UTF-8 BOM
    --no-progress             Disable the live scan-progress status line

FILTERING:
    -d, --max-depth <N>       Limit recursion depth
    -e, --exclude <name>      Skip an exact entry name (repeatable)
    --exclude-glob <pattern>  Skip entries matching * / ? (repeatable)
    --include-ext <ext>       Include only files with this extension (repeatable)
    --exclude-ext <ext>       Exclude files with this extension (repeatable)
    --newer-than <DATE>       Include files modified on/after YYYY-MM-DD
    --older-than <DATE>       Include files modified on/before YYYY-MM-DD
    --min-size <SIZE>         Hide files smaller than SIZE in the tree view
    --ignore-file <file>      Use a specific ignore file
    --no-ignore-file          Disable automatic .treeportignore loading

ANALYSIS:
    -t, --top <N>             Number of largest files/directories in the summary
    -s, --sort <name|size>    Tree sort order
    --find-duplicates         Detect duplicate files by content

MISC:
    -h, --help                Show help
```

Run `treeport --help` for the authoritative runtime help.

## Unicode and RTL support

Treeport treats filenames as real UTF-8 data throughout the scan pipeline. RTL names are isolated when embedded into LTR terminal/report structures so that bidirectional reordering does not corrupt tree branches, columns, or punctuation.

Example:

```bash
treeport "پروژه من"  --markdown "گزارش.md" --utf8-bom
```

On Windows, Treeport switches the console input/output code page to UTF-8 and enables ANSI processing when supported.

Directional isolates are a **display concern** only. CSV/file-list exports keep the underlying filename as raw UTF-8 rather than storing display control characters around it. JSON is emitted without a BOM.

## `.treeportignore`

Place a `.treeportignore` file in the directory being scanned:

```gitignore
# comments start with '#'
node_modules
*.log
target
.DS_Store
```

Patterns are matched against **entry names**, not full relative paths. This is intentionally a small `.gitignore`-style feature rather than a full Git ignore implementation.

Disable automatic loading:

```bash
treeport . --no-ignore-file
```

Use a different ignore file:

```bash
treeport . --ignore-file path/to/my-ignore-file
```

## Export formats

| Format             | Flag                          | Best for                                    |
| ------------------ | ----------------------------- | ------------------------------------------- |
| Terminal tree      | default                       | Interactive directory overview              |
| Detailed table     | `-o, --output` / `-f, --full` | Complete per-entry inventory                |
| Markdown           | `-m, --markdown`              | Documentation, GitHub, archival reports     |
| JSON               | `-j, --json`                  | Scripts, dashboards, CI, further processing |
| CSV                | `-c, --csv`                   | Spreadsheet-friendly full inventory         |
| File inventory CSV | `--file-list`                 | Numbered file-only inventory                |

CSV and file-inventory exports are BOM-prefixed for Windows/Excel compatibility. JSON is not BOM-prefixed.

### File inventory

```bash
treeport . --file-list files.csv
treeport . --include-ext rs --file-list rust-files.csv
```

The inventory contains:

```text
File Number,File Name,Relative Path
```

Rows are sorted by file name. Filenames are stored as raw UTF-8.

## Filesystem safety

Treeport uses `symlink_metadata` for directory entries, so symlinks are detected without following them. This prevents symlink cycles from causing recursive traversal.

Child filesystem errors are recorded where possible instead of automatically aborting the entire scan.

## Duplicate detection

Duplicate detection first groups files by exact size and hashes only groups containing multiple files. File contents are streamed in chunks rather than loaded entirely into memory.

The current implementation deliberately uses a dependency-free standard-library hasher. Duplicate groups should therefore be treated as **high-confidence matches**, not as a cryptographic proof of byte identity.

## Testing

For a practical integration test, create a fixture directory containing nested directories, empty files/directories, several extensions, duplicate files, hidden files, Unicode/RTL names, ignored entries, and a symbolic link where supported.

Then exercise:

```bash
treeport ./test-fixture
treeport ./test-fixture --sort size --top 20
treeport ./test-fixture --find-duplicates
treeport ./test-fixture --include-ext rs
ntreeport ./test-fixture --exclude-glob "*.log"
treeport ./test-fixture --max-depth 1
ntreeport ./test-fixture --markdown report.md --json report.json --csv report.csv
```

## Versioning

Treeport follows [Semantic Versioning](https://semver.org/) and maintains release notes in [CHANGELOG.md](CHANGELOG.md).

Release checklist:

1. Update `version` in `Cargo.toml`.
2. Move the appropriate `Unreleased` entries into a dated release section.
3. Run formatting, tests, Clippy, and a release build.
4. Commit the release changes.
5. Create and push the matching Git tag.

Example:

```bash
git tag -a v0.2.0 -m "treeport v0.2.0"
git push origin main --tags
```

## Migrating from the single-file layout

The application was refactored from a monolithic `main.rs` into focused modules. The CLI is intended to remain the public interface; the internal module layout should be treated as implementation detail.

The refactor does **not** imply a change in the core scanning model: symlinks remain un-followed, filters still operate during scanning, and report writers consume the collected scan result.

## Contributing

Issues and pull requests are welcome.

Before submitting changes:

```bash
cargo fmt
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

Please keep the core project dependency-free unless there is a strong technical reason to introduce a dependency.

## License

MIT. See [LICENSE](LICENSE).

## Changelog

See [CHANGELOG.md](CHANGELOG.md).
