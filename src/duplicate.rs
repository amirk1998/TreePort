use std::collections::{hash_map::DefaultHasher, HashMap};
use std::fs::File;
use std::hash::Hasher;
use std::io::{self, BufReader, Read};
use std::path::Path;

use crate::model::{Entry, Kind};

pub const DUP_HASH_SIZE_CAP: u64 = 512 * 1024 * 1024;

#[derive(Debug, Default)]
pub struct DuplicateResult {
    pub groups: Vec<Vec<usize>>,
    pub skipped_too_large: usize,
}

pub fn find_duplicates(entries: &[Entry]) -> DuplicateResult {
    let mut by_size: HashMap<u64, Vec<usize>> = HashMap::new();
    for (index, entry) in entries.iter().enumerate() {
        if entry.kind == Kind::File && entry.size > 0 {
            by_size.entry(entry.size).or_default().push(index);
        }
    }

    let mut result = DuplicateResult::default();
    for (size, candidates) in by_size {
        if candidates.len() < 2 { continue; }
        if size > DUP_HASH_SIZE_CAP {
            result.skipped_too_large += candidates.len();
            continue;
        }
        let mut by_hash: HashMap<u64, Vec<usize>> = HashMap::new();
        for index in candidates {
            if let Some(hash) = hash_file(&entries[index].path) {
                by_hash.entry(hash).or_default().push(index);
            }
        }
        for group in by_hash.into_values() {
            if group.len() > 1 { result.groups.push(group); }
        }
    }

    result.groups.sort_by(|a, b| {
        let aw = wasted_space(a, entries);
        let bw = wasted_space(b, entries);
        bw.cmp(&aw).then_with(|| a.len().cmp(&b.len()))
    });
    result
}

pub fn wasted_space(group: &[usize], entries: &[Entry]) -> u64 {
    entries[group[0]].size.saturating_mul(group.len().saturating_sub(1) as u64)
}

fn hash_file(path: &Path) -> Option<u64> {
    let file = File::open(path).ok()?;
    let mut reader = BufReader::new(file);
    let mut hasher = DefaultHasher::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let bytes_read = reader.read(&mut buffer).ok()?;
        if bytes_read == 0 { break; }
        hasher.write(&buffer[..bytes_read]);
    }
    Some(hasher.finish())
}

pub fn relative_paths<'a>(group: &'a [usize], entries: &'a [Entry]) -> impl Iterator<Item = &'a Entry> + 'a {
    group.iter().map(|&index| &entries[index])
}

#[allow(dead_code)]
fn _io_marker(_: io::Result<()>) {}
