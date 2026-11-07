#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

/// Windows: is a standard handle connected to an anonymous pipe (not a console
/// or an invalid handle)? The browser launches native messaging hosts with
/// stdin/stdout pipes; a user double-click does not.
#[cfg(windows)]
mod win_host_detect {
    #[link(name = "kernel32")]
    extern "system" {
        fn GetFileType(h_file: *mut core::ffi::c_void) -> u32;
    }

    const FILE_TYPE_PIPE: u32 = 0x0003;

    pub fn stdio_is_pipe() -> bool {
        use std::os::windows::io::AsRawHandle;
        let stdin = std::io::stdin();
        let stdout = std::io::stdout();
        let (i, o) = (stdin.as_raw_handle(), stdout.as_raw_handle());
        // SAFETY: raw handles are valid for the lifetime of the stdin/stdout locks.
        unsafe { GetFileType(i) == FILE_TYPE_PIPE && GetFileType(o) == FILE_TYPE_PIPE }
    }
}

fn launched_as_host() -> bool {
    let args = std::env::args().skip(1);
    for a in args {
        // Chrome passes --parent-window=<hwnd>; Firefox passes -parent-window.
        if a == "--host" || a.starts_with("--parent-window") || a.starts_with("-parent-window") {
            return true;
        }
    }
    #[cfg(windows)]
    {
        // Both browsers give the host stdio pipes. This also covers launchers
        // that pass no arguments at all.
        win_host_detect::stdio_is_pipe()
    }
    #[cfg(not(windows))]
    {
        false
    }
}

fn main() {
    if launched_as_host() {
        lilac_host::run_host();
        return;
    }
    orchard_lib::run()
}
