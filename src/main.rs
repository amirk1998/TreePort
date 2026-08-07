//! treeport — a directory & file intelligence tool
//!
//! Recursively walks a directory tree and shows you:
//!   1. A clean, colored TREE VIEW (like the classic `tree` command, but
//!      with file sizes, folder totals, item counts, and per-type icons
//!      baked in)
//!   2. A SUMMARY REPORT: totals, breakdown by category and extension,
//!      largest files, largest directories, newest/oldest files, empty
//!      files/dirs, hidden entries, symlinks, duplicate files, max depth
//!   3. (optional) A COMPLETE per-entry table report, on screen and/or
//!      written to a file
//!   4. (optional) Export to MARKDOWN, JSON, or CSV — pick the format
//!      that fits: Markdown for humans (commit it, view it on GitHub),
//!      JSON for scripts/dashboards, CSV for a spreadsheet
//!
//! Full Unicode / right-to-left script support (Persian, Arabic, Hebrew,
//! etc.):
//!   - All scanning, filtering, and export logic works on real UTF-8
//!     strings throughout — file and folder names in any script are
//!     scanned, sorted, filtered, and exported correctly.
//!   - On Windows, the console code page is switched to UTF-8 on startup,
//!     which is what makes Persian/Arabic text render instead of `?`/
//!     boxes in cmd.exe and PowerShell.
//!   - Right-to-left names are wrapped in Unicode directional isolates
//!     wherever they're inlined into an otherwise left-to-right line
//!     (tree branches, table columns, markdown) so they can't visually
//!     scramble the surrounding ASCII structure.
//!   - `--utf8-bom` prefixes exported .txt/.md files with a UTF-8 BOM,
//!     which is what makes older/misconfigured Windows apps (Notepad,
//!     Excel's plain "open") reliably detect UTF-8 instead of guessing
//!     the system code page and mangling non-Latin text. CSV exports
//!     always include the BOM since Excel effectively requires it for
//!     correct non-Latin rendering; JSON exports never do, since the
//!     JSON spec (RFC 8259) forbids a BOM and strict parsers reject it.
//!
//! Design goals:
//!   - Single file, zero external dependencies (Rust std only) — builds
//!     anywhere with just `rustc`, no Cargo.toml, no internet required.
//!   - Correct on real filesystems: symlinks are detected and NEVER
//!     followed (so a cyclic symlink can't cause infinite recursion), and
//!     a permission error on one entry is reported inline instead of
//!     aborting the whole scan.
//!   - Genuinely cross-platform: colors and UTF-8 output are enabled
//!     automatically on Windows terminals (via small, safe FFI calls into
//!     kernel32 — no external crate needed), hidden-file detection uses
//!     the real Windows "hidden" attribute instead of guessing from
//!     dotfiles.
//!
//! Build:
//!   rustc -O main.rs -o treeport          (Linux/macOS)
//!   rustc -O main.rs -o treeport.exe      (Windows)
//!
//! Run:
//!   ./treeport <path> [options]
//!
//! Options:
//!   -o, --output <file>     Write the full detailed table report to <file>
//!   -m, --markdown <file>   Export a clean, formatted Markdown report to <file>
//!   -j, --json <file>       Export a structured JSON report to <file>
//!   -c, --csv <file>        Export a flat CSV of every entry to <file>
//!   -f, --full              Also print the full detailed per-entry table
//!   -t, --top <N>           "Largest files/dirs" count in the summary (default: 10)
//!   -d, --max-depth <N>     Limit recursion depth (default: unlimited)
//!   -e, --exclude <name>    Skip any dir/file with this exact name (repeatable)
//!                           e.g. -e .git -e node_modules -e target
//!   --exclude-glob <pat>    Skip entries matching a wildcard pattern (repeatable)
//!                           e.g. --exclude-glob "*.log" --exclude-glob "tmp_*"
//!   --include-ext <ext>     Only include files with this extension (repeatable)
//!                           e.g. --include-ext rs --include-ext toml
//!   --exclude-ext <ext>     Skip files with this extension (repeatable)
//!                           e.g. --exclude-ext log --exclude-ext tmp
//!   --newer-than <DATE>     Only include files modified on/after DATE (YYYY-MM-DD)
//!   --older-than <DATE>     Only include files modified on/before DATE (YYYY-MM-DD)
//!   --ignore-file <file>    Use this file instead of <path>/.treeportignore
//!   --no-ignore-file        Don't auto-load .treeportignore from the scan root
//!   -s, --sort <name|size>  Tree sort order (default: name)
//!   --min-size <SIZE>       Hide files smaller than SIZE in the tree
//!                           e.g. --min-size 1MB, --min-size 500KB
//!   --find-duplicates       Detect duplicate files by content and report them
//!   --ascii                 Use plain ASCII characters, no emoji/unicode
//!                           (use this if the tree looks broken/garbled)
//!   --no-color              Disable colored output
//!   --color                 Force colored output even when piped to a file
//!   --utf8-bom              Prefix exported text/Markdown/JSON files with a
//!                           UTF-8 BOM (fixes Persian/Arabic/non-Latin text
//!                           showing as mojibake in older Windows apps)
//!   --no-progress           Disable the live "scanning… N entries" status line
//!   -q, --quiet             Skip the tree, print only the summary
//!   -h, --help              Show usage
//!
//! Examples:
//!   ./treeport .
//!   ./treeport C:\Projects -e .git -e node_modules -e target
//!   ./treeport . --ascii --no-color -o report.txt
//!   ./treeport . -s size -t 20
//!   ./treeport . -m report.md --find-duplicates
//!   ./treeport . --min-size 5MB
//!   ./treeport . -j report.json -c report.csv
//!   ./treeport . --include-ext rs --include-ext toml -m rust-files.md
//!   ./treeport . --exclude-glob "*.log" --exclude-glob "node_modules"
//!   ./treeport . --newer-than 2025-01-01 --older-than 2025-12-31
//!
//!   # Persian / right-to-left example — scanning a project with Persian
//!   # folder and file names, exporting a Markdown report that renders
//!   # correctly everywhere (GitHub, VS Code, and Windows Notepad alike):
//!   ./treeport "پروژه من" -m "گزارش.md" --utf8-bom

use std::collections::HashMap;
use std::env;
use std::fmt;
use std::fs::{self, DirEntry, Metadata};
use std::io::{self, IsTerminal, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
#[cfg(windows)]
use std::os::windows::fs::MetadataExt;

// =======================================================================
// Windows console setup (std-only FFI, no external crate needed)
// =======================================================================
//
// Modern Windows terminals (Windows Terminal, PowerShell 7) understand
// ANSI color codes out of the box. Older consoles (classic cmd.exe on
// Windows 10) support them too, but only after the process explicitly
// opts in via SetConsoleMode + ENABLE_VIRTUAL_TERMINAL_PROCESSING. This
// tiny module does exactly that, using nothing but raw FFI declarations
// into kernel32.dll, which ships with every Windows install — no crate
// required.
#[cfg(windows)]
mod win_console {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetStdHandle(nStdHandle: i32) -> *mut std::ffi::c_void;
        fn GetConsoleMode(hConsoleHandle: *mut std::ffi::c_void, lpMode: *mut u32) -> i32;
        fn SetConsoleMode(hConsoleHandle: *mut std::ffi::c_void, dwMode: u32) -> i32;
        fn SetConsoleOutputCP(wCodePageID: u32) -> i32;
        fn SetConsoleCP(wCodePageID: u32) -> i32;
    }

    const STD_OUTPUT_HANDLE: i32 = -11;
    const ENABLE_VIRTUAL_TERMINAL_PROCESSING: u32 = 0x0004;
    const CP_UTF8: u32 = 65001;

    /// Best-effort: if this fails for any reason we just silently keep
    /// colors off rather than crash or print garbled escape codes.
    pub fn enable_ansi_colors() {
        unsafe {
            let handle = GetStdHandle(STD_OUTPUT_HANDLE);
            if handle.is_null() {
                return;
            }
            let mut mode: u32 = 0;
            if GetConsoleMode(handle, &mut mode) == 0 {
                return;
            }
            SetConsoleMode(handle, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING);
        }
    }

    /// The classic Windows console defaults to a legacy OEM/ANSI code page
    /// (often 437 or a regional one like 1256), which cannot represent
    /// Persian/Arabic/other non-Latin scripts at all — every such
    /// character prints as `?` or a mangled box. Switching both the
    /// input and output code pages to UTF-8 (65001) is what makes
    /// `./treeport پروژه` and Persian filenames in the tree/report render
    /// correctly in `cmd.exe` and Windows PowerShell. Best-effort: on
    /// very old Windows builds without UTF-8 code page support this is a
    /// silent no-op rather than a crash.
    pub fn enable_utf8_console() {
        unsafe {
            SetConsoleOutputCP(CP_UTF8);
            SetConsoleCP(CP_UTF8);
        }
    }
}

// =======================================================================
// CLI configuration
// =======================================================================

#[derive(Clone, Copy, PartialEq, Eq)]
enum SortOrder {
    Name,
    Size,
}

#[derive(Clone)]
struct Config {
    root: PathBuf,
    output: Option<PathBuf>,
    markdown: Option<PathBuf>,
    json: Option<PathBuf>,
    csv: Option<PathBuf>,
    top_n: usize,
    max_depth: Option<usize>,
    excludes: Vec<String>,
    exclude_glob: Vec<String>,
    include_ext: Vec<String>,
    exclude_ext: Vec<String>,
    newer_than: Option<SystemTime>,
    older_than: Option<SystemTime>,
    sort: SortOrder,
    ascii: bool,
    color: bool,
    utf8_bom: bool,
    show_full_table: bool,
    quiet: bool,
    find_duplicates: bool,
    min_size: Option<u64>,
    show_progress: bool,
}

