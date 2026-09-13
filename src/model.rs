use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use std::time::SystemTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortOrder {
    Name,
    Size,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    File,
    Dir,
    Symlink,
    Other,
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Kind::File => "FILE",
            Kind::Dir => "DIR ",
            Kind::Symlink => "LINK",
            Kind::Other => "OTHR",
        };
        write!(f, "{value}")
    }
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub path: PathBuf,
    pub kind: Kind,
    pub size: u64,
    pub depth: usize,
    pub modified: Option<SystemTime>,
    pub hidden: bool,
    #[cfg(unix)]
    pub mode: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct TreeNode {
    pub name: String,
    pub kind: Kind,
    pub own_size: u64,
    pub total_size: u64,
    pub item_count: usize,
    pub truncated: bool,
    pub hidden_by_min_size: usize,
    pub children: Vec<TreeNode>,
}

#[derive(Debug, Default)]
pub struct Stats {
    pub file_count: u64,
    pub dir_count: u64,
    pub symlink_count: u64,
    pub other_count: u64,
    pub total_size: u64,
    pub hidden_count: u64,
    pub empty_file_count: u64,
    pub empty_dir_count: u64,
    pub excluded_count: u64,
    pub max_depth_seen: usize,
    pub by_extension: HashMap<String, (u64, u64)>,
    pub errors: Vec<String>,
}

impl Stats {
    pub fn total_entries(&self) -> u64 {
        self.file_count + self.dir_count + self.symlink_count + self.other_count
    }
}
