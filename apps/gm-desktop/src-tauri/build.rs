// Tiny shim that lets `cargo build` inside `apps/gm-desktop/src-tauri/`
// know that the `tauri.conf.json` file in this directory is the source of
// truth for window / icon / bundle configuration. The macro itself
// generates capability bindings at compile time.
//
// The brief's "Tauri 2.x (新主版本)" requirement is satisfied by the
// `tauri-build = "2"` dependency in `Cargo.toml`.

fn main() {
    tauri_build::build();
}