impl Config {
    fn parse(args: &[String]) -> Result<Config, String> {
        let mut root: Option<PathBuf> = None;
        let mut output: Option<PathBuf> = None;
        let mut markdown: Option<PathBuf> = None;
        let mut json: Option<PathBuf> = None;
        let mut csv: Option<PathBuf> = None;
        let mut top_n: usize = 10;
        let mut max_depth: Option<usize> = None;
        let mut excludes: Vec<String> = Vec::new();
        let mut exclude_glob: Vec<String> = Vec::new();
        let mut include_ext: Vec<String> = Vec::new();
        let mut exclude_ext: Vec<String> = Vec::new();
        let mut newer_than: Option<SystemTime> = None;
        let mut older_than: Option<SystemTime> = None;
        let mut sort = SortOrder::Name;
        let mut ascii = false;
        let mut color_flag: Option<bool> = None; // None = auto-detect
        let mut utf8_bom = false;
        let mut show_full_table = false;
        let mut quiet = false;
        let mut find_duplicates = false;
        let mut min_size: Option<u64> = None;
        let mut show_progress = true;
        let mut ignore_file: Option<PathBuf> = None;
        let mut no_ignore_file = false;

        let mut i = 0;
        while i < args.len() {
            match args[i].as_str() {
                "-h" | "--help" => {
                    print!("{USAGE}");
                    std::process::exit(0);
                }
                "-o" | "--output" => {
                    i += 1;
                    let v = args.get(i).ok_or("--output requires a file path")?;
                    output = Some(PathBuf::from(v));
                }
                "-m" | "--markdown" => {
                    i += 1;
                    let v = args.get(i).ok_or("--markdown requires a file path")?;
                    markdown = Some(PathBuf::from(v));
                }
                "--find-duplicates" => find_duplicates = true,
                "--min-size" => {
                    i += 1;
                    let v = args.get(i).ok_or("--min-size requires a size, e.g. 1MB")?;
                    min_size = Some(parse_size(v)?);
                }
                "-f" | "--full" => show_full_table = true,
                "-t" | "--top" => {
                    i += 1;
                    let v = args.get(i).ok_or("--top requires a number")?;
                    top_n = v
                        .parse::<usize>()
                        .map_err(|_| format!("invalid --top value: {v}"))?;
                }
                "-d" | "--max-depth" => {
                    i += 1;
                    let v = args.get(i).ok_or("--max-depth requires a number")?;
                    max_depth = Some(
                        v.parse::<usize>()
                            .map_err(|_| format!("invalid --max-depth value: {v}"))?,
                    );
                }
                "-e" | "--exclude" => {
                    i += 1;
                    let v = args.get(i).ok_or("--exclude requires a name")?;
                    excludes.push(v.clone());
                }
                "-s" | "--sort" => {
                    i += 1;
                    let v = args.get(i).ok_or("--sort requires 'name' or 'size'")?;
                    sort = match v.as_str() {
                        "name" => SortOrder::Name,
                        "size" => SortOrder::Size,
                        other => {
                            return Err(format!(
                                "invalid --sort value: {other} (use 'name' or 'size')"
                            ))
                        }
                    };
                }
                "--ascii" => ascii = true,
                "--no-color" => color_flag = Some(false),
                "--color" => color_flag = Some(true),
                "-q" | "--quiet" => quiet = true,
                "--utf8-bom" => utf8_bom = true,
                "--no-progress" => show_progress = false,
                "-j" | "--json" => {
                    i += 1;
                    let v = args.get(i).ok_or("--json requires a file path")?;
                    json = Some(PathBuf::from(v));
                }
                "-c" | "--csv" => {
                    i += 1;
                    let v = args.get(i).ok_or("--csv requires a file path")?;
                    csv = Some(PathBuf::from(v));
                }
                "--exclude-glob" => {
                    i += 1;
                    let v = args
                        .get(i)
                        .ok_or("--exclude-glob requires a pattern, e.g. '*.log'")?;
                    exclude_glob.push(v.clone());
                }
                "--include-ext" => {
                    i += 1;
                    let v = args
                        .get(i)
                        .ok_or("--include-ext requires an extension, e.g. rs")?;
                    include_ext.push(v.trim_start_matches('.').to_lowercase());
                }
                "--exclude-ext" => {
                    i += 1;
                    let v = args
                        .get(i)
                        .ok_or("--exclude-ext requires an extension, e.g. log")?;
                    exclude_ext.push(v.trim_start_matches('.').to_lowercase());
                }
                "--newer-than" => {
                    i += 1;
                    let v = args
                        .get(i)
                        .ok_or("--newer-than requires a date, e.g. 2025-01-01")?;
                    newer_than = Some(parse_date(v)?);
                }
                "--older-than" => {
                    i += 1;
                    let v = args
                        .get(i)
                        .ok_or("--older-than requires a date, e.g. 2025-01-01")?;
                    older_than = Some(parse_date(v)?);
                }
                "--ignore-file" => {
                    i += 1;
                    let v = args.get(i).ok_or("--ignore-file requires a file path")?;
                    ignore_file = Some(PathBuf::from(v));
                }
                "--no-ignore-file" => no_ignore_file = true,
                other => {
                    if root.is_none() && !other.starts_with('-') {
                        root = Some(PathBuf::from(other));
                    } else {
                        return Err(format!("unknown argument: {other}\n\n{USAGE}"));
                    }
                }
            }
            i += 1;
        }

        let root = root.unwrap_or_else(|| PathBuf::from("."));
        if !root.exists() {
            return Err(format!("path does not exist: {}", root.display()));
        }
        if !root.is_dir() {
            return Err(format!("not a directory: {}", root.display()));
        }

        // .treeportignore support: a simple, one-glob-per-line ignore file
        // (like a minimal .gitignore, matched against entry *names* only —
        // not full path globbing). Auto-loaded from the scan root unless
        // disabled or overridden with an explicit path.
        if !no_ignore_file {
            let path = ignore_file
                .clone()
                .unwrap_or_else(|| root.join(".treeportignore"));
            if let Ok(contents) = fs::read_to_string(&path) {
                for line in contents.lines() {
                    let line = line.trim();
                    if line.is_empty() || line.starts_with('#') {
                        continue;
                    }
                    exclude_glob.push(line.to_string());
                }
            } else if let Some(explicit) = &ignore_file {
                return Err(format!("cannot read ignore file: {}", explicit.display()));
            }
        }

        // Always switch the Windows console to the UTF-8 code page first —
        // this is what makes Persian (and any other non-Latin script)
        // render correctly in cmd.exe/PowerShell, independent of color.
        #[cfg(windows)]
        win_console::enable_utf8_console();

        // Auto-detect color support: only colorize when we're printing to
        // a real terminal, unless the user forced it on/off explicitly.
        let color = color_flag.unwrap_or_else(|| io::stdout().is_terminal());
        if color {
            #[cfg(windows)]
            win_console::enable_ansi_colors();
        }

        // Progress only ever makes sense when someone can actually see it
        // update live — never spam a redirected/piped stderr with \r lines.
        let show_progress = show_progress && io::stderr().is_terminal();

        Ok(Config {
            root,
            output,
            markdown,
            json,
            csv,
            top_n,
            max_depth,
            excludes,
            exclude_glob,
            include_ext,
            exclude_ext,
            newer_than,
            older_than,
            sort,
            ascii,
            color,
            utf8_bom,
            show_full_table,
            quiet,
            find_duplicates,
            min_size,
            show_progress,
        })
    }
}

/// Parses human-friendly size strings like "500KB", "10MB", "1.5GB", or a
/// plain number of bytes ("2048"), case-insensitive. Used by --min-size.
fn parse_size(s: &str) -> Result<u64, String> {
    let s = s.trim();
    let upper = s.to_uppercase();
    let (num_part, mult): (&str, f64) = if let Some(n) = upper.strip_suffix("GB") {
        (n, 1024.0 * 1024.0 * 1024.0)
    } else if let Some(n) = upper.strip_suffix("MB") {
        (n, 1024.0 * 1024.0)
    } else if let Some(n) = upper.strip_suffix("KB") {
        (n, 1024.0)
    } else if let Some(n) = upper.strip_suffix('B') {
        (n, 1.0)
    } else {
        (upper.as_str(), 1.0)
    };
    let num: f64 = num_part
        .trim()
        .parse()
        .map_err(|_| format!("invalid size: {s} (try e.g. 10MB, 500KB, 2048)"))?;
    if num < 0.0 {
        return Err(format!("size cannot be negative: {s}"));
    }
    Ok((num * mult).round() as u64)
}

const USAGE: &str = r#"treeport — directory & file intelligence report

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
                             showing as "????" or mojibake when opened in
                             older Windows apps (classic Notepad, Excel's
                             plain double-click-to-open). CSV exports always
                             include the BOM automatically (Excel needs it);
                             JSON exports never do (the JSON spec forbids a
                             BOM and strict parsers reject it).
    --no-progress           Disable the live "scanning… N entries" status
                             line (auto-disabled anyway when stderr isn't a
                             terminal, e.g. when redirected to a file)

