use std::fs::{self, DirEntry, Metadata};
use std::io::{self, Write};
use std::path::Path;
use std::time::{Duration, Instant, SystemTime};

use crate::cli::Config;
use crate::model::{Entry, Kind, Stats, TreeNode};
use crate::platform;
use crate::util::{extension_of, glob_match};

pub struct ScanResult {
    pub tree: TreeNode,
    pub entries: Vec<Entry>,
    pub stats: Stats,
}

pub struct Scanner<'a> {
    config: &'a Config,
}

impl<'a> Scanner<'a> {
    pub fn new(config: &'a Config) -> Self {
        Self { config }
    }

    pub fn scan(&self, progress: &mut Progress) -> Result<ScanResult, String> {
        let mut entries = Vec::new();
        let mut stats = Stats::default();
        let root_name = self.config.root.display().to_string();
        let tree = walk(
            &self.config.root,
            root_name,
            0,
            self.config,
            &mut entries,
            &mut stats,
            progress,
        );
        Ok(ScanResult {
            tree,
            entries,
            stats,
        })
    }
}

pub struct Progress {
    enabled: bool,
    count: u64,
    last_print: Instant,
}

impl Progress {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            count: 0,
            last_print: Instant::now(),
        }
    }

    pub fn tick(&mut self) {
        if !self.enabled {
            return;
        }
        self.count += 1;
        if self.count % 64 == 0 && self.last_print.elapsed() >= Duration::from_millis(100) {
            eprint!("\rscanning… {} entries", self.count);
            let _ = io::stderr().flush();
            self.last_print = Instant::now();
        }
    }

    pub fn finish(&self) {
        if self.enabled && self.count > 0 {
            eprint!("\r{}\r", " ".repeat(24));
            let _ = io::stderr().flush();
        }
    }
}

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
            return empty_dir_node(name);
        }
    };

    let mut children_raw: Vec<DirEntry> = read_dir.filter_map(Result::ok).collect();
    children_raw.sort_by_key(|e| e.file_name());

    let mut children = Vec::new();
    let mut truncated = false;
    let mut hidden_by_min_size = 0usize;
    let mut hidden_min_size_total = 0u64;

    for child in children_raw {
        let path = child.path();
        let child_name = child.file_name().to_string_lossy().into_owned();

        if cfg.excludes.iter().any(|ex| ex == &child_name)
            || cfg
                .exclude_glob
                .iter()
                .any(|pat| glob_match(pat, &child_name))
        {
            stats.excluded_count += 1;
            continue;
        }

        let meta = match fs::symlink_metadata(&path) {
            Ok(meta) => meta,
            Err(e) => {
                stats
                    .errors
                    .push(format!("cannot stat {}: {}", path.display(), e));
                continue;
            }
        };

        let kind = classify(&meta);
        let hidden = platform::is_hidden(&child_name, &meta);
        let size = if kind == Kind::File { meta.len() } else { 0 };
        let modified = meta.modified().ok();

        if kind == Kind::File && !passes_file_filters(&path, modified, cfg) {
            stats.excluded_count += 1;
            continue;
        }

        progress.tick();
        record_stats(kind, size, hidden, depth, &path, &meta, stats);
        entries.push(Entry {
            path: path.clone(),
            kind,
            size,
            depth,
            modified,
            hidden,
            #[cfg(unix)]
            mode: Some(std::os::unix::fs::PermissionsExt::mode(&meta.permissions())),
        });

        if kind == Kind::File {
            if let Some(min) = cfg.min_size {
                if size < min {
                    hidden_by_min_size += 1;
                    hidden_min_size_total += size;
                    continue;
                }
            }
        }

        let node = match kind {
            Kind::Dir => {
                if cfg.max_depth.is_none_or(|max| depth < max) {
                    walk(&path, child_name, depth + 1, cfg, entries, stats, progress)
                } else {
                    truncated = true;
                    TreeNode {
                        name: child_name,
                        kind,
                        own_size: 0,
                        total_size: 0,
                        item_count: 0,
                        truncated: true,
                        hidden_by_min_size: 0,
                        children: Vec::new(),
                    }
                }
            }
            _ => TreeNode {
                name: child_name,
                kind,
                own_size: size,
                total_size: size,
                item_count: 0,
                truncated: false,
                hidden_by_min_size: 0,
                children: Vec::new(),
            },
        };
        children.push(node);
    }

    children.sort_by(|a, b| match cfg.sort {
        crate::model::SortOrder::Name => {
            let a_dir = a.kind == Kind::Dir;
            let b_dir = b.kind == Kind::Dir;
            b_dir
                .cmp(&a_dir)
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        }
        crate::model::SortOrder::Size => b.total_size.cmp(&a.total_size),
    });

    let total_size = children.iter().map(|c| c.total_size).sum::<u64>() + hidden_min_size_total;
    let item_count = children.iter().map(|c| 1 + c.item_count).sum::<usize>() + hidden_by_min_size;

    TreeNode {
        name,
        kind: Kind::Dir,
        own_size: 0,
        total_size,
        item_count,
        truncated,
        hidden_by_min_size,
        children,
    }
}

fn empty_dir_node(name: String) -> TreeNode {
    TreeNode {
        name,
        kind: Kind::Dir,
        own_size: 0,
        total_size: 0,
        item_count: 0,
        truncated: false,
        hidden_by_min_size: 0,
        children: Vec::new(),
    }
}

fn passes_file_filters(path: &Path, modified: Option<SystemTime>, cfg: &Config) -> bool {
    let ext = extension_of(path);
    if !cfg.include_ext.is_empty() && !cfg.include_ext.contains(&ext) {
        return false;
    }
    if cfg.exclude_ext.contains(&ext) {
        return false;
    }
    if let Some(newer) = cfg.newer_than
        && modified.is_none_or(|m| m < newer)
    {
        return false;
    }
    if let Some(older) = cfg.older_than
        && modified.is_none_or(|m| m > older)
    {
        return false;
    }
    true
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
    kind: Kind,
    size: u64,
    hidden: bool,
    depth: usize,
    path: &Path,
    meta: &Metadata,
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
    let _ = meta;
}
