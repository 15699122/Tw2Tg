#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    #[cfg(windows)]
    if std::env::args_os()
        .skip(1)
        .eq([std::ffi::OsStr::new("--xarchive-writer-keeper")])
    {
        // The creating process gives this helper one inherited Job Object
        // handle. Recovery terminates the job and this process together.
        loop {
            std::thread::park();
        }
    }
    xarchive_desktop_lib::run();
}