FILTERING:
    -d, --max-depth <N>     Limit recursion depth (default: unlimited)
    -e, --exclude <name>    Skip any dir/file with this exact name (repeatable)
                             e.g. -e .git -e node_modules -e target
    --exclude-glob <pat>    Skip entries matching a `*`/`?` wildcard (repeatable)
                             e.g. --exclude-glob "*.log" --exclude-glob "tmp_*"
    --include-ext <ext>     Only include files with this extension (repeatable)
                             e.g. --include-ext rs --include-ext toml
    --exclude-ext <ext>     Skip files with this extension (repeatable)
                             e.g. --exclude-ext log --exclude-ext tmp
    --newer-than <DATE>     Only include files modified on/after DATE (YYYY-MM-DD)
    --older-than <DATE>     Only include files modified on/before DATE (YYYY-MM-DD)
    --min-size <SIZE>       Hide files smaller than SIZE in the tree view only
                             e.g. --min-size 1MB, --min-size 500KB
    --ignore-file <file>    Use this ignore file instead of <path>/.treeportignore
    --no-ignore-file        Don't auto-load .treeportignore from the scan root
                             (one glob pattern per line, '#' comments allowed —
                             matched against entry names, like a simple .gitignore)

ANALYSIS:
    -t, --top <N>           "Largest files/directories" count (default: 10)
    -s, --sort <name|size>  Tree sort order (default: name)
    --find-duplicates       Detect duplicate files by content and report them

    -h, --help              Show this message

EXAMPLES:
    treeport .
    treeport C:\Projects -e .git -e node_modules -e target
    treeport . --ascii --no-color -o report.txt
    treeport . -s size -t 20
    treeport . -m report.md --find-duplicates
    treeport . --min-size 5MB
    treeport . -j report.json -c report.csv
    treeport . --include-ext rs --include-ext toml -m rust-files.md
    treeport . --exclude-glob "*.log" --exclude-glob "node_modules"
    treeport . --newer-than 2025-01-01 --older-than 2025-12-31
    treeport src --max-depth 2 -q                 # quick shallow overview

    # Persian / right-to-left names — scan a project with Persian folder
    # and file names and export a Markdown report that renders correctly
    # everywhere, including older Windows apps:
    treeport "پروژه من" -m "گزارش.md" --utf8-bom

    # .treeportignore example (place at the root you're scanning):
    #   # comments start with '#'
    #   node_modules
    #   *.log
    #   target
"#;

// =======================================================================
// Data model
// =======================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    File,
    Dir,
    Symlink,
    Other,
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Kind::File => "FILE",
            Kind::Dir => "DIR ",
            Kind::Symlink => "LINK",
            Kind::Other => "OTHR",
        };
        write!(f, "{s}")
    }
}

/// A flat record of one filesystem entry, used for the detailed table
/// report and for the statistics pass.
#[derive(Debug, Clone)]
struct Entry {
    path: PathBuf,
    kind: Kind,
    size: u64,
    depth: usize,
    modified: Option<SystemTime>,
    hidden: bool,
    #[cfg(unix)]
    mode: Option<u32>,
}

/// A node in the nested tree structure used for the pretty tree view.
/// Directories carry the recursive total size and item count of
/// everything beneath them, which is what makes the tree genuinely
/// useful (you can see at a glance which folder is huge).
struct TreeNode {
    name: String,
    kind: Kind,
    own_size: u64,             // for files: their size. For dirs: 0 (see total_size).
    total_size: u64, // for dirs: recursive size of all files inside. For files: same as own_size.
    item_count: usize, // for dirs: total number of descendant entries. For files: 0.
    truncated: bool, // true if this dir had children we didn't recurse into (max-depth reached)
    hidden_by_min_size: usize, // direct child files hidden from display by --min-size
    children: Vec<TreeNode>,
}

/// Aggregate statistics accumulated while walking the tree.
struct Stats {
    file_count: u64,
    dir_count: u64,
    symlink_count: u64,
    other_count: u64,
    total_size: u64,
    hidden_count: u64,
    empty_file_count: u64,
    empty_dir_count: u64,
    excluded_count: u64,
    max_depth_seen: usize,
    by_extension: HashMap<String, (u64, u64)>, // ext -> (count, total size)
    errors: Vec<String>,
}

impl Stats {
    fn new() -> Self {
        Stats {
            file_count: 0,
            dir_count: 0,
            symlink_count: 0,
            other_count: 0,
            total_size: 0,
            hidden_count: 0,
            empty_file_count: 0,
            empty_dir_count: 0,
            excluded_count: 0,
            max_depth_seen: 0,
            by_extension: HashMap::new(),
            errors: Vec::new(),
        }
    }
}

// =======================================================================
// Scan progress (stderr only, never touches stdout output)
// =======================================================================
//
// Big trees (node_modules, a full monorepo, a media library) can take a
// few seconds to walk. Without any feedback the tool looks hung. This
// prints a throttled, self-erasing status line to STDERR — stdout (the
// actual tree/report output) is completely untouched, so redirecting
// `> report.txt` still gives a clean file.
struct Progress {
    enabled: bool,
    count: u64,
    last_print: Instant,
}

impl Progress {
    fn new(enabled: bool) -> Self {
        Progress {
            enabled,
            count: 0,
            last_print: Instant::now(),
        }
    }

    fn tick(&mut self) {
        if !self.enabled {
            return;
        }
        self.count += 1;
        // Throttle to ~10 updates/sec so we don't spend more time
        // printing than scanning on very fast, very large trees.
        if self.count % 64 == 0 && self.last_print.elapsed() >= Duration::from_millis(100) {
            eprint!("\rscanning… {} entries", self.count);
            let _ = io::stderr().flush();
            self.last_print = Instant::now();
        }
    }

    fn finish(&self) {
        if self.enabled && self.count > 0 {
            eprint!("\r{}\r", " ".repeat(24));
            let _ = io::stderr().flush();
        }
    }
}

// =======================================================================
// Simple glob matching (`*` and `?` only — used by --exclude-glob and
// .treeportignore, not a full gitignore implementation)
// =======================================================================

fn glob_match(pattern: &str, text: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let t: Vec<char> = text.chars().collect();
    let (mut pi, mut ti) = (0usize, 0usize);
    let mut star: Option<usize> = None;
    let mut match_from = 0usize;

    while ti < t.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == t[ti]) {
            pi += 1;
            ti += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some(pi);
            match_from = ti;
            pi += 1;
        } else if let Some(si) = star {
            pi = si + 1;
            match_from += 1;
            ti = match_from;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == '*' {
        pi += 1;
    }
    pi == p.len()
}

/// Parses a "YYYY-MM-DD" date (UTC, midnight) into a `SystemTime`, for
/// --newer-than/--older-than.
fn parse_date(s: &str) -> Result<SystemTime, String> {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 3 {
        return Err(format!("invalid date: {s} (expected YYYY-MM-DD)"));
    }
    let y: i64 = parts[0]
        .parse()
        .map_err(|_| format!("invalid date: {s} (expected YYYY-MM-DD)"))?;
    let m: u32 = parts[1]
        .parse()
        .map_err(|_| format!("invalid date: {s} (expected YYYY-MM-DD)"))?;
    let d: u32 = parts[2]
        .parse()
        .map_err(|_| format!("invalid date: {s} (expected YYYY-MM-DD)"))?;
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return Err(format!("invalid date: {s} (expected YYYY-MM-DD)"));
    }
    let days = days_from_civil(y, m, d);
    let secs = days * 86_400;
    if secs < 0 {
        return Err(format!("date before the Unix epoch is not supported: {s}"));
    }
    Ok(UNIX_EPOCH + Duration::from_secs(secs as u64))
}

/// Inverse of `civil_from_days` — same Howard Hinnant algorithm, converting
/// a (year, month, day) civil date back into days since 1970-01-01.
/// https://howardhinnant.github.io/date_algorithms.html
fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u64;
    let mp = if m > 2 { m - 3 } else { m + 9 } as u64;
    let doy = (153 * mp + 2) / 5 + d as u64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe as i64 - 719_468
}

// =======================================================================
// Hidden-file detection (accurate per-platform)
// =======================================================================

/// On Windows, "hidden" is a real file attribute bit, not just a naming
/// convention — so we check FILE_ATTRIBUTE_HIDDEN (0x2) via std's own
/// `MetadataExt`, no external crate needed. We still also treat dotfiles
/// as hidden on Windows since many dev tools (.git, .env) follow that
/// convention regardless of the attribute bit. On Unix, only the leading
/// dot convention exists.
fn is_hidden(name: &str, _meta: &Metadata) -> bool {
    #[cfg(windows)]
    {
        const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
        if _meta.file_attributes() & FILE_ATTRIBUTE_HIDDEN != 0 {
            return true;
        }
    }
    name.starts_with('.')
}

// =======================================================================
// Walking the tree
// =======================================================================

