use super::file_list::csv_escape;
use crate::model::{Entry, Kind};
use std::path::Path;

pub fn render(root: &Path, entries: &[Entry]) -> String {
    let mut out = String::from("path,kind,size_bytes,depth,modified,hidden\n");
    for entry in entries {
        let rel = entry
            .path
            .strip_prefix(root)
            .unwrap_or(&entry.path)
            .display()
            .to_string();
        let kind = match entry.kind {
            Kind::File => "file",
            Kind::Dir => "dir",
            Kind::Symlink => "symlink",
            Kind::Other => "other",
        };
        out.push_str(&format!(
            "{},{},{},{},{},{}\n",
            csv_escape(&rel),
            kind,
            entry.size,
            entry.depth,
            csv_escape(&crate::util::format_time(entry.modified)),
            entry.hidden
        ));
    }
    out
}
