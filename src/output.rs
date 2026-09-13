use std::fs;
use std::path::Path;

pub fn write_text_file(path: &Path, content: &str, bom: bool) -> Result<(), String> {
    let mut bytes = Vec::with_capacity(content.len() + usize::from(bom) * 3);
    if bom { bytes.extend_from_slice(&[0xEF, 0xBB, 0xBF]); }
    bytes.extend_from_slice(content.as_bytes());
    fs::write(path, bytes).map_err(|e| format!("failed to write {}: {}", path.display(), e))
}
