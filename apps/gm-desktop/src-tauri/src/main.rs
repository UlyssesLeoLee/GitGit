//! `gm-desktop` binary entrypoint — defers to the library's `run()`.

#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

fn main() {
    gm_desktop_lib::run();
}
