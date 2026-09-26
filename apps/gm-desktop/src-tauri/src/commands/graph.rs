//! Knowledge-graph IPC commands — ported from `apps/desktop/src-tauri/src/commands.rs`
//! per ADR-0023 (consolidate desktop apps: absorb knowledge-graph viewer into gm-desktop).
//!
//! The Svelte `/graph` route invokes these via `tauri::invoke` to walk the local
//! `docs/requirements/` directory and surface engineering-graph primitives
//! (requirement IDs, ADR refs, RGS gates, etc.) without going through the
//! gitgit HTTP server. Per ADR-0020 §2.2 the HTTP server is for repository ops;
//! graph visualization is a separate, local-only surface.

use crate::graph::{docs_to_graph, load_graph_from_repo, parse_doc, repo_docs_root};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::PathBuf;

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

fn build_docs_out(parsed: &[crate::graph::ParsedDoc]) -> Vec<DocMetaOut> {
    parsed
        .iter()
        .map(|d| DocMetaOut {
            path: d.source.clone(),
            title: d.title.clone(),
            preview: d.preview.clone(),
        })
        .collect()
}

#[tauri::command]
pub fn graph_load() -> Result<GraphLoadResult, String> {
    let root = repo_docs_root().map_err(|e| e.to_string())?;
    let (parsed, _) = load_graph_from_repo(&root).map_err(|e| e.to_string())?;
    let g = docs_to_graph(&parsed);
    let nodes = g
        .nodes
        .into_iter()
        .map(serde_json::to_value)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    let edges = g
        .edges
        .into_iter()
        .map(serde_json::to_value)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(GraphLoadResult {
        nodes,
        edges,
        docs: build_docs_out(&parsed),
    })
}

#[tauri::command]
pub fn graph_list_nodes() -> Result<Vec<serde_json::Value>, String> {
    let root = repo_docs_root().map_err(|e| e.to_string())?;
    let (parsed, _) = load_graph_from_repo(&root).map_err(|e| e.to_string())?;
    let g = docs_to_graph(&parsed);
    g.nodes
        .into_iter()
        .map(serde_json::to_value)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn graph_list_edges() -> Result<Vec<serde_json::Value>, String> {
    let root = repo_docs_root().map_err(|e| e.to_string())?;
    let (parsed, _) = load_graph_from_repo(&root).map_err(|e| e.to_string())?;
    let g = docs_to_graph(&parsed);
    g.edges
        .into_iter()
        .map(serde_json::to_value)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
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
    pub by_type: BTreeMap<String, usize>,
}

#[tauri::command]
pub fn graph_stats() -> Result<GraphStats, String> {
    let root = repo_docs_root().map_err(|e| e.to_string())?;
    let (parsed, _) = load_graph_from_repo(&root).map_err(|e| e.to_string())?;
    let g = docs_to_graph(&parsed);
    let mut by_type: BTreeMap<String, usize> = BTreeMap::new();
    for n in &g.nodes {
        *by_type.entry(n.kind.clone()).or_insert(0) += 1;
    }
    Ok(GraphStats {
        nodes: g.nodes.len(),
        edges: g.edges.len(),
        by_type,
    })
}

#[tauri::command]
pub fn docs_list() -> Result<Vec<DocMetaOut>, String> {
    let root = repo_docs_root().map_err(|e| e.to_string())?;
    let (parsed, _) = load_graph_from_repo(&root).map_err(|e| e.to_string())?;
    Ok(build_docs_out(&parsed))
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
    Ok(DocReadOut {
        path: parsed.source,
        title: parsed.title,
        content: raw,
    })
}