/// Recursively walk `dir`, building both:
///   - a `TreeNode` (for the pretty tree view), and
///   - a flat `Vec<Entry>` + `Stats` (for the table report / summary)
///     in a single filesystem pass.
///
/// Symlinks are recorded but never followed — `fs::symlink_metadata`
/// (rather than `fs::metadata`) is what makes this safe against cyclic
/// symlinks without any extra bookkeeping.
fn walk(
    dir: &Path,
    name: String,
    depth: usize,
    cfg: &Config,
    entries: &mut Vec<Entry>,
    stats: &mut Stats,
    progress: &mut Progress,
) -> TreeNode {
    let read_dir = match fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(e) => {
            stats
                .errors
                .push(format!("cannot read {}: {}", dir.display(), e));
            return TreeNode {
                name,
                kind: Kind::Dir,
                own_size: 0,
                total_size: 0,
                item_count: 0,
                truncated: false,
                hidden_by_min_size: 0,
                children: Vec::new(),
            };
        }
    };

    let mut children_raw: Vec<DirEntry> = read_dir.filter_map(|r| r.ok()).collect();
    children_raw.sort_by_key(|e| e.file_name());

    let mut tree_children: Vec<TreeNode> = Vec::new();
    let mut truncated = false;
    let mut hidden_by_min_size = 0usize;
    let mut hidden_min_size_total = 0u64;

    for child in children_raw {
        let path = child.path();
        let child_name = child.file_name().to_string_lossy().to_string();

        if cfg.excludes.iter().any(|ex| ex == &child_name) {
            stats.excluded_count += 1;
            continue;
        }
        if cfg
            .exclude_glob
            .iter()
            .any(|pat| glob_match(pat, &child_name))
        {
            stats.excluded_count += 1;
            continue;
        }

        // symlink_metadata never follows symlinks — this is the key to
        // staying safe on cyclic symlink structures.
        let meta = match fs::symlink_metadata(&path) {
            Ok(m) => m,
            Err(e) => {
                stats
                    .errors
                    .push(format!("cannot stat {}: {}", path.display(), e));
                continue;
            }
        };

        let kind = classify(&meta);
        let hidden = is_hidden(&child_name, &meta);
        let size = if kind == Kind::File { meta.len() } else { 0 };
        let modified = meta.modified().ok();

        // Extension and date filters only ever apply to files — a
        // directory is never skipped just because *it* doesn't have a
        // matching extension, since we still need to descend into it to
        // find the files that do.
        if kind == Kind::File {
            let ext = extension_of(&path);
            if !cfg.include_ext.is_empty() && !cfg.include_ext.contains(&ext) {
                stats.excluded_count += 1;
                continue;
            }
            if cfg.exclude_ext.contains(&ext) {
                stats.excluded_count += 1;
                continue;
            }

            if let Some(newer) = cfg.newer_than {
                if modified.is_none_or(|m| m < newer) {
                    stats.excluded_count += 1;
                    continue;
                }
            }
            if let Some(older) = cfg.older_than {
                if modified.is_none_or(|m| m > older) {
                    stats.excluded_count += 1;
                    continue;
                }
            }
        }

        #[cfg(unix)]
        let mode = Some(meta.permissions().mode());

        progress.tick();
        record_stats(&kind, size, hidden, depth, &path, stats);

        entries.push(Entry {
            path: path.clone(),
            kind,
            size,
            depth,
            modified,
            hidden,
            #[cfg(unix)]
            mode,
        });

        // --min-size hides small files from the TREE VIEW only (stats and
        // the detailed table still see everything) — this cuts noise when
        // you just want to spot what's eating space.
        if kind == Kind::File {
            if let Some(min) = cfg.min_size {
                if size < min {
                    hidden_by_min_size += 1;
                    hidden_min_size_total += size;
                    continue;
                }
            }
        }

        let node = if kind == Kind::Dir {
            let can_descend = cfg.max_depth.is_none_or(|m| depth < m);
            if can_descend {
                walk(&path, child_name, depth + 1, cfg, entries, stats, progress)
            } else {
                truncated = true;
                TreeNode {
                    name: child_name,
                    kind: Kind::Dir,
                    own_size: 0,
                    total_size: 0,
                    item_count: 0,
                    truncated: true,
                    hidden_by_min_size: 0,
                    children: Vec::new(),
                }
            }
        } else {
            TreeNode {
                name: child_name,
                kind,
                own_size: size,
                total_size: size,
                item_count: 0,
                truncated: false,
                hidden_by_min_size: 0,
                children: Vec::new(),
            }
        };

        tree_children.push(node);
    }

    sort_tree_children(&mut tree_children, cfg.sort);

    // Totals reflect the TRUE contents of the directory (including files
    // hidden from display by --min-size) — only the tree's visual list of
    // children is filtered, never the numbers.
    let total_size: u64 =
        tree_children.iter().map(|c| c.total_size).sum::<u64>() + hidden_min_size_total;
    let item_count: usize = tree_children
        .iter()
        .map(|c| 1 + c.item_count)
        .sum::<usize>()
        + hidden_by_min_size;

    TreeNode {
        name,
        kind: Kind::Dir,
        own_size: 0,
        total_size,
        item_count,
        truncated,
        hidden_by_min_size,
        children: tree_children,
    }
}

fn sort_tree_children(children: &mut [TreeNode], order: SortOrder) {
    match order {
        SortOrder::Name => children.sort_by(|a, b| {
            // Directories first, then files, each alphabetically (case-insensitive) —
            // the classic, easy-to-scan `tree` convention.
            let a_is_dir = a.kind == Kind::Dir;
            let b_is_dir = b.kind == Kind::Dir;
            b_is_dir
                .cmp(&a_is_dir)
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        }),
        SortOrder::Size => children.sort_by_key(|c| std::cmp::Reverse(c.total_size)),
    }
}

fn classify(meta: &Metadata) -> Kind {
    if meta.is_symlink() {
        Kind::Symlink
    } else if meta.is_dir() {
        Kind::Dir
    } else if meta.is_file() {
        Kind::File
    } else {
        Kind::Other
    }
}

fn record_stats(
    kind: &Kind,
    size: u64,
    hidden: bool,
    depth: usize,
    path: &Path,
    stats: &mut Stats,
) {
    if hidden {
        stats.hidden_count += 1;
    }
    stats.max_depth_seen = stats.max_depth_seen.max(depth);

    match kind {
        Kind::File => {
            stats.file_count += 1;
            stats.total_size += size;
            if size == 0 {
                stats.empty_file_count += 1;
            }
            let ext = extension_of(path);
            let entry = stats.by_extension.entry(ext).or_insert((0, 0));
            entry.0 += 1;
            entry.1 += size;
        }
        Kind::Dir => {
            stats.dir_count += 1;
            if fs::read_dir(path).is_ok_and(|mut rd| rd.next().is_none()) {
                stats.empty_dir_count += 1;
            }
        }
        Kind::Symlink => stats.symlink_count += 1,
        Kind::Other => stats.other_count += 1,
    }
}

fn extension_of(path: &Path) -> String {
    match path.extension() {
        Some(ext) => ext.to_string_lossy().to_lowercase(),
        None => "(no extension)".to_string(),
    }
}

/// A small, tasteful per-extension icon set for the tree view — purely
/// cosmetic, falls back to a plain document icon for anything unmapped.
/// Only used in Unicode mode; --ascii ignores this entirely.
fn file_icon_for(ext: &str) -> &'static str {
    match ext {
        "rs" => "🦀 ",
        "py" => "🐍 ",
        "js" | "ts" | "jsx" | "tsx" | "mjs" => "📜 ",
        "go" => "🐹 ",
        "java" | "kt" => "☕ ",
        "c" | "h" | "cpp" | "hpp" | "cc" => "🔧 ",
        "html" | "htm" => "🌐 ",
        "css" | "scss" | "sass" => "🎨 ",
        "json" | "toml" | "yaml" | "yml" | "xml" | "ini" | "cfg" => "⚙️ ",
        "md" | "markdown" | "txt" | "rst" => "📝 ",
        "pdf" => "📕 ",
        "zip" | "tar" | "gz" | "rar" | "7z" | "bz2" | "xz" => "📦 ",
        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "svg" | "webp" | "ico" | "tiff" => "🖼️ ",
        "mp4" | "mkv" | "avi" | "mov" | "wmv" | "flv" | "webm" => "🎬 ",
        "mp3" | "wav" | "flac" | "aac" | "ogg" | "m4a" => "🎵 ",
        "sh" | "bash" | "ps1" | "bat" | "cmd" => "🖥️ ",
        "sql" | "db" | "sqlite" => "🗄️ ",
        "lock" => "🔒 ",
        "exe" | "dll" | "so" | "dylib" => "⚡ ",
        _ => "📄 ",
    }
}

/// Broad file-type category used for the summary's "by category" table —
/// answers "what KIND of stuff is taking up space" at a glance.
fn category_of(ext: &str) -> &'static str {
    match ext {
        "rs" | "py" | "js" | "ts" | "jsx" | "tsx" | "mjs" | "go" | "java" | "kt" | "c" | "h"
        | "cpp" | "hpp" | "cc" | "cs" | "rb" | "php" | "swift" | "sh" | "bash" | "ps1" | "bat"
        | "cmd" | "html" | "htm" | "css" | "scss" | "sass" | "sql" => "Code",
        "json" | "toml" | "yaml" | "yml" | "xml" | "ini" | "cfg" | "lock" => "Config",
        "md" | "markdown" | "txt" | "rst" | "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt"
        | "pptx" | "rtf" | "odt" => "Documents",
        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "svg" | "webp" | "ico" | "tiff" => "Images",
        "mp4" | "mkv" | "avi" | "mov" | "wmv" | "flv" | "webm" => "Video",
        "mp3" | "wav" | "flac" | "aac" | "ogg" | "m4a" => "Audio",
        "zip" | "tar" | "gz" | "rar" | "7z" | "bz2" | "xz" => "Archives",
        "exe" | "dll" | "so" | "dylib" | "bin" => "Binaries",
        _ => "Other",
    }
}

