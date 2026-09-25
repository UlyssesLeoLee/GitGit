//! Tauri command handlers — bridge the Svelte frontend to the local graph.

use crate::graph::{load_graph_from_repo, parse_doc, repo_docs_root, docs_to_graph};
use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Serialize)]
pub struct AppVersion {
    pub name: String,
    pub version: String,
}

#[tauri::command]
pub fn app_version() -> AppVersion {
    AppVersion { name: env!("CARGO_PKG_NAME").to_string(), version: env!("CARGO_PKG_VERSION").to_string() }
}

#[tauri::command]
pub fn app_platform() -> String {
    std::env::consts::OS.to_string()
}

#[derive(Debug, Serialize)]
pub struct DocMetaOut {
    pub path: String,
    pub title: String,
    pub preview: String,
}

#[derive(Debug, Serialize)]
pub struct GraphLoadResult {
    pub nodes: Vec<serde_json::Value>,
    pub edges: Vec<serde_json::Value>,
    pub docs: Vec<DocMetaOut>,
}

#[tauri::command]
pub fn graph_load() -> Result<GraphLoadResult, String> {
    let root = repo_docs_root().map_err(|e| e.to_string())?;
    let (parsed, doc_metas) = load_graph_from_repo(&root).map_err(|e| e.to_string())?;
    let g = docs_to_graph(&parsed);
    let docs: Vec<DocMetaOut> = doc_metas
        .into_iter()
        .map(|d| DocMetaOut { path: d.source.clone(), title: d.title, preview: d.preview })
        .collect();
    Ok(GraphLoadResult {
        nodes: g.nodes.into_iter().map(serde_json::to_value).collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?,
        edges: g.edges.into_iter().map(serde_json::to_value).collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?,
        docs,
    })
}

#[tauri::command]
pub fn graph_list_nodes() -> Result<Vec<serde_json::Value>, String> {
    let root = repo_docs_root().map_err(|e| e.to_string())?;
    let (parsed, _) = load_graph_from_repo(&root).map_err(|e| e.to_string())?;
    let g = docs_to_graph(&parsed);
    g.nodes.into_iter().map(serde_json::to_value).collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn graph_list_edges() -> Result<Vec<serde_json::Value>, String> {
    let root = repo_docs_root().map_err(|e| e.to_string())?;
    let (parsed, _) = load_graph_from_repo(&root).map_err(|e| e.to_string())?;
    let g = docs_to_graph(&parsed);
    g.edges.into_iter().map(serde_json::to_value).collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn graph_get_node(id: String) -> Result<Option<serde_json::Value>, String> {
    let root = repo_docs_root().map_err(|e| e.to_string())?;
    let (parsed, _) = load_graph_from_repo(&root).map_err(|e| e.to_string())?;
    let g = docs_to_graph(&parsed);
    Ok(g.nodes.into_iter().find(|n| n.id == id).map(|n| serde_json::to_value(n).unwrap()))
}

#[derive(Debug, Serialize)]
pub struct GraphStats {
    pub nodes: usize,
    pub edges: usize,
    pub by_type: std::collections::BTreeMap<String, usize>,
}

#[tauri::command]
pub fn graph_stats() -> Result<GraphStats, String> {
    let root = repo_docs_root().map_err(|e| e.to_string())?;
    let (parsed, _) = load_graph_from_repo(&root).map_err(|e| e.to_string())?;
    let g = docs_to_graph(&parsed);
    let mut by_type = std::collections::BTreeMap::new();
    for n in &g.nodes {
        *by_type.entry(n.kind.clone()).or_insert(0) += 1;
    }
    Ok(GraphStats { nodes: g.nodes.len(), edges: g.edges.len(), by_type })
}

#[tauri::command]
pub fn docs_list() -> Result<Vec<DocMetaOut>, String> {
    let root = repo_docs_root().map_err(|e| e.to_string())?;
    let (_, metas) = load_graph_from_repo(&root).map_err(|e| e.to_string())?;
    Ok(metas.into_iter().map(|d| DocMetaOut { path: d.source.clone(), title: d.title, preview: d.preview }).collect())
}

#[derive(Debug, Serialize)]
pub struct DocReadOut {
    pub path: String,
    pub title: String,
    pub content: String,
}

#[tauri::command]
pub fn docs_read(path: String) -> Result<DocReadOut, String> {
    let root = repo_docs_root().map_err(|e| e.to_string())?;
    let pb: PathBuf = root.join(&path);
    let raw = std::fs::read_to_string(&pb).map_err(|e| format!("read {}: {}", pb.display(), e))?;
    let parsed = parse_doc(&path, &raw);
    Ok(DocReadOut { path: parsed.source, title: parsed.title, content: raw })
}

// `kind_of` is used inside `docs_to_graph` (exposed for the smoke script via
// the public API) — kept here so re-exports stay tree-shake-friendly.
#[allow(dead_code)]
fn _kind_of_marker() {}