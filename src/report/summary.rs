use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::analysis::compute_categories;
use crate::duplicate::{wasted_space, DuplicateResult, DUP_HASH_SIZE_CAP, relative_paths};
use crate::model::{Entry, Kind, Stats};
use crate::util::{bidi_safe, format_time, human_size, relative_display};

pub fn render_detailed(root: &Path, entries: &[Entry]) -> String { super::detailed::render(root, entries) }

pub fn render_summary(root: &Path, stats: &Stats, entries: &[Entry], dirs: &[(PathBuf, u64, usize)], top_n: usize, elapsed: Duration, duplicates: Option<&DuplicateResult>) -> String {
    let mut out = String::new();
    out.push_str(&format!("SUMMARY REPORT — {}\n", root.display()));
    out.push_str(&"=".repeat(70)); out.push('\n');
    out.push_str(&format!("Total entries scanned : {}\n", stats.total_entries()));
    out.push_str(&format!("  Files               : {}\n", stats.file_count));
    out.push_str(&format!("  Directories         : {}\n", stats.dir_count));
    out.push_str(&format!("  Symlinks            : {}\n", stats.symlink_count));
    if stats.other_count > 0 { out.push_str(&format!("  Other (sockets/etc) : {}\n", stats.other_count)); }
    out.push_str(&format!("Total size of files   : {} ({} bytes)\n", human_size(stats.total_size), stats.total_size));
    out.push_str(&format!("Hidden entries        : {}\n", stats.hidden_count));
    out.push_str(&format!("Empty files           : {}\n", stats.empty_file_count));
    out.push_str(&format!("Empty directories     : {}\n", stats.empty_dir_count));
    if stats.excluded_count > 0 { out.push_str(&format!("Excluded entries      : {}\n", stats.excluded_count)); }
    out.push_str(&format!("Max tree depth        : {}\n", stats.max_depth_seen));
    if stats.file_count > 0 { out.push_str(&format!("Average file size     : {}\n", human_size((stats.total_size as f64 / stats.file_count as f64).round() as u64))); }
    out.push_str(&format!("Scan duration         : {:.1?}\n", elapsed));

    out.push_str("\nBreakdown by category\n"); out.push_str(&"-".repeat(70)); out.push('\n');
    out.push_str(&format!("{:<14} {:>10} {:>15}\n", "CATEGORY", "COUNT", "TOTAL SIZE"));
    for (cat, count, size) in compute_categories(entries) { out.push_str(&format!("{:<14} {:>10} {:>15}\n", cat, count, human_size(size))); }

    out.push_str("\nBreakdown by extension (top 15 by total size)\n"); out.push_str(&"-".repeat(70)); out.push('\n');
    out.push_str(&format!("{:<18} {:>10} {:>15}\n", "EXTENSION", "COUNT", "TOTAL SIZE"));
    let mut ext: Vec<_> = stats.by_extension.iter().collect();
    ext.sort_by(|a, b| b.1.1.cmp(&a.1.1).then_with(|| a.0.cmp(b.0)));
    for (name, (count, size)) in ext.iter().take(15) { out.push_str(&format!("{:<18} {:>10} {:>15}\n", name, count, human_size(*size))); }

    out.push_str(&format!("\nLargest {top_n} files\n")); out.push_str(&"-".repeat(70)); out.push('\n');
    let mut files: Vec<&Entry> = entries.iter().filter(|e| e.kind == Kind::File).collect();
    files.sort_by(|a,b| b.size.cmp(&a.size).then_with(|| a.path.cmp(&b.path)));
    for file in files.iter().take(top_n) { out.push_str(&format!("{:>12}  {}\n", human_size(file.size), bidi_safe(&relative_display(root, &file.path)))); }

    if !dirs.is_empty() {
        out.push_str(&format!("\nLargest {top_n} directories\n")); out.push_str(&"-".repeat(70)); out.push('\n');
        for (path, size, count) in dirs { out.push_str(&format!("{:>12}  {}  ({count} items)\n", human_size(*size), bidi_safe(&relative_display(root, path)))); }
    }

    let mut timeline: Vec<&Entry> = entries.iter().filter(|e| e.kind == Kind::File && e.modified.is_some()).collect();
    timeline.sort_by_key(|e| e.modified);
    if let Some(oldest) = timeline.first() { out.push_str(&format!("\nOldest file: {} ({})\n", bidi_safe(&relative_display(root, &oldest.path)), format_time(oldest.modified))); }
    if let Some(newest) = timeline.last() { out.push_str(&format!("Newest file: {} ({})\n", bidi_safe(&relative_display(root, &newest.path)), format_time(newest.modified))); }

    if let Some(result) = duplicates {
        out.push_str("\nDuplicate files\n"); out.push_str(&"-".repeat(70)); out.push('\n');
        if result.groups.is_empty() { out.push_str("No duplicate files found.\n"); }
        else {
            let total_wasted: u64 = result.groups.iter().map(|g| wasted_space(g, entries)).sum();
            out.push_str(&format!("{} duplicate group(s) found — approx. {} wasted\n", result.groups.len(), human_size(total_wasted)));
            for (i, group) in result.groups.iter().take(10).enumerate() {
                out.push_str(&format!("  Group {}: {} x{} copies\n", i + 1, human_size(entries[group[0]].size), group.len()));
                for entry in relative_paths(group, entries) { out.push_str(&format!("    - {}\n", bidi_safe(&relative_display(root, &entry.path)))); }
            }
            if result.groups.len() > 10 { out.push_str(&format!("  ... and {} more group(s) (see markdown/full report)\n", result.groups.len() - 10)); }
        }
        if result.skipped_too_large > 0 { out.push_str(&format!("({} file(s) skipped — larger than {})\n", result.skipped_too_large, human_size(DUP_HASH_SIZE_CAP))); }
    }

    if !stats.errors.is_empty() {
        out.push_str(&format!("\nWarnings ({} entries could not be read)\n", stats.errors.len()));
        out.push_str(&"-".repeat(70)); out.push('\n');
        for error in &stats.errors { out.push_str(&format!("  - {error}\n")); }
    }
    out
}