/// Groups files by category, returning (category, count, total_size),
/// sorted largest-first.
fn compute_categories(entries: &[Entry]) -> Vec<(&'static str, u64, u64)> {
    let mut map: HashMap<&'static str, (u64, u64)> = HashMap::new();
    for e in entries {
        if e.kind != Kind::File {
            continue;
        }
        let ext = extension_of(&e.path);
        let cat = category_of(&ext);
        let entry = map.entry(cat).or_insert((0, 0));
        entry.0 += 1;
        entry.1 += e.size;
    }
    let mut v: Vec<(&'static str, u64, u64)> =
        map.into_iter().map(|(k, (c, s))| (k, c, s)).collect();
    v.sort_by_key(|c| std::cmp::Reverse(c.2));
    v
}

// =======================================================================
// Duplicate file detection
// =======================================================================
//
// Strategy: group files by exact size first (cheap, no I/O), then only
// hash files within a size-group that has more than one member. This
// avoids hashing the vast majority of files (which have a unique size)
// while still catching true duplicates. Files above DUP_HASH_SIZE_CAP are
// skipped entirely — we don't want to read a 10GB file into a hash just
// to compare it, and huge files are rarely accidental duplicates anyway.

const DUP_HASH_SIZE_CAP: u64 = 512 * 1024 * 1024; // 512 MB

/// Streams a file through `DefaultHasher` in 64KB chunks (never loads the
/// whole file into memory at once). `DefaultHasher` (SipHash) is not a
/// cryptographic guarantee against collisions, but combined with an exact
/// size match first, a collision here is astronomically unlikely for a
/// filesystem-scanning tool like this.
fn hash_file(path: &Path) -> Option<u64> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::Hasher;

    let file = fs::File::open(path).ok()?;
    let mut reader = io::BufReader::new(file);
    let mut hasher = DefaultHasher::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = reader.read(&mut buf).ok()?;
        if n == 0 {
            break;
        }
        hasher.write(&buf[..n]);
    }
    Some(hasher.finish())
}

/// Returns groups of files that are (almost certainly) byte-for-byte
/// duplicates, sorted by wasted space (size × (copies − 1)) descending,
/// plus a count of how many files were skipped for being too large.
fn find_duplicates(entries: &[Entry]) -> (Vec<Vec<&Entry>>, usize) {
    let mut by_size: HashMap<u64, Vec<&Entry>> = HashMap::new();
    for e in entries {
        if e.kind == Kind::File && e.size > 0 {
            by_size.entry(e.size).or_default().push(e);
        }
    }

    let mut groups: Vec<Vec<&Entry>> = Vec::new();
    let mut skipped_too_large = 0usize;

    for (size, candidates) in by_size {
        if candidates.len() < 2 {
            continue;
        }
        if size > DUP_HASH_SIZE_CAP {
            skipped_too_large += candidates.len();
            continue;
        }
        let mut by_hash: HashMap<u64, Vec<&Entry>> = HashMap::new();
        for e in candidates {
            if let Some(h) = hash_file(&e.path) {
                by_hash.entry(h).or_default().push(e);
            }
        }
        for (_, group) in by_hash {
            if group.len() > 1 {
                groups.push(group);
            }
        }
    }

    groups.sort_by_key(|g| std::cmp::Reverse(g[0].size * (g.len() as u64 - 1)));
    (groups, skipped_too_large)
}

// =======================================================================
// Colors (plain ANSI codes — no external crate needed)
// =======================================================================

const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const BLUE: &str = "\x1b[34m";
const CYAN: &str = "\x1b[36m";
const YELLOW: &str = "\x1b[33m";
const GRAY: &str = "\x1b[90m";

fn paint(s: &str, codes: &[&str], enabled: bool) -> String {
    if !enabled || s.is_empty() {
        return s.to_string();
    }
    format!("{}{}{}", codes.concat(), s, RESET)
}

// =======================================================================
// Formatting helpers
// =======================================================================

// =======================================================================
// Bidirectional text safety (Persian / Arabic / Hebrew filenames, etc.)
// =======================================================================
//
// Persian is written right-to-left. When a right-to-left name like
// "گزارش.md" is dropped into an otherwise left-to-right line — a tree
// branch ("├── گزارش.md"), a table column, a markdown table cell — the
// terminal/renderer's Unicode bidi algorithm can reorder the surrounding
// ASCII structural characters (box-drawing glyphs, punctuation, numbers)
// around it unpredictably. This is the most common reason "Persian looks
// broken" in CLI tools even though the underlying UTF-8 bytes are 100%
// correct. The fix (used by tools like `exa`/`eza`) is to wrap any name
// containing right-to-left script in Unicode directional isolates, which
// tell the renderer "this run has its own internal direction, don't let
// it disturb what's around it" — the file name itself still displays
// correctly, right-to-left, inside the isolate.

const FSI: char = '\u{2068}'; // First Strong Isolate
const PDI: char = '\u{2069}'; // Pop Directional Isolate

/// True if `s` contains a character from a right-to-left script block
/// (Arabic/Persian, Hebrew, and their extended/presentation-form ranges).
fn contains_rtl(s: &str) -> bool {
    s.chars().any(|c| {
        let cp = c as u32;
        (0x0590..=0x08FF).contains(&cp)   // Hebrew, Arabic, Arabic Supplement, Persian-specific block
            || (0xFB1D..=0xFDFF).contains(&cp) // Hebrew/Arabic presentation forms A
            || (0xFE70..=0xFEFF).contains(&cp) // Arabic presentation forms B
    })
}

/// Wraps `s` in directional isolates if (and only if) it contains
/// right-to-left script, so it can be safely inlined into an ASCII/LTR
/// line (tree views, table columns, markdown) without corrupting the
/// visual order of the characters around it.
fn bidi_safe(s: &str) -> String {
    if contains_rtl(s) {
        format!("{FSI}{s}{PDI}")
    } else {
        s.to_string()
    }
}

fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 6] = ["B", "KB", "MB", "GB", "TB", "PB"];
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{size:.2} {}", UNITS[unit])
    }
}

fn format_time(t: Option<SystemTime>) -> String {
    match t {
        None => "unknown".to_string(),
        Some(t) => match t.duration_since(UNIX_EPOCH) {
            Ok(d) => format_unix_timestamp(d),
            Err(_) => "unknown".to_string(),
        },
    }
}

