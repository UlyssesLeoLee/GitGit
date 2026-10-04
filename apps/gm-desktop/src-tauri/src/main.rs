//! `gm-desktop` binary entrypoint — defers to the library's `run()`.

#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

fn main() {
    // The release build sets `windows_subsystem = "windows"`, so there is
    // no console to print to. The message still goes to stderr for debug
    // builds, and the non-zero exit is the part that is always visible:
    // a shell that could not build its own context has nothing to run.
    if let Err(e) = gm_desktop_lib::run() {
        eprintln!("gitgit desktop failed to start: {e}");
        std::process::exit(1);
    }
}
