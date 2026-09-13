#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetStdHandle(nStdHandle: i32) -> *mut std::ffi::c_void;
    fn GetConsoleMode(hConsoleHandle: *mut std::ffi::c_void, lpMode: *mut u32) -> i32;
    fn SetConsoleMode(hConsoleHandle: *mut std::ffi::c_void, dwMode: u32) -> i32;
    fn SetConsoleOutputCP(wCodePageID: u32) -> i32;
    fn SetConsoleCP(wCodePageID: u32) -> i32;
}

const STD_OUTPUT_HANDLE: i32 = -11;
const ENABLE_VIRTUAL_TERMINAL_PROCESSING: u32 = 0x0004;
const CP_UTF8: u32 = 65001;

pub fn enable_ansi_colors() {
    unsafe {
        let handle = GetStdHandle(STD_OUTPUT_HANDLE);
        if handle.is_null() {
            return;
        }
        let mut mode = 0u32;
        if GetConsoleMode(handle, &mut mode) == 0 {
            return;
        }
        let _ = SetConsoleMode(handle, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING);
    }
}

pub fn enable_utf8_console() {
    unsafe {
        let _ = SetConsoleOutputCP(CP_UTF8);
        let _ = SetConsoleCP(CP_UTF8);
    }
}
