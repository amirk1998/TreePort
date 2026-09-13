use std::path::Path;

use crate::model::Entry;
use crate::util::{bidi_safe, format_time, human_size, relative_display};

pub fn render(root: &Path, entries: &[Entry]) -> String {
    let mut out = String::new();
    out.push_str(&format!("COMPLETE REPORT — {}\n", root.display()));
    out.push_str(&format!(
        "Generated: {}\n",
        format_time(Some(std::time::SystemTime::now()))
    ));
    out.push_str(&"=".repeat(100));
    out.push('\n');
    out.push_str(&format!(
        "{:<5} {:>12} {:<20} {:<9} {}\n",
        "TYPE", "SIZE", "MODIFIED", "PERMS", "PATH"
    ));
    out.push_str(&"-".repeat(100));
    out.push('\n');
    for entry in entries {
        let indent = "  ".repeat(entry.depth);
        let name = relative_display(root, &entry.path);
        let size = if entry.kind == crate::model::Kind::File {
            human_size(entry.size)
        } else {
            "-".to_string()
        };
        #[cfg(unix)]
        let perms = crate::util::format_mode(entry.mode);
        #[cfg(not(unix))]
        let perms = "n/a".to_string();
        out.push_str(&format!(
            "{:<5} {:>12} {:<20} {:<9} {}{}\n",
            entry.kind,
            size,
            format_time(entry.modified),
            perms,
            indent,
            bidi_safe(&name)
        ));
    }
    out
}
