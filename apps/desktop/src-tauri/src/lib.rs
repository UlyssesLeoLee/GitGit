//! GitGit Desktop — Tauri runtime.
//!
//! Exposes a small set of `tauri::command` IPCs that the Svelte frontend
//! invokes to load the engineering knowledge graph from the local
//! `docs/requirements/` directory. The graph itself is built by walking the
//! markdown files, extracting requirement IDs (REQ-NNN, PREFIX-REQ-NNN,
//! ADR-NNNN, RGS-IMPL-NNN), and synthesising edges between them.

mod commands;
mod graph;

use commands::*;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            graph_load,
            graph_list_nodes,
            graph_list_edges,
            graph_get_node,
            graph_stats,
            docs_list,
            docs_read,
            app_platform,
            app_version,
        ])
        .run(tauri::generate_context!())
        .expect("error while running GitGit Desktop");
}