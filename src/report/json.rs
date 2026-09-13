use std::path::Path;
use std::time::{Duration, SystemTime};

use crate::analysis::compute_categories;
use crate::cli::Config;
use crate::duplicate::{DuplicateResult, relative_paths};
use crate::model::{Entry, Kind, Stats};
use crate::util::format_time;

pub fn render(
    cfg: &Config,
    stats: &Stats,
    entries: &[Entry],
    duplicates: Option<&DuplicateResult>,
    elapsed: Duration,
) -> String {
    let rel = |p: &Path| p.strip_prefix(&cfg.root).unwrap_or(p).display().to_string();
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

    let cats = compute_categories(entries);
    out.push_str("  \"by_category\": [\n");
    for (i, (cat, count, size)) in cats.iter().enumerate() {
        out.push_str(&format!(
            "    {{ \"category\": {}, \"count\": {count}, \"total_size_bytes\": {size} }}{}\n",
            json_str(cat),
            if i + 1 < cats.len() { "," } else { "" }
        ));
    }
    out.push_str("  ],\n");

    let mut ext: Vec<_> = stats.by_extension.iter().collect();
    ext.sort_by(|a, b| b.1.1.cmp(&a.1.1).then_with(|| a.0.cmp(b.0)));
    out.push_str("  \"by_extension\": [\n");
    for (i, (name, (count, size))) in ext.iter().enumerate() {
        out.push_str(&format!(
            "    {{ \"extension\": {}, \"count\": {count}, \"total_size_bytes\": {size} }}{}\n",
            json_str(name),
            if i + 1 < ext.len() { "," } else { "" }
        ));
    }
    out.push_str("  ],\n");

    out.push_str("  \"entries\": [\n");
    for (i, e) in entries.iter().enumerate() {
        let kind = match e.kind {
            Kind::File => "file",
            Kind::Dir => "dir",
            Kind::Symlink => "symlink",
            Kind::Other => "other",
        };
        out.push_str(&format!("    {{ \"path\": {}, \"kind\": {}, \"size_bytes\": {}, \"depth\": {}, \"modified\": {}, \"hidden\": {} }}{}\n", json_str(&rel(&e.path)), json_str(kind), e.size, e.depth, json_str(&format_time(e.modified)), e.hidden, if i + 1 < entries.len() { "," } else { "" }));
    }
    out.push_str("  ]");

    if let Some(result) = duplicates {
        out.push_str(",\n  \"duplicates\": {\n");
        out.push_str(&format!(
            "    \"skipped_too_large\": {},\n",
            result.skipped_too_large
        ));
        out.push_str("    \"groups\": [\n");
        for (gi, group) in result.groups.iter().enumerate() {
            let size = entries[group[0]].size;
            out.push_str(&format!("      {{ \"size_bytes\": {size}, \"files\": ["));
            for (fi, entry) in relative_paths(group, entries).enumerate() {
                out.push_str(&json_str(&rel(&entry.path)));
                if fi + 1 < group.len() {
                    out.push_str(", ");
                }
            }
            out.push_str(&format!(
                "] }}{}\n",
                if gi + 1 < result.groups.len() {
                    ","
                } else {
                    ""
                }
            ));
        }
        out.push_str("    ]\n  }\n");
    } else {
        out.push('\n');
    }
    out.push_str("}\n");
    out
}

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
