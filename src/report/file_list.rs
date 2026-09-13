use std::path::Path;

use crate::model::{Entry, Kind};

pub fn render(root: &Path, entries: &[Entry]) -> String {
    let mut files: Vec<&Entry> = entries.iter().filter(|e| e.kind == Kind::File).collect();
    files.sort_by(|a, b| {
        let name_a = a
            .path
            .file_name()
            .map(|n| n.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let name_b = b
            .path
            .file_name()
            .map(|n| n.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let path_a = relative(root, &a.path);
        let path_b = relative(root, &b.path);
        name_a.cmp(&name_b).then_with(|| path_a.cmp(&path_b))
    });

    let mut out = String::from("File Number,File Name,Relative Path\r\n");
    for (i, entry) in files.into_iter().enumerate() {
        let name = entry
            .path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| entry.path.display().to_string());
        let rel = relative(root, &entry.path);
        out.push_str(&format!(
            "{},{},{}\r\n",
            i + 1,
            csv_escape(&name),
            csv_escape(&rel)
        ));
    }
    out
}

fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

pub fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') || s.contains('\r') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    fn entry(path: &str, kind: Kind) -> Entry {
        Entry {
            path: PathBuf::from(path),
            kind,
            size: 1,
            depth: 1,
            modified: None,
            hidden: false,
            #[cfg(unix)]
            mode: None,
        }
    }

    #[test]
    fn numbered_sorted_utf8_csv() {
        let root = Path::new("/proj");
        let entries = vec![
            entry("/proj/گزارش.md", Kind::File),
            entry("/proj/b.md", Kind::File),
            entry("/proj/sub", Kind::Dir),
            entry("/proj/a, \"quoted\".md", Kind::File),
        ];
        let csv = render(root, &entries);
        let rows: Vec<&str> = csv.split("\r\n").collect();
        assert_eq!(rows[0], "File Number,File Name,Relative Path");
        assert_eq!(
            rows[1],
            "1,\"a, \"\"quoted\"\".md\",\"a, \"\"quoted\"\".md\""
        );
        assert_eq!(rows[2], "2,b.md,b.md");
        assert!(rows[3].starts_with("3,گزارش.md,"));
        assert!(!csv.contains('\u{2068}') && !csv.contains('\u{2069}'));
    }

    #[test]
    fn tie_breaks_on_relative_path() {
        let root = Path::new("/proj");
        let entries = vec![
            entry("/proj/z/notes.md", Kind::File),
            entry("/proj/a/notes.md", Kind::File),
        ];
        let csv = render(root, &entries);
        let rows: Vec<&str> = csv.split("\r\n").collect();
        assert_eq!(rows[1], "1,notes.md,a/notes.md");
        assert_eq!(rows[2], "2,notes.md,z/notes.md");
    }
}
