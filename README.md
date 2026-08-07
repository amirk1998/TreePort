# Treeport 🌳

**A fast, zero-dependency directory & file intelligence tool for the command line — with full Unicode / right-to-left script support (Persian, Arabic, Hebrew, …) built in.**

Treeport recursively walks a directory and gives you a clean tree view, a full statistical summary, and reports you can export as Markdown, JSON, or CSV — all from a single, dependency-free Rust binary.

---

## Table of contents

- [Features](#features)
- [Install](#install)
- [Quick start](#quick-start)
- [Full CLI reference](#full-cli-reference)
- [Persian / right-to-left support](#persian--right-to-left-support)
- [Ignoring files — `.treeportignore`](#ignoring-files--treeportignore)
- [Export formats](#export-formats)
- [Building from source](#building-from-source)
- [Versioning](#versioning)
- [Changelog](#changelog)
- [Contributing](#contributing)
- [License](#license)

## Features

- 🌲 **Tree view** — colored, icon-annotated directory tree with per-folder totals and item counts (like `tree`, but genuinely useful).
- 📊 **Summary report** — totals, breakdown by category and extension, largest files _and_ largest directories, oldest/newest files, empty files/dirs, hidden entries, symlinks, and max depth.
- 🌍 **Real Unicode / RTL support** — Persian, Arabic, Hebrew, and any other script are scanned, sorted, filtered, and exported correctly, and rendered without corrupting the surrounding tree/table layout. See [below](#persian--right-to-left-support).
- 📤 **Three export formats** — Markdown (for humans, GitHub, docs), JSON (for scripts, dashboards, CI), CSV (for a spreadsheet).
- 🧬 **Duplicate detection** — finds byte-identical files by content, grouped and sized by wasted space.
- 🎛️ **Rich filtering** — exclude by exact name or glob, include/exclude by extension, filter by modification date, hide small files from the tree, limit recursion depth.
- 📄 **`.treeportignore` support** — a simple, `.gitignore`-style ignore file, auto-loaded from the scan root.
- ⏱️ **Live progress** — a throttled status line on stderr for big trees, never touching the actual report output.
- 🖥️ **Genuinely cross-platform** — Linux, macOS, and Windows, with correct color and Unicode console handling on all three.
- 📦 **Zero dependencies** — pure Rust std. No supply chain to audit, no `cargo build` waiting on the network.

## Install

### Download a prebuilt binary

Grab the latest binary for your OS from the [Releases page](https://github.com/<your-username>/treeport/releases) — no Rust toolchain required. Available for:

| OS      | Architecture            | Asset                           |
| ------- | ----------------------- | ------------------------------- |
| Linux   | x86_64                  | `treeport-linux-x86_64.tar.gz`  |
| Linux   | aarch64                 | `treeport-linux-aarch64.tar.gz` |
| macOS   | x86_64 (Intel)          | `treeport-macos-x86_64.tar.gz`  |
| macOS   | aarch64 (Apple Silicon) | `treeport-macos-aarch64.tar.gz` |
| Windows | x86_64                  | `treeport-windows-x86_64.zip`   |

Unpack it and put the `treeport` (or `treeport.exe`) binary somewhere on your `PATH`.

### Install with Cargo

```bash
cargo install --git https://github.com/<your-username>/treeport
```

### Build from source

See [Building from source](#building-from-source).

## Quick start

```bash
# Scan the current directory
treeport .

# Scan a specific path, skip common noise directories
treeport ~/projects/my-app -e .git -e node_modules -e target

# Sort by size, show the top 20 largest files, export a Markdown report
treeport . -s size -t 20 -m report.md

# Find duplicate files and export everything as JSON for a script to consume
treeport . --find-duplicates -j report.json

# Quick, shallow, quiet overview (summary only, no tree)
treeport . --max-depth 2 -q
```

## Full CLI reference

```text
USAGE:
    treeport <path> [options]

OUTPUT / EXPORT:
    -o, --output <file>     Write the full detailed table report to <file>
    -m, --markdown <file>   Export a clean, formatted Markdown report to <file>
    -j, --json <file>       Export a structured JSON report to <file>
                             (stats, categories, extensions, every entry —
                             great for scripts, dashboards, CI checks)
    -c, --csv <file>        Export a flat CSV of every entry to <file>
                             (path, kind, size, depth, modified, hidden —
                             opens straight into Excel/Sheets)
    -f, --full              Also print the full detailed per-entry table
    -q, --quiet             Skip the tree, print only the summary
    --no-color              Disable colored output
    --color                 Force colored output even when piped to a file
    --ascii                 Plain ASCII tree characters, no emoji/unicode
                             (use this if the tree looks broken/garbled)
    --utf8-bom              Prefix exported .txt/.md files with a UTF-8 BOM.
                             Fixes Persian/Arabic/Hebrew/any non-Latin text
                             showing as mojibake in older Windows apps.
                             CSV exports always include the BOM (Excel needs
                             it); JSON exports never do (the spec forbids it).
    --no-progress           Disable the live "scanning… N entries" status line

FILTERING:
    -d, --max-depth <N>     Limit recursion depth (default: unlimited)
    -e, --exclude <name>    Skip any dir/file with this exact name (repeatable)
    --exclude-glob <pat>    Skip entries matching a `*`/`?` wildcard (repeatable)
    --include-ext <ext>     Only include files with this extension (repeatable)
    --exclude-ext <ext>     Skip files with this extension (repeatable)
    --newer-than <DATE>     Only include files modified on/after DATE (YYYY-MM-DD)
    --older-than <DATE>     Only include files modified on/before DATE (YYYY-MM-DD)
    --min-size <SIZE>       Hide files smaller than SIZE in the tree view only
    --ignore-file <file>    Use this ignore file instead of <path>/.treeportignore
    --no-ignore-file        Don't auto-load .treeportignore from the scan root

ANALYSIS:
    -t, --top <N>           "Largest files/directories" count (default: 10)
    -s, --sort <name|size>  Tree sort order (default: name)
    --find-duplicates       Detect duplicate files by content and report them

    -h, --help              Show this message
```

Run `treeport --help` any time for the same reference with extra examples.

## Persian / right-to-left support

Treeport was built with real right-to-left support, not just "doesn't crash on Unicode":

- **Correct data, always.** File and folder names are handled as real UTF-8 throughout — scanning, sorting, filtering, and exporting never mangle non-Latin text.
- **Correct console rendering on Windows.** `cmd.exe` and PowerShell default to a legacy code page that can't display Persian/Arabic at all (you'd see `?` or boxes). Treeport switches the console to UTF-8 automatically on startup.
- **No bidi corruption.** Mixing a right-to-left name like `گزارش.md` into an otherwise left-to-right line (a tree branch, a table column) can make a terminal's bidi algorithm visually scramble the surrounding structure. Treeport wraps RTL names in Unicode directional isolates so the structure stays intact and the name still renders correctly, right-to-left, inside it.
- **Exports that open correctly everywhere.** `--utf8-bom` prefixes Markdown/text exports with a UTF-8 BOM, so older or misconfigured Windows apps (classic Notepad, Excel's plain "open") detect the encoding correctly instead of guessing the system code page and mangling the text. CSV exports always include the BOM, since Excel effectively requires it.

```bash
treeport "پروژه من" -m "گزارش.md" --utf8-bom
```

## Ignoring files — `.treeportignore`

Drop a `.treeportignore` file at the root you're scanning to exclude files/folders without typing `-e`/`--exclude-glob` every time:

```gitignore
# comments start with '#'
node_modules
*.log
target
.DS_Store
```

One glob pattern per line, matched against entry _names_ (not full paths) — a simplified `.gitignore`, not a full implementation. Disable it with `--no-ignore-file`, or point to a different file with `--ignore-file <path>`.

## Export formats

| Format           | Flag                    | Best for                                                     |
| ---------------- | ----------------------- | ------------------------------------------------------------ |
| Markdown         | `-m, --markdown <file>` | Committing to a repo, viewing on GitHub, sharing with humans |
| JSON             | `-j, --json <file>`     | Scripts, dashboards, CI checks, further processing           |
| CSV              | `-c, --csv <file>`      | Opening straight in Excel/Google Sheets                      |
| Plain text table | `-o, --output <file>`   | A complete per-entry table, piped or archived                |

All three (Markdown/JSON/CSV) can be requested in the same run.

## Building from source

Requires only the Rust toolchain (no other dependencies to fetch):

```bash
git clone https://github.com/<your-username>/treeport
cd treeport
cargo build --release
# binary at target/release/treeport (or target\release\treeport.exe on Windows)
```

Since Treeport has zero external dependencies, you can also build it as a single file with just `rustc`, no Cargo required:

```bash
rustc -O src/main.rs -o treeport
```

## Versioning

Treeport follows [Semantic Versioning](https://semver.org/) (`MAJOR.MINOR.PATCH`). See [How to set a version](#how-to-set-a-version) below for the release process, and [CHANGELOG.md](CHANGELOG.md) for the history of every release.

### How to set a version

1. Edit the `version` field in `Cargo.toml`:
   ```toml
   [package]
   name = "treeport"
   version = "0.2.0"
   ```
2. Move the `## [Unreleased]` entries in `CHANGELOG.md` into a new dated section for that version (see [Changelog](#changelog)).
3. Commit both:
   ```bash
   git add Cargo.toml CHANGELOG.md
   git commit -m "chore(release): v0.2.0"
   ```
4. Tag the commit and push the tag — this is what triggers the release build:
   ```bash
   git tag -a v0.2.0 -m "treeport v0.2.0"
   git push origin main --tags
   ```

Pushing a tag matching `v*.*.*` triggers [`.github/workflows/release.yml`](.github/workflows/release.yml), which cross-compiles Treeport for Linux, macOS, and Windows and attaches the binaries to a new GitHub Release automatically. See [Release automation](#release-automation) below.

When to bump which number:

| Change                                                          | Bump                                                |
| --------------------------------------------------------------- | --------------------------------------------------- |
| Bug fix, no CLI/output changes                                  | **PATCH** (`0.1.0` → `0.1.1`)                       |
| New flag/feature, backward-compatible                           | **MINOR** (`0.1.0` → `0.2.0`)                       |
| Removed/renamed a flag, changed output format in a breaking way | **MAJOR** (`0.x.y` → `1.0.0`, or `1.x.y` → `2.0.0`) |

While the project is pre-1.0 (`0.x.y`), the CLI/output format may still change between minor versions — this is standard SemVer practice for young projects. Move to `1.0.0` once you're happy calling the CLI interface stable.

## Changelog

All notable changes are recorded in [CHANGELOG.md](CHANGELOG.md), following the [Keep a Changelog](https://keepachangelog.com/) format.

## Release automation

Two workflows live under `.github/workflows/`:

- **`ci.yml`** — runs on every push/PR: builds and tests on Linux, macOS, and Windows, so a break on any platform is caught before it ships.
- **`release.yml`** — runs when you push a tag matching `v*.*.*`: cross-compiles release binaries for Linux (x86_64, aarch64), macOS (x86_64, aarch64), and Windows (x86_64), packages each as a `.tar.gz`/`.zip`, and publishes them to a new GitHub Release for that tag.

To cut a release, you generally don't need to touch these files — just follow [How to set a version](#how-to-set-a-version) above.

## Migrating from dirscan

Treeport is a rename/continuation of the `dirscan` project — the code, flags, and output are unchanged. If you have scripts calling `dirscan`, either:

- rename the binary back to `dirscan` after building (`mv target/release/treeport dirscan`), or
- keep both names by adding a second `[[bin]]` entry in `Cargo.toml` that points at the same `src/main.rs`.

The one behavioral difference: the default ignore-file name changed from `.dirscanignore` to `.treeportignore` (use `--ignore-file .dirscanignore` to keep an existing one working).

## Contributing

Issues and pull requests are welcome. Please run `cargo fmt` and `cargo clippy` before submitting, and keep the project dependency-free (Rust std only) unless there's a strong reason not to.

## License

<!-- Pick a license and add a LICENSE file — MIT or Apache-2.0 are common choices for CLI tools like this. -->

See [LICENSE](LICENSE).
