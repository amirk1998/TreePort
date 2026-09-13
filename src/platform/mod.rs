use std::fs::Metadata;

#[cfg(windows)]
mod windows;

pub fn initialize_console(color: bool) {
    #[cfg(windows)]
    {
        windows::enable_utf8_console();
        if color {
            windows::enable_ansi_colors();
        }
    }
    let _ = color;
}

pub fn is_hidden(name: &str, meta: &Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
        if meta.file_attributes() & FILE_ATTRIBUTE_HIDDEN != 0 {
            return true;
        }
    }
    let _ = meta;
    name.starts_with('.')
}