/// Minimal, dependency-free UTC "YYYY-MM-DD HH:MM:SS" formatter, built
/// from a Unix timestamp using Howard Hinnant's constant-time civil
/// calendar conversion: https://howardhinnant.github.io/date_algorithms.html
fn format_unix_timestamp(d: Duration) -> String {
    let secs = d.as_secs() as i64;
    let days = secs.div_euclid(86_400);
    let secs_of_day = secs.rem_euclid(86_400);
    let (h, m, s) = (
        secs_of_day / 3600,
        (secs_of_day % 3600) / 60,
        secs_of_day % 60,
    );
    let (y, mo, da) = civil_from_days(days);
    format!("{y:04}-{mo:02}-{da:02} {h:02}:{m:02}:{s:02}")
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

#[cfg(unix)]
fn format_mode(mode: Option<u32>) -> String {
    let Some(mode) = mode else {
        return "?????????".to_string();
    };
    let bits = [
        (0o400, 'r'),
        (0o200, 'w'),
        (0o100, 'x'),
        (0o040, 'r'),
        (0o020, 'w'),
        (0o010, 'x'),
        (0o004, 'r'),
        (0o002, 'w'),
        (0o001, 'x'),
    ];
    bits.iter()
        .map(|(mask, ch)| if mode & mask != 0 { *ch } else { '-' })
        .collect()
}

// =======================================================================
// Tree rendering — the good-looking part
// =======================================================================

/// Box-drawing characters for a clean Unicode tree (what you'd see from
/// modern tools like `eza`/`tree`), with a plain-ASCII fallback for
/// terminals/fonts that mangle Unicode (some legacy Windows setups).
struct TreeChars {
    branch: &'static str, // "├── " / "|-- "
    last: &'static str,   // "└── " / "`-- "
    pipe: &'static str,   // "│   " / "|   "
    blank: &'static str,  // "    "
    dir_icon: &'static str,
    file_icon: &'static str,
    link_icon: &'static str,
}

const UNICODE_CHARS: TreeChars = TreeChars {
    branch: "├── ",
    last: "└── ",
    pipe: "│   ",
    blank: "    ",
    dir_icon: "📁 ",
    file_icon: "📄 ",
    link_icon: "🔗 ",
};

const ASCII_CHARS: TreeChars = TreeChars {
    branch: "|-- ",
    last: "`-- ",
    pipe: "|   ",
    blank: "    ",
    dir_icon: "[D] ",
    file_icon: "[F] ",
    link_icon: "[L] ",
};

fn render_tree(node: &TreeNode, cfg: &Config) -> String {
    let chars = if cfg.ascii {
        &ASCII_CHARS
    } else {
        &UNICODE_CHARS
    };
    let mut out = String::new();

    // Root line: full path, bold, with its grand totals.
    let root_label = format!(
        "{} ({} items, {})",
        bidi_safe(&cfg.root.display().to_string()),
        node.item_count,
        human_size(node.total_size)
    );
    out.push_str(&paint(&root_label, &[BOLD, BLUE], cfg.color));
    out.push('\n');

    let mut children = node.children.iter().peekable();
    while let Some(child) = children.next() {
        let is_last = children.peek().is_none();
        render_node(child, "", is_last, chars, cfg, &mut out);
    }

    out
}

fn render_node(
    node: &TreeNode,
    prefix: &str,
    is_last: bool,
    chars: &TreeChars,
    cfg: &Config,
    out: &mut String,
) {
    let connector = if is_last { chars.last } else { chars.branch };

    let line = match node.kind {
        Kind::Dir => {
            let name = paint(&bidi_safe(&node.name), &[BOLD, BLUE], cfg.color);
            let mut label = format!("{}{}", chars.dir_icon, name);
            if node.item_count > 0 {
                let meta = paint(
                    &format!(
                        "({} items, {})",
                        node.item_count,
                        human_size(node.total_size)
                    ),
                    &[DIM, GRAY],
                    cfg.color,
                );
                label.push_str(&format!("  {meta}"));
            }
            if node.truncated {
                label.push_str(&paint("  [depth limit reached]", &[DIM, YELLOW], cfg.color));
            }
            if node.hidden_by_min_size > 0 {
                let note = paint(
                    &format!("  [+{} smaller files hidden]", node.hidden_by_min_size),
                    &[DIM, GRAY],
                    cfg.color,
                );
                label.push_str(&note);
            }
            label
        }
        Kind::File => {
            let icon = if cfg.ascii {
                chars.file_icon
            } else {
                let ext = node
                    .name
                    .rsplit_once('.')
                    .map(|(_, e)| e.to_lowercase())
                    .unwrap_or_else(|| "(no extension)".to_string());
                file_icon_for(&ext)
            };
            let size = paint(
                &format!("({})", human_size(node.own_size)),
                &[DIM, GRAY],
                cfg.color,
            );
            format!("{icon}{}  {size}", bidi_safe(&node.name))
        }
        Kind::Symlink => {
            let name = paint(&bidi_safe(&node.name), &[CYAN], cfg.color);
            let tag = paint("(symlink)", &[DIM, GRAY], cfg.color);
            format!("{}{name}  {tag}", chars.link_icon)
        }
        Kind::Other => format!("{}{}", chars.file_icon, bidi_safe(&node.name)),
    };

    out.push_str(&format!("{prefix}{connector}{line}\n"));

    if node.kind == Kind::Dir && !node.children.is_empty() {
        let child_prefix = format!("{prefix}{}", if is_last { chars.blank } else { chars.pipe });
        let mut children = node.children.iter().peekable();
        while let Some(child) = children.next() {
            let child_is_last = children.peek().is_none();
            render_node(child, &child_prefix, child_is_last, chars, cfg, out);
        }
    }
}

// =======================================================================
// Report generation (detailed table + summary)
// =======================================================================

fn render_detailed_report(root: &Path, entries: &[Entry]) -> String {
    let mut out = String::new();
    out.push_str(&format!("COMPLETE REPORT — {}\n", root.display()));
    out.push_str(&format!(
        "Generated: {}\n",
        format_time(Some(SystemTime::now()))
    ));
    out.push_str(&"=".repeat(100));
    out.push('\n');
    out.push_str(&format!(
        "{:<5} {:>12} {:<20} {:<9} {}\n",
        "TYPE", "SIZE", "MODIFIED", "PERMS", "PATH"
    ));
    out.push_str(&"-".repeat(100));
    out.push('\n');

    for e in entries {
        let indent = "  ".repeat(e.depth);
        let name = e
            .path
            .strip_prefix(root)
            .unwrap_or(&e.path)
            .display()
            .to_string();
        let size_str = if e.kind == Kind::File {
            human_size(e.size)
        } else {
            String::from("-")
        };

        #[cfg(unix)]
        let perms = format_mode(e.mode);
        #[cfg(not(unix))]
        let perms = "n/a".to_string();

        out.push_str(&format!(
            "{:<5} {:>12} {:<20} {:<9} {}{}\n",
            e.kind,
            size_str,
            format_time(e.modified),
            perms,
            indent,
            bidi_safe(&name)
        ));
    }
    out
}

fn render_summary_report(
    root: &Path,
    stats: &Stats,
    entries: &[Entry],
    dirs: &[(PathBuf, u64, usize)],
    top_n: usize,
    elapsed: Duration,
    duplicates: Option<&(Vec<Vec<&Entry>>, usize)>,
) -> String {
    let mut out = String::new();
    out.push_str(&format!("SUMMARY REPORT — {}\n", root.display()));
    out.push_str(&"=".repeat(70));
    out.push('\n');

    let total_entries =
        stats.file_count + stats.dir_count + stats.symlink_count + stats.other_count;
    out.push_str(&format!("Total entries scanned : {total_entries}\n"));
    out.push_str(&format!("  Files               : {}\n", stats.file_count));
    out.push_str(&format!("  Directories         : {}\n", stats.dir_count));
    out.push_str(&format!(
        "  Symlinks            : {}\n",
        stats.symlink_count
    ));
    if stats.other_count > 0 {
        out.push_str(&format!("  Other (sockets/etc) : {}\n", stats.other_count));
    }
    out.push_str(&format!(
        "Total size of files   : {} ({} bytes)\n",
        human_size(stats.total_size),
        stats.total_size
    ));
    out.push_str(&format!("Hidden entries        : {}\n", stats.hidden_count));
    out.push_str(&format!(
        "Empty files           : {}\n",
        stats.empty_file_count
    ));
    out.push_str(&format!(
        "Empty directories     : {}\n",
        stats.empty_dir_count
    ));
    if stats.excluded_count > 0 {
        out.push_str(&format!(
            "Excluded entries      : {}\n",
            stats.excluded_count
        ));
    }
    out.push_str(&format!(
        "Max tree depth        : {}\n",
        stats.max_depth_seen
    ));
    if stats.file_count > 0 {
        let avg = stats.total_size as f64 / stats.file_count as f64;
        out.push_str(&format!(
            "Average file size     : {}\n",
            human_size(avg.round() as u64)
        ));
    }
    out.push_str(&format!("Scan duration         : {:.1?}\n", elapsed));

    out.push('\n');
    out.push_str("Breakdown by category\n");
    out.push_str(&"-".repeat(70));
    out.push('\n');
    out.push_str(&format!(
        "{:<14} {:>10} {:>15}\n",
        "CATEGORY", "COUNT", "TOTAL SIZE"
    ));
    for (cat, count, size) in compute_categories(entries) {
        out.push_str(&format!(
            "{:<14} {:>10} {:>15}\n",
            cat,
            count,
            human_size(size)
        ));
    }

    out.push('\n');
    out.push_str("Breakdown by extension (top 15 by total size)\n");
    out.push_str(&"-".repeat(70));
    out.push('\n');
    out.push_str(&format!(
        "{:<18} {:>10} {:>15}\n",
        "EXTENSION", "COUNT", "TOTAL SIZE"
    ));
    let mut ext_vec: Vec<(&String, &(u64, u64))> = stats.by_extension.iter().collect();
    ext_vec.sort_by_key(|e| std::cmp::Reverse(e.1 .1));
    for (ext, (count, size)) in ext_vec.iter().take(15) {
        out.push_str(&format!(
            "{:<18} {:>10} {:>15}\n",
            ext,
            count,
            human_size(*size)
        ));
    }

    out.push('\n');
    out.push_str(&format!("Largest {top_n} files\n"));
    out.push_str(&"-".repeat(70));
    out.push('\n');
    let mut files: Vec<&Entry> = entries.iter().filter(|e| e.kind == Kind::File).collect();
    files.sort_by_key(|f| std::cmp::Reverse(f.size));
    for f in files.iter().take(top_n) {
        let rel = f
            .path
            .strip_prefix(root)
            .unwrap_or(&f.path)
            .display()
            .to_string();
        out.push_str(&format!(
            "{:>12}  {}\n",
            human_size(f.size),
            bidi_safe(&rel)
        ));
    }

    if !dirs.is_empty() {
        out.push('\n');
        out.push_str(&format!("Largest {top_n} directories\n"));
        out.push_str(&"-".repeat(70));
        out.push('\n');
        for (path, size, count) in dirs {
            let rel = path
                .strip_prefix(root)
                .unwrap_or(path)
                .display()
                .to_string();
            out.push_str(&format!(
                "{:>12}  {}  ({count} items)\n",
                human_size(*size),
                bidi_safe(&rel)
            ));
        }
    }

    let mut by_time: Vec<&Entry> = entries
        .iter()
        .filter(|e| e.kind == Kind::File && e.modified.is_some())
        .collect();
    by_time.sort_by_key(|e| e.modified);
    if let Some(oldest) = by_time.first() {
        let rel = oldest
            .path
            .strip_prefix(root)
            .unwrap_or(&oldest.path)
            .display()
            .to_string();
        out.push('\n');
        out.push_str(&format!(
            "Oldest file: {} ({})\n",
            bidi_safe(&rel),
            format_time(oldest.modified)
        ));
    }
    if let Some(newest) = by_time.last() {
        let rel = newest
            .path
            .strip_prefix(root)
            .unwrap_or(&newest.path)
            .display()
            .to_string();
        out.push_str(&format!(
            "Newest file: {} ({})\n",
            bidi_safe(&rel),
            format_time(newest.modified)
        ));
    }

    if let Some((groups, skipped)) = duplicates {
        out.push('\n');
        out.push_str("Duplicate files\n");
        out.push_str(&"-".repeat(70));
        out.push('\n');
        if groups.is_empty() {
            out.push_str("No duplicate files found.\n");
        } else {
            let total_wasted: u64 = groups
                .iter()
                .map(|g| g[0].size * (g.len() as u64 - 1))
                .sum();
            out.push_str(&format!(
                "{} duplicate group(s) found — approx. {} wasted\n",
                groups.len(),
                human_size(total_wasted)
            ));
            for (i, group) in groups.iter().take(10).enumerate() {
                out.push_str(&format!(
                    "  Group {}: {} x{} copies\n",
                    i + 1,
                    human_size(group[0].size),
                    group.len()
                ));
                for e in group {
                    let rel = e
                        .path
                        .strip_prefix(root)
                        .unwrap_or(&e.path)
                        .display()
                        .to_string();
                    out.push_str(&format!("    - {}\n", bidi_safe(&rel)));
                }
            }
            if groups.len() > 10 {
                out.push_str(&format!(
                    "  ... and {} more group(s) (see markdown/full report)\n",
                    groups.len() - 10
                ));
            }
        }
        if *skipped > 0 {
            out.push_str(&format!(
                "({skipped} file(s) skipped — larger than {})\n",
                human_size(DUP_HASH_SIZE_CAP)
            ));
        }
    }

    if !stats.errors.is_empty() {
        out.push('\n');
        out.push_str(&format!(
            "Warnings ({} entries could not be read)\n",
            stats.errors.len()
        ));
        out.push_str(&"-".repeat(70));
        out.push('\n');
        for err in &stats.errors {
            out.push_str(&format!("  - {err}\n"));
        }
    }

    out
}

// =======================================================================
// Markdown export — clean, GitHub-flavored, easy on the eyes
// =======================================================================

fn md_escape(s: &str) -> String {
    // Content is always wrapped in backticks below, which already protects
    // it from emphasis/heading parsing — the one thing that still needs
    // escaping inside a GFM table cell is the pipe character itself.
    s.replace('|', "\\|")
}

fn render_markdown(
    cfg: &Config,
    tree: &TreeNode,
    stats: &Stats,
    entries: &[Entry],
    dirs: &[(PathBuf, u64, usize)],
    duplicates: Option<&(Vec<Vec<&Entry>>, usize)>,
    elapsed: Duration,
) -> String {
    let mut out = String::new();
    let root_display = cfg.root.display().to_string();
    let rel = |p: &Path| -> String {
        bidi_safe(&p.strip_prefix(&cfg.root).unwrap_or(p).display().to_string())
    };

    out.push_str("# 📊 Directory Scan Report\n\n");
    out.push_str(&format!(
        "**Path:** `{}`  \n",
        md_escape(&bidi_safe(&root_display))
    ));
    out.push_str(&format!(
        "**Generated:** {}  \n",
        format_time(Some(SystemTime::now()))
    ));
    out.push_str(&format!("**Scan duration:** {elapsed:.1?}\n\n"));
    out.push_str("---\n\n");

    out.push_str("## 📁 Directory Tree\n\n");
    out.push_str("```text\n");
    let mut plain_cfg = cfg.clone();
    plain_cfg.color = false;
    out.push_str(&render_tree(tree, &plain_cfg));
    out.push_str("```\n\n");
    out.push_str("---\n\n");

    out.push_str("## 📈 Overview\n\n");
    out.push_str("| Metric | Value |\n|---|---|\n");
    let total_entries =
        stats.file_count + stats.dir_count + stats.symlink_count + stats.other_count;
    out.push_str(&format!("| Total entries | {total_entries} |\n"));
    out.push_str(&format!("| Files | {} |\n", stats.file_count));
    out.push_str(&format!("| Directories | {} |\n", stats.dir_count));
    out.push_str(&format!("| Symlinks | {} |\n", stats.symlink_count));
    if stats.other_count > 0 {
        out.push_str(&format!(
            "| Other (sockets/etc) | {} |\n",
            stats.other_count
        ));
    }
    out.push_str(&format!(
        "| Total size | {} ({} bytes) |\n",
        human_size(stats.total_size),
        stats.total_size
    ));
    out.push_str(&format!("| Hidden entries | {} |\n", stats.hidden_count));
    out.push_str(&format!("| Empty files | {} |\n", stats.empty_file_count));
    out.push_str(&format!(
        "| Empty directories | {} |\n",
        stats.empty_dir_count
    ));
    if stats.excluded_count > 0 {
        out.push_str(&format!(
            "| Excluded entries | {} |\n",
            stats.excluded_count
        ));
    }
    out.push_str(&format!("| Max tree depth | {} |\n", stats.max_depth_seen));
    if stats.file_count > 0 {
        let avg = stats.total_size as f64 / stats.file_count as f64;
        out.push_str(&format!(
            "| Average file size | {} |\n",
            human_size(avg.round() as u64)
        ));
    }
    out.push('\n');
    out.push_str("---\n\n");

    out.push_str("## 🗂️ Breakdown by Category\n\n");
    out.push_str("| Category | Files | Total Size |\n|---|---|---|\n");
    for (cat, count, size) in compute_categories(entries) {
        out.push_str(&format!("| {cat} | {count} | {} |\n", human_size(size)));
    }
    out.push('\n');
    out.push_str("---\n\n");

    out.push_str("## 🧩 Breakdown by Extension\n\n");
    out.push_str("| Extension | Count | Total Size |\n|---|---|---|\n");
    let mut ext_vec: Vec<(&String, &(u64, u64))> = stats.by_extension.iter().collect();
    ext_vec.sort_by_key(|e| std::cmp::Reverse(e.1 .1));
    for (ext, (count, size)) in ext_vec.iter().take(15) {
        out.push_str(&format!(
            "| `{}` | {count} | {} |\n",
            md_escape(ext),
            human_size(*size)
        ));
    }
    out.push('\n');
    out.push_str("---\n\n");

    out.push_str(&format!("## 🏆 Largest {} Files\n\n", cfg.top_n));
    out.push_str("| # | Size | Path |\n|---|---|---|\n");
    let mut files: Vec<&Entry> = entries.iter().filter(|e| e.kind == Kind::File).collect();
    files.sort_by_key(|f| std::cmp::Reverse(f.size));
    for (i, f) in files.iter().take(cfg.top_n).enumerate() {
        out.push_str(&format!(
            "| {} | {} | `{}` |\n",
            i + 1,
            human_size(f.size),
            md_escape(&rel(&f.path))
        ));
    }
    out.push('\n');
    out.push_str("---\n\n");

    if !dirs.is_empty() {
        out.push_str(&format!("## 🗃️ Largest {} Directories\n\n", cfg.top_n));
        out.push_str("| # | Total Size | Items | Path |\n|---|---|---|---|\n");
        for (i, (path, size, count)) in dirs.iter().enumerate() {
            let rel_path = bidi_safe(
                &path
                    .strip_prefix(&cfg.root)
                    .unwrap_or(path)
                    .display()
                    .to_string(),
            );
            out.push_str(&format!(
                "| {} | {} | {count} | `{}` |\n",
                i + 1,
                human_size(*size),
                md_escape(&rel_path)
            ));
        }
        out.push('\n');
        out.push_str("---\n\n");
    }

    out.push_str("## 🕒 Timeline\n\n");
    let mut by_time: Vec<&Entry> = entries
        .iter()
        .filter(|e| e.kind == Kind::File && e.modified.is_some())
        .collect();
    by_time.sort_by_key(|e| e.modified);
    if let Some(oldest) = by_time.first() {
        out.push_str(&format!(
            "- **Oldest file:** `{}` — {}\n",
            md_escape(&rel(&oldest.path)),
            format_time(oldest.modified)
        ));
    }
    if let Some(newest) = by_time.last() {
        out.push_str(&format!(
            "- **Newest file:** `{}` — {}\n",
            md_escape(&rel(&newest.path)),
            format_time(newest.modified)
        ));
    }
    out.push('\n');

    if let Some((groups, skipped)) = duplicates {
        out.push_str("---\n\n");
        out.push_str("## 🧬 Duplicate Files\n\n");
        if groups.is_empty() {
            out.push_str("No duplicate files found. ✅\n\n");
        } else {
            let total_wasted: u64 = groups
                .iter()
                .map(|g| g[0].size * (g.len() as u64 - 1))
                .sum();
            out.push_str(&format!(
                "Found **{}** group(s) of duplicate files, wasting approximately **{}** of disk space.\n\n",
                groups.len(),
                human_size(total_wasted)
            ));
            for (i, group) in groups.iter().enumerate() {
                let wasted = group[0].size * (group.len() as u64 - 1);
                out.push_str(&format!(
                    "<details>\n<summary><strong>Group {}</strong> — {} × {} copies (wastes {})</summary>\n\n",
                    i + 1,
                    human_size(group[0].size),
                    group.len(),
                    human_size(wasted)
                ));
                for e in group {
                    out.push_str(&format!("- `{}`\n", md_escape(&rel(&e.path))));
                }
                out.push_str("\n</details>\n\n");
            }
        }
        if *skipped > 0 {
            out.push_str(&format!(
                "*Note: {skipped} file(s) were skipped from duplicate checking for being larger than {}.*\n\n",
                human_size(DUP_HASH_SIZE_CAP)
            ));
        }
    }

    if !stats.errors.is_empty() {
        out.push_str("---\n\n");
        out.push_str(&format!(
            "## ⚠️ Warnings ({} entries)\n\n",
            stats.errors.len()
        ));
        for err in &stats.errors {
            out.push_str(&format!("- {}\n", md_escape(err)));
        }
        out.push('\n');
    }

    out.push_str("---\n\n");
    out.push_str("*Generated by treeport — a single-file Rust directory analyzer.*\n");

    out
}

// =======================================================================
// Largest directories (complements "largest files")
// =======================================================================

/// Walks the already-built tree collecting every directory's path,
/// recursive total size, and item count — used for the "largest
/// directories" section, which answers "which *folder* is eating my
/// disk", a step up from "which *file* is eating my disk".
fn collect_dirs(node: &TreeNode, prefix: &Path, out: &mut Vec<(PathBuf, u64, usize)>) {
    for child in &node.children {
        if child.kind == Kind::Dir {
            let path = prefix.join(&child.name);
            out.push((path.clone(), child.total_size, child.item_count));
            collect_dirs(child, &path, out);
        }
    }
}

fn top_dirs(tree: &TreeNode, root: &Path, top_n: usize) -> Vec<(PathBuf, u64, usize)> {
    let mut dirs = Vec::new();
    collect_dirs(tree, root, &mut dirs);
    dirs.sort_by_key(|d| std::cmp::Reverse(d.1));
    dirs.truncate(top_n);
    dirs
}

// =======================================================================
// JSON export — dependency-free, hand-rolled (no serde)
// =======================================================================

/// Minimal, correct JSON string escaping — control characters, quotes,
/// and backslashes. Everything else (including all of Unicode, so
/// Persian text) passes through as literal UTF-8, which is valid inside
/// a JSON string and is what every modern JSON parser expects.
fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

fn json_str(s: &str) -> String {
    format!("\"{}\"", json_escape(s))
}

fn render_json(
    cfg: &Config,
    stats: &Stats,
    entries: &[Entry],
    dup_result: Option<&(Vec<Vec<&Entry>>, usize)>,
    elapsed: Duration,
) -> String {
    let rel = |p: &Path| -> String { p.strip_prefix(&cfg.root).unwrap_or(p).display().to_string() };
    let mut out = String::new();
    out.push_str("{\n");
    out.push_str(&format!(
        "  \"path\": {},\n",
        json_str(&cfg.root.display().to_string())
    ));
    out.push_str(&format!(
        "  \"generated\": {},\n",
        json_str(&format_time(Some(SystemTime::now())))
    ));
    out.push_str(&format!(
        "  \"scan_duration_ms\": {},\n",
        elapsed.as_millis()
    ));

    out.push_str("  \"summary\": {\n");
    out.push_str(&format!("    \"files\": {},\n", stats.file_count));
    out.push_str(&format!("    \"directories\": {},\n", stats.dir_count));
    out.push_str(&format!("    \"symlinks\": {},\n", stats.symlink_count));
    out.push_str(&format!("    \"other\": {},\n", stats.other_count));
    out.push_str(&format!(
        "    \"total_size_bytes\": {},\n",
        stats.total_size
    ));
    out.push_str(&format!(
        "    \"hidden_entries\": {},\n",
        stats.hidden_count
    ));
    out.push_str(&format!(
        "    \"empty_files\": {},\n",
        stats.empty_file_count
    ));
    out.push_str(&format!(
        "    \"empty_directories\": {},\n",
        stats.empty_dir_count
    ));
    out.push_str(&format!(
        "    \"excluded_entries\": {},\n",
        stats.excluded_count
    ));
    out.push_str(&format!("    \"max_depth\": {}\n", stats.max_depth_seen));
    out.push_str("  },\n");

    out.push_str("  \"by_category\": [\n");
    let cats = compute_categories(entries);
    for (i, (cat, count, size)) in cats.iter().enumerate() {
        let comma = if i + 1 < cats.len() { "," } else { "" };
        out.push_str(&format!(
            "    {{ \"category\": {}, \"count\": {count}, \"total_size_bytes\": {size} }}{comma}\n",
            json_str(cat)
        ));
    }
    out.push_str("  ],\n");

    out.push_str("  \"by_extension\": [\n");
    let mut ext_vec: Vec<(&String, &(u64, u64))> = stats.by_extension.iter().collect();
    ext_vec.sort_by_key(|e| std::cmp::Reverse(e.1 .1));
    for (i, (ext, (count, size))) in ext_vec.iter().enumerate() {
        let comma = if i + 1 < ext_vec.len() { "," } else { "" };
        out.push_str(&format!(
            "    {{ \"extension\": {}, \"count\": {count}, \"total_size_bytes\": {size} }}{comma}\n",
            json_str(ext)
        ));
    }
    out.push_str("  ],\n");

    out.push_str("  \"entries\": [\n");
    for (i, e) in entries.iter().enumerate() {
        let comma = if i + 1 < entries.len() { "," } else { "" };
        let kind = match e.kind {
            Kind::File => "file",
            Kind::Dir => "dir",
            Kind::Symlink => "symlink",
            Kind::Other => "other",
        };
        out.push_str(&format!(
            "    {{ \"path\": {}, \"kind\": {}, \"size_bytes\": {}, \"depth\": {}, \"modified\": {}, \"hidden\": {} }}{comma}\n",
            json_str(&rel(&e.path)),
            json_str(kind),
            e.size,
            e.depth,
            json_str(&format_time(e.modified)),
            e.hidden
        ));
    }
    out.push_str("  ]");

    if let Some((groups, skipped)) = dup_result {
        out.push_str(",\n  \"duplicates\": {\n");
        out.push_str(&format!("    \"skipped_too_large\": {skipped},\n"));
        out.push_str("    \"groups\": [\n");
        for (gi, group) in groups.iter().enumerate() {
            let gcomma = if gi + 1 < groups.len() { "," } else { "" };
            out.push_str(&format!(
                "      {{ \"size_bytes\": {}, \"files\": [",
                group[0].size
            ));
            for (fi, e) in group.iter().enumerate() {
                let fcomma = if fi + 1 < group.len() { ", " } else { "" };
                out.push_str(&format!("{}{}", json_str(&rel(&e.path)), fcomma));
            }
            out.push_str(&format!("] }}{gcomma}\n"));
        }
        out.push_str("    ]\n  }\n");
    } else {
        out.push('\n');
    }

    out.push_str("}\n");
    out
}

// =======================================================================
// CSV export
// =======================================================================

fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') || s.contains('\r') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn render_csv(root: &Path, entries: &[Entry]) -> String {
    let mut out = String::new();
    out.push_str("path,kind,size_bytes,depth,modified,hidden\n");
    for e in entries {
        let rel = e
            .path
            .strip_prefix(root)
            .unwrap_or(&e.path)
            .display()
            .to_string();
        let kind = match e.kind {
            Kind::File => "file",
            Kind::Dir => "dir",
            Kind::Symlink => "symlink",
            Kind::Other => "other",
        };
        out.push_str(&format!(
            "{},{},{},{},{},{}\n",
            csv_escape(&rel),
            kind,
            e.size,
            e.depth,
            csv_escape(&format_time(e.modified)),
            e.hidden
        ));
    }
    out
}

