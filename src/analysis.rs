use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::model::{Entry, Kind, TreeNode};
use crate::util::extension_of;

pub fn category_of(ext: &str) -> &'static str {
    match ext {
        "rs" | "py" | "js" | "ts" | "jsx" | "tsx" | "mjs" | "go" | "java" | "kt" | "c" | "h"
        | "cpp" | "hpp" | "cc" | "cs" | "rb" | "php" | "swift" | "sh" | "bash" | "ps1" | "bat"
        | "cmd" | "html" | "htm" | "css" | "scss" | "sass" | "sql" => "Code",
        "json" | "toml" | "yaml" | "yml" | "xml" | "ini" | "cfg" | "lock" => "Config",
        "md" | "markdown" | "txt" | "rst" | "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt"
        | "pptx" | "rtf" | "odt" => "Documents",
        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "svg" | "webp" | "ico" | "tiff" => "Images",
        "mp4" | "mkv" | "avi" | "mov" | "wmv" | "flv" | "webm" => "Video",
        "mp3" | "wav" | "flac" | "aac" | "ogg" | "m4a" => "Audio",
        "zip" | "tar" | "gz" | "rar" | "7z" | "bz2" | "xz" => "Archives",
        "exe" | "dll" | "so" | "dylib" | "bin" => "Binaries",
        _ => "Other",
    }
}

pub fn file_icon_for(ext: &str) -> &'static str {
    match ext {
        "rs" => "🦀 ",
        "py" => "🐍 ",
        "js" | "ts" | "jsx" | "tsx" | "mjs" => "📜 ",
        "go" => "🐹 ",
        "java" | "kt" => "☕ ",
        "c" | "h" | "cpp" | "hpp" | "cc" => "🔧 ",
        "html" | "htm" => "🌐 ",
        "css" | "scss" | "sass" => "🎨 ",
        "json" | "toml" | "yaml" | "yml" | "xml" | "ini" | "cfg" => "⚙️ ",
        "md" | "markdown" | "txt" | "rst" => "📝 ",
        "pdf" => "📕 ",
        "zip" | "tar" | "gz" | "rar" | "7z" | "bz2" | "xz" => "📦 ",
        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "svg" | "webp" | "ico" | "tiff" => "🖼️ ",
        "mp4" | "mkv" | "avi" | "mov" | "wmv" | "flv" | "webm" => "🎬 ",
        "mp3" | "wav" | "flac" | "aac" | "ogg" | "m4a" => "🎵 ",
        "sh" | "bash" | "ps1" | "bat" | "cmd" => "🖥️ ",
        "sql" | "db" | "sqlite" => "🗄️ ",
        "lock" => "🔒 ",
        "exe" | "dll" | "so" | "dylib" => "⚡ ",
        _ => "📄 ",
    }
}

pub fn compute_categories(entries: &[Entry]) -> Vec<(&'static str, u64, u64)> {
    let mut map: HashMap<&'static str, (u64, u64)> = HashMap::new();
    for entry in entries.iter().filter(|e| e.kind == Kind::File) {
        let category = category_of(&extension_of(&entry.path));
        let row = map.entry(category).or_insert((0, 0));
        row.0 += 1;
        row.1 += entry.size;
    }
    let mut result: Vec<_> = map
        .into_iter()
        .map(|(cat, (count, size))| (cat, count, size))
        .collect();
    result.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| a.0.cmp(b.0)));
    result
}

pub fn top_dirs(tree: &TreeNode, root: &Path, top_n: usize) -> Vec<(PathBuf, u64, usize)> {
    let mut dirs = Vec::new();
    collect_dirs(tree, root, &mut dirs);
    dirs.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    dirs.truncate(top_n);
    dirs
}

fn collect_dirs(node: &TreeNode, prefix: &Path, output: &mut Vec<(PathBuf, u64, usize)>) {
    for child in &node.children {
        if child.kind == Kind::Dir {
            let path = prefix.join(&child.name);
            output.push((path.clone(), child.total_size, child.item_count));
            collect_dirs(child, &path, output);
        }
    }
}
