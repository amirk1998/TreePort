use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use crate::analysis::compute_categories;
use crate::cli::Config;
use crate::duplicate::{wasted_space, DuplicateResult, DUP_HASH_SIZE_CAP, relative_paths};
use crate::model::{Entry, Kind, Stats, TreeNode};
use crate::report::tree;
use crate::util::{bidi_safe, format_time, human_size, relative_display};

fn md_escape(s: &str) -> String { s.replace('|', "\\|") }

pub fn render(cfg: &Config, tree_node: &TreeNode, stats: &Stats, entries: &[Entry], dirs: &[(PathBuf, u64, usize)], duplicates: Option<&DuplicateResult>, elapsed: Duration) -> String {
    let mut out = String::new();
    let root_display = cfg.root.display().to_string();
    let rel = |p: &Path| bidi_safe(&relative_display(&cfg.root, p));

    out.push_str("# 📊 Directory Scan Report\n\n");
    out.push_str(&format!("**Path:** `{}`  \n", md_escape(&bidi_safe(&root_display))));
    out.push_str(&format!("**Generated:** {}  \n", format_time(Some(SystemTime::now()))));
    out.push_str(&format!("**Scan duration:** {elapsed:.1?}\n\n---\n\n"));

    out.push_str("## 📁 Directory Tree\n\n```text\n");
    let mut plain = cfg.clone(); plain.color = false;
    out.push_str(&tree::render(tree_node, &plain));
    out.push_str("```\n\n---\n\n## 📈 Overview\n\n| Metric | Value |\n|---|---|\n");
    out.push_str(&format!("| Total entries | {} |\n", stats.total_entries()));
    out.push_str(&format!("| Files | {} |\n", stats.file_count));
    out.push_str(&format!("| Directories | {} |\n", stats.dir_count));
    out.push_str(&format!("| Symlinks | {} |\n", stats.symlink_count));
    if stats.other_count > 0 { out.push_str(&format!("| Other (sockets/etc) | {} |\n", stats.other_count)); }
    out.push_str(&format!("| Total size | {} ({} bytes) |\n", human_size(stats.total_size), stats.total_size));
    out.push_str(&format!("| Hidden entries | {} |\n", stats.hidden_count));
    out.push_str(&format!("| Empty files | {} |\n", stats.empty_file_count));
    out.push_str(&format!("| Empty directories | {} |\n", stats.empty_dir_count));
    if stats.excluded_count > 0 { out.push_str(&format!("| Excluded entries | {} |\n", stats.excluded_count)); }
    out.push_str(&format!("| Max tree depth | {} |\n", stats.max_depth_seen));
    if stats.file_count > 0 { out.push_str(&format!("| Average file size | {} |\n", human_size((stats.total_size as f64 / stats.file_count as f64).round() as u64))); }

    out.push_str("\n---\n\n## 🗂️ Breakdown by Category\n\n| Category | Files | Total Size |\n|---|---|---|\n");
    for (cat, count, size) in compute_categories(entries) { out.push_str(&format!("| {cat} | {count} | {} |\n", human_size(size))); }

    out.push_str("\n---\n\n## 🧩 Breakdown by Extension\n\n| Extension | Count | Total Size |\n|---|---|---|\n");
    let mut ext: Vec<_> = stats.by_extension.iter().collect();
    ext.sort_by(|a, b| b.1.1.cmp(&a.1.1).then_with(|| a.0.cmp(b.0)));
    for (name, (count, size)) in ext.iter().take(15) { out.push_str(&format!("| `{}` | {count} | {} |\n", md_escape(name), human_size(*size))); }

    out.push_str(&format!("\n---\n\n## 🏆 Largest {} Files\n\n| # | Size | Path |\n|---|---|---|\n", cfg.top_n));
    let mut files: Vec<&Entry> = entries.iter().filter(|e| e.kind == Kind::File).collect();
    files.sort_by(|a,b| b.size.cmp(&a.size).then_with(|| a.path.cmp(&b.path)));
    for (i, file) in files.iter().take(cfg.top_n).enumerate() { out.push_str(&format!("| {} | {} | `{}` |\n", i + 1, human_size(file.size), md_escape(&rel(&file.path)))); }

    if !dirs.is_empty() {
        out.push_str(&format!("\n---\n\n## 🗃️ Largest {} Directories\n\n| # | Total Size | Items | Path |\n|---|---|---|---|\n", cfg.top_n));
        for (i, (path, size, count)) in dirs.iter().enumerate() { out.push_str(&format!("| {} | {} | {count} | `{}` |\n", i + 1, human_size(*size), md_escape(&rel(path)))); }
    }

    out.push_str("\n---\n\n## 🕒 Timeline\n\n");
    let mut timeline: Vec<&Entry> = entries.iter().filter(|e| e.kind == Kind::File && e.modified.is_some()).collect();
    timeline.sort_by_key(|e| e.modified);
    if let Some(oldest) = timeline.first() { out.push_str(&format!("- **Oldest file:** `{}` — {}\n", md_escape(&rel(&oldest.path)), format_time(oldest.modified))); }
    if let Some(newest) = timeline.last() { out.push_str(&format!("- **Newest file:** `{}` — {}\n", md_escape(&rel(&newest.path)), format_time(newest.modified))); }

    if let Some(result) = duplicates {
        out.push_str("\n---\n\n## 🧬 Duplicate Files\n\n");
        if result.groups.is_empty() { out.push_str("No duplicate files found. ✅\n\n"); }
        else {
            let wasted: u64 = result.groups.iter().map(|g| wasted_space(g, entries)).sum();
            out.push_str(&format!("Found **{}** group(s) of duplicate files, wasting approximately **{}** of disk space.\n\n", result.groups.len(), human_size(wasted)));
            for (i, group) in result.groups.iter().enumerate() {
                out.push_str(&format!("<details>\n<summary><strong>Group {}</strong> — {} × {} copies (wastes {})</summary>\n\n", i + 1, human_size(entries[group[0]].size), group.len(), human_size(wasted_space(group, entries))));
                for entry in relative_paths(group, entries) { out.push_str(&format!("- `{}`\n", md_escape(&rel(&entry.path)))); }
                out.push_str("\n</details>\n\n");
            }
        }
        if result.skipped_too_large > 0 { out.push_str(&format!("*Note: {} file(s) were skipped from duplicate checking for being larger than {}.*\n\n", result.skipped_too_large, human_size(DUP_HASH_SIZE_CAP))); }
    }

    if !stats.errors.is_empty() {
        out.push_str("\n---\n\n");
        out.push_str(&format!("## ⚠️ Warnings ({} entries)\n\n", stats.errors.len()));
        for error in &stats.errors { out.push_str(&format!("- {}\n", md_escape(error))); }
    }
    out.push_str("\n---\n\n*Generated by treeport — a modular, dependency-free Rust directory analyzer.*\n");
    out
}