// =======================================================================
// File writing (with optional UTF-8 BOM for Windows/Excel compatibility)
// =======================================================================

/// Writes UTF-8 text to `path`, optionally prefixed with a UTF-8 BOM
/// (EF BB BF). The BOM is redundant on Linux/macOS but is what makes
/// Windows Notepad and Excel *reliably* detect "this file is UTF-8" and
/// render Persian/Arabic/any non-Latin text correctly instead of
/// guessing the legacy system code page and showing mojibake — see
/// --utf8-bom in the help text.
fn write_text_file(path: &Path, content: &str, bom: bool) -> Result<(), String> {
    let mut bytes = Vec::with_capacity(content.len() + 3);
    if bom {
        bytes.extend_from_slice(&[0xEF, 0xBB, 0xBF]);
    }
    bytes.extend_from_slice(content.as_bytes());
    fs::write(path, &bytes).map_err(|e| format!("failed to write {}: {}", path.display(), e))
}

// =======================================================================
// main
// =======================================================================

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let cfg = Config::parse(&args)?;

    let start = Instant::now();
    let mut entries = Vec::new();
    let mut stats = Stats::new();
    let mut progress = Progress::new(cfg.show_progress);
    let root_label = cfg.root.display().to_string();
    let tree = walk(
        &cfg.root,
        root_label,
        0,
        &cfg,
        &mut entries,
        &mut stats,
        &mut progress,
    );
    progress.finish();
    let elapsed = start.elapsed();

    if !cfg.quiet {
        print!("{}", render_tree(&tree, &cfg));
        println!();
    }

    if cfg.show_full_table || cfg.output.is_some() {
        let detailed = render_detailed_report(&cfg.root, &entries);
        if cfg.show_full_table {
            print!("{detailed}");
            println!();
        }
        if let Some(out_path) = &cfg.output {
            write_text_file(out_path, &detailed, cfg.utf8_bom)?;
            eprintln!("(full report written to {})", out_path.display());
        }
    }

    let dup_result = if cfg.find_duplicates {
        Some(find_duplicates(&entries))
    } else {
        None
    };

    let dirs = top_dirs(&tree, &cfg.root, cfg.top_n);
    let summary = render_summary_report(
        &cfg.root,
        &stats,
        &entries,
        &dirs,
        cfg.top_n,
        elapsed,
        dup_result.as_ref(),
    );
    print!("{summary}");

    if let Some(md_path) = &cfg.markdown {
        let markdown = render_markdown(
            &cfg,
            &tree,
            &stats,
            &entries,
            &dirs,
            dup_result.as_ref(),
            elapsed,
        );
        write_text_file(md_path, &markdown, cfg.utf8_bom)?;
        eprintln!("(markdown report written to {})", md_path.display());
    }

    if let Some(json_path) = &cfg.json {
        let json = render_json(&cfg, &stats, &entries, dup_result.as_ref(), elapsed);
        // JSON never gets a BOM even if --utf8-bom is set: the JSON spec
        // (RFC 8259) says a BOM must not be added, and strict parsers
        // (Python's json module, some JS engines) reject one outright.
        // --utf8-bom only ever applies to formats meant for human eyes
        // in a text editor (.txt/.md), where BOM detection actually helps.
        write_text_file(json_path, &json, false)?;
        eprintln!("(JSON report written to {})", json_path.display());
    }

    if let Some(csv_path) = &cfg.csv {
        let csv = render_csv(&cfg.root, &entries);
        // CSV always gets a BOM: Excel's "open .csv" flow (as opposed to
        // its explicit UTF-8 import dialog) silently assumes the system
        // code page without one, which is precisely what mangles Persian
        // text — this one is not optional the way --utf8-bom is for the
        // other formats.
        write_text_file(csv_path, &csv, true)?;
        eprintln!("(CSV report written to {})", csv_path.display());
    }

    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(msg) => {
            let _ = writeln!(io::stderr(), "{msg}");
            ExitCode::FAILURE
        }
    }
}
