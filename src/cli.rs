use std::fs;
use std::io::{self, IsTerminal};
use std::path::PathBuf;
use std::time::SystemTime;

use crate::model::SortOrder;
use crate::platform;
use crate::validate_root;

#[derive(Debug, Clone)]
pub struct Config {
    pub root: PathBuf,
    pub output: Option<PathBuf>,
    pub markdown: Option<PathBuf>,
    pub json: Option<PathBuf>,
    pub csv: Option<PathBuf>,
    pub file_list: Option<PathBuf>,
    pub top_n: usize,
    pub max_depth: Option<usize>,
    pub excludes: Vec<String>,
    pub exclude_glob: Vec<String>,
    pub include_ext: Vec<String>,
    pub exclude_ext: Vec<String>,
    pub newer_than: Option<SystemTime>,
    pub older_than: Option<SystemTime>,
    pub sort: SortOrder,
    pub ascii: bool,
    pub color: bool,
    pub utf8_bom: bool,
    pub show_full_table: bool,
    pub quiet: bool,
    pub find_duplicates: bool,
    pub min_size: Option<u64>,
    pub show_progress: bool,
}

impl Config {
    pub fn parse(args: &[String]) -> Result<Self, String> {
        let mut root = None;
        let mut output = None;
        let mut markdown = None;
        let mut json = None;
        let mut csv = None;
        let mut file_list = None;
        let mut top_n = 10;
        let mut max_depth = None;
        let mut excludes = Vec::new();
        let mut exclude_glob = Vec::new();
        let mut include_ext = Vec::new();
        let mut exclude_ext = Vec::new();
        let mut newer_than = None;
        let mut older_than = None;
        let mut sort = SortOrder::Name;
        let mut ascii = false;
        let mut color_flag = None;
        let mut utf8_bom = false;
        let mut show_full_table = false;
        let mut quiet = false;
        let mut find_duplicates = false;
        let mut min_size = None;
        let mut show_progress = true;
        let mut ignore_file = None;
        let mut no_ignore_file = false;

        let mut i = 0;
        while i < args.len() {
            match args[i].as_str() {
                "-h" | "--help" => {
                    print!("{USAGE}");
                    std::process::exit(0);
                }
                "-o" | "--output" => {
                    output = Some(PathBuf::from(Self::next(
                        args,
                        &mut i,
                        "--output requires a file path",
                    )?))
                }
                "-m" | "--markdown" => {
                    markdown = Some(PathBuf::from(Self::next(
                        args,
                        &mut i,
                        "--markdown requires a file path",
                    )?))
                }
                "-j" | "--json" => {
                    json = Some(PathBuf::from(Self::next(
                        args,
                        &mut i,
                        "--json requires a file path",
                    )?))
                }
                "-c" | "--csv" => {
                    csv = Some(PathBuf::from(Self::next(
                        args,
                        &mut i,
                        "--csv requires a file path",
                    )?))
                }
                "--file-list" => {
                    file_list = Some(PathBuf::from(Self::next(
                        args,
                        &mut i,
                        "--file-list requires a file path",
                    )?))
                }
                "--find-duplicates" => find_duplicates = true,
                "--min-size" => {
                    min_size = Some(crate::util::parse_size(Self::next(
                        args,
                        &mut i,
                        "--min-size requires a size, e.g. 1MB",
                    )?)?)
                }
                "-f" | "--full" => show_full_table = true,
                "-t" | "--top" => {
                    let v = Self::next(args, &mut i, "--top requires a number")?;
                    top_n = v.parse().map_err(|_| format!("invalid --top value: {v}"))?;
                }
                "-d" | "--max-depth" => {
                    let v = Self::next(args, &mut i, "--max-depth requires a number")?;
                    max_depth = Some(
                        v.parse()
                            .map_err(|_| format!("invalid --max-depth value: {v}"))?,
                    );
                }
                "-e" | "--exclude" => excludes
                    .push(Self::next(args, &mut i, "--exclude requires a name")?.to_string()),
                "--exclude-glob" => exclude_glob.push(
                    Self::next(
                        args,
                        &mut i,
                        "--exclude-glob requires a pattern, e.g. '*.log'",
                    )?
                    .to_string(),
                ),
                "--include-ext" => include_ext.push(
                    Self::next(args, &mut i, "--include-ext requires an extension, e.g. rs")?
                        .trim_start_matches('.')
                        .to_lowercase(),
                ),
                "--exclude-ext" => exclude_ext.push(
                    Self::next(
                        args,
                        &mut i,
                        "--exclude-ext requires an extension, e.g. log",
                    )?
                    .trim_start_matches('.')
                    .to_lowercase(),
                ),
                "--newer-than" => {
                    let v = Self::next(
                        args,
                        &mut i,
                        "--newer-than requires a date, e.g. 2025-01-01",
                    )?;
                    newer_than = Some(crate::util::parse_date(v)?);
                }
                "--older-than" => {
                    let v = Self::next(
                        args,
                        &mut i,
                        "--older-than requires a date, e.g. 2025-01-01",
                    )?;
                    older_than = Some(crate::util::parse_date(v)?);
                }
                "-s" | "--sort" => {
                    let v = Self::next(args, &mut i, "--sort requires 'name' or 'size'")?;
                    sort = match v {
                        "name" => SortOrder::Name,
                        "size" => SortOrder::Size,
                        other => {
                            return Err(format!(
                                "invalid --sort value: {other} (use 'name' or 'size')"
                            ));
                        }
                    };
                }
                "--ascii" => ascii = true,
                "--no-color" => color_flag = Some(false),
                "--color" => color_flag = Some(true),
                "--utf8-bom" => utf8_bom = true,
                "--no-progress" => show_progress = false,
                "-q" | "--quiet" => quiet = true,
                "--ignore-file" => {
                    ignore_file = Some(PathBuf::from(Self::next(
                        args,
                        &mut i,
                        "--ignore-file requires a file path",
                    )?))
                }
                "--no-ignore-file" => no_ignore_file = true,
                other if root.is_none() && !other.starts_with('-') => {
                    root = Some(PathBuf::from(other))
                }
                other => return Err(format!("unknown argument: {other}\n\n{USAGE}")),
            }
            i += 1;
        }

        let root = root.unwrap_or_else(|| PathBuf::from("."));
        validate_root(&root)?;

        if !no_ignore_file {
            let path = ignore_file
                .clone()
                .unwrap_or_else(|| root.join(".treeportignore"));
            match fs::read_to_string(&path) {
                Ok(contents) => {
                    for line in contents.lines() {
                        let line = line.trim();
                        if !line.is_empty() && !line.starts_with('#') {
                            exclude_glob.push(line.to_string());
                        }
                    }
                }
                Err(_) if ignore_file.is_some() => {
                    return Err(format!("cannot read ignore file: {}", path.display()));
                }
                Err(_) => {}
            }
        }

        let color = color_flag.unwrap_or_else(|| io::stdout().is_terminal());
        platform::initialize_console(color);
        let show_progress = show_progress && io::stderr().is_terminal();

        Ok(Self {
            root,
            output,
            markdown,
            json,
            csv,
            file_list,
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

    fn next<'a>(args: &'a [String], i: &mut usize, message: &str) -> Result<&'a str, String> {
        *i += 1;
        args.get(*i)
            .map(String::as_str)
            .ok_or_else(|| message.to_string())
    }
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
    --file-list <file>      Export a CSV inventory of FILES only — one row per
                             file, numbered: "File Number", "File Name",
                             "Relative Path" (relative to <path>). Sorted by
                             file name; real UTF-8 throughout, so Persian /
                             Arabic / Hebrew names survive intact. Narrow it
                             to one format with --include-ext (e.g.
                             --include-ext md); with no --include-ext every
                             file is listed. Always BOM-prefixed, like --csv.
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
    treeport . --include-ext md --file-list docs.csv    # inventory of .md files
    treeport "پروژه من" --include-ext md --file-list "فهرست.md.csv"
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
