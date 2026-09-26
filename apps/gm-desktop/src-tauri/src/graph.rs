//! Local graph construction — walks the repo's `docs/requirements/` and
//! extracts engineering-graph primitives.

use anyhow::{anyhow, Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedDoc {
    pub source: String,
    pub title: String,
    pub preview: String,
    pub raw: String,
    pub requirement_ids: Vec<String>,
    pub adr_ids: Vec<String>,
    pub rgs_ids: Vec<String>,
    pub sections: Vec<SectionMeta>,
    pub tag_counts: TagCounts,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionMeta {
    pub number: i32,
    pub title: String,
    pub preview: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TagCounts {
    pub tbd: usize,
    pub phases: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub kind: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub id: String,
    pub kind: String,
    pub from: String,
    pub to: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    pub created_at: String,
}

/// Locate the repo root from the running binary. Tauri sets the working dir to
/// the app folder; the desktop app expects a sibling `../../docs/requirements`
/// (the GitGit repo). If missing, fall back to `./docs/requirements` next to
/// the executable, then to `~/GitGit/docs/requirements`.
///
/// `docs/requirements/` was archived to `docs_archive_rust_impl_2026_08_26/requirements/`
/// on dev (commit c822a60, 2026-08-26) as part of the 14-crate → single-crate MVP
/// rewrite, so that path is checked as a fallback too.
pub fn repo_docs_root() -> Result<PathBuf> {
    let candidates = [
        // explicit override takes priority over every path guess
        std::env::var_os("GITGIT_DOCS").map(PathBuf::from),
        // when the app sits at apps/desktop/src-tauri/target/debug/...
        std::env::current_dir().ok().map(|d| d.join("../../../docs/requirements")),
        std::env::current_dir().ok().map(|d| d.join("../../../docs_archive_rust_impl_2026_08_26/requirements")),
        // when packaged
        std::env::current_exe().ok().and_then(|p| p.parent().map(|p| p.join("docs/requirements"))),
        // user home fallback
        std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))
            .map(|h| PathBuf::from(h).join("GitGit/docs/requirements")),
    ];
    for c in candidates.into_iter().flatten() {
        if c.exists() {
            return Ok(c.canonicalize().unwrap_or(c));
        }
    }
    Err(anyhow!("could not locate docs/requirements — set GITGIT_DOCS env var to the requirements folder"))
}

pub fn load_graph_from_repo(root: &Path) -> Result<(Vec<ParsedDoc>, Vec<ParsedDoc>)> {
    let mut docs: Vec<ParsedDoc> = Vec::new();
    for entry in WalkDir::new(root).max_depth(1).into_iter().filter_map(Result::ok) {
        let path = entry.path();
        if path.is_file() && path.extension().map(|s| s == "md").unwrap_or(false) {
            let raw = std::fs::read_to_string(path)
                .with_context(|| format!("read {}", path.display()))?;
            let rel = path.strip_prefix(root).unwrap_or(path).to_string_lossy().replace('\\', "/");
            let parsed = parse_doc(&rel, &raw);
            docs.push(parsed);
        }
    }
    // Stable sort by source path so graph layout is deterministic across runs.
    docs.sort_by(|a, b| a.source.cmp(&b.source));
    let metas = docs.clone();
    Ok((docs, metas))
}

pub fn parse_doc(source: &str, raw: &str) -> ParsedDoc {
    let req_re   = Regex::new(r"(?:[A-Z]{2,5}-)?REQ-(\d{3})(?:-(SEQ|MVP))?").unwrap();
    let adr_re   = Regex::new(r"\bADR-(\d{4})\b").unwrap();
    let rgs_re   = Regex::new(r"\bRGS-IMPL-(\d{3})\b").unwrap();
    let sec_re   = Regex::new(r"^##\s+(\d+)\.\s+(.+?)\s*$").unwrap();
    let h1_re    = Regex::new(r"^#\s+(.+?)\s*$").unwrap();
    let phase_re = Regex::new(r"Phase\s+(\d{1,2})").unwrap();
    let tbd_re   = Regex::new(r"\bTBD\b").unwrap();

    let mut title = source.split('/').last().unwrap_or(source).to_string();
    for line in raw.lines() {
        if let Some(c) = h1_re.captures(line) { title = c.get(1).unwrap().as_str().to_string(); break; }
    }
    let title = title;

    let mut reqs = BTreeSet::new();
    let mut adrs = BTreeSet::new();
    let mut rgss = BTreeSet::new();
    let mut phases = BTreeSet::new();
    let mut tbd = 0usize;
    for line in raw.lines() {
        for c in req_re.captures_iter(line) {
            let canonical = c.get(0).unwrap().as_str()
                .trim_end_matches("-SEQ").trim_end_matches("-MVP")
                .to_string();
            reqs.insert(canonical);
        }
        for c in adr_re.captures_iter(line) { adrs.insert(format!("ADR-{}", &c[1])); }
        for c in rgs_re.captures_iter(line) { rgss.insert(format!("RGS-IMPL-{}", &c[1])); }
        for c in phase_re.captures_iter(line) { phases.insert(c[1].parse::<u32>().unwrap_or(0)); }
        tbd += tbd_re.find_iter(line).count();
    }

    let mut sections = Vec::new();
    for (i, line) in raw.lines().enumerate() {
        if let Some(c) = sec_re.captures(line) {
            let preview = raw.lines().skip(i + 1).take(5).collect::<Vec<_>>().join(" ")
                .replace(|ch: char| matches!(ch, '#' | '*' | '_' | '`' | '>'), "")
                .trim()
                .chars().take(240).collect::<String>();
            sections.push(SectionMeta {
                number: c[1].parse().unwrap_or(0),
                title: c[2].trim().to_string(),
                preview,
            });
        }
    }

    let preview = raw.chars().take(240).collect::<String>();
    ParsedDoc {
        source: source.to_string(),
        title,
        preview,
        raw: raw.to_string(),
        requirement_ids: reqs.into_iter().collect(),
        adr_ids:         adrs.into_iter().collect(),
        rgs_ids:         rgss.into_iter().collect(),
        sections,
        tag_counts: TagCounts { tbd, phases: phases.into_iter().collect() },
    }
}

pub fn kind_of(id: &str) -> String {
    if id.starts_with("ADR-") { return "adr".into(); }
    if id.starts_with("RGS-IMPL-") { return "adr".into(); }
    if let Some(caps) = Regex::new(r"^([A-Z]{2,5})-REQ-(\d{3})").unwrap().captures(id) {
        let prefix = caps.get(1).unwrap().as_str();
        return match prefix {
            "OPS" | "UX" | "CDX" | "CI" | "NFR" | "SEC" => "policy",
            "AGT" | "AI" => "agent",
            "CTX" => "document",
            _ => "requirement",
        }.into();
    }
    if Regex::new(r"^REQ-\d{3}$").unwrap().is_match(id) { return "requirement".into(); }
    "document".into()
}

pub fn docs_to_graph(parsed: &[ParsedDoc]) -> GraphPair {
    let now = chrono::Utc::now().to_rfc3339();
    let mut nodes: HashMap<String, GraphNode> = HashMap::new();
    let mut edges: Vec<GraphEdge> = Vec::new();
    let mut seen_edges: HashSet<String> = HashSet::new();
    let mut eid = 0u64;

    let mut upsert = |n: GraphNode| {
        nodes.entry(n.id.clone())
            .and_modify(|existing| {
                let mut tag_bset: BTreeSet<String> = existing.tags.iter().cloned().collect();
                for t in &n.tags { tag_bset.insert(t.clone()); }
                existing.tags = tag_bset.into_iter().collect();
                if existing.body.is_none() { existing.body = n.body.clone(); }
                if existing.source.is_none() { existing.source = n.source.clone(); }
            })
            .or_insert(n);
    };

    // 1) Doc nodes + requirement / ADR nodes + reference edges from doc→req
    for d in parsed {
        let doc_id = format!("DOC:{}", d.source);
        upsert(GraphNode {
            id: doc_id.clone(),
            kind: "document".into(),
            title: d.title.clone(),
            body: Some(d.preview.chars().take(240).collect()),
            tags: vec![format!("phases:{}", d.tag_counts.phases.iter().map(u32::to_string).collect::<Vec<_>>().join(","))],
            source: Some(d.source.clone()),
            created_at: now.clone(),
            updated_at: now.clone(),
        });
        for r in &d.requirement_ids {
            upsert(GraphNode {
                id: r.clone(),
                kind: kind_of(r),
                title: format!("Requirement {}", r),
                tags: vec![],
                source: Some(d.source.clone()),
                created_at: now.clone(),
                updated_at: now.clone(),
                body: None,
            });
            let k = format!("references|{}|{}", doc_id, r);
            if seen_edges.insert(k.clone()) {
                edges.push(GraphEdge {
                    id: format!("e{}", { eid += 1; eid }),
                    kind: "references".into(),
                    from: doc_id.clone(),
                    to: r.clone(),
                    weight: Some(1.0),
                    note: Some("defined in".into()),
                    created_at: now.clone(),
                });
            }
        }
        for a in &d.adr_ids {
            upsert(GraphNode {
                id: a.clone(),
                kind: "adr".into(),
                title: format!("ADR {}", a),
                tags: vec![],
                source: Some(d.source.clone()),
                created_at: now.clone(),
                updated_at: now.clone(),
                body: None,
            });
            let k = format!("references|{}|{}", doc_id, a);
            if seen_edges.insert(k.clone()) {
                edges.push(GraphEdge {
                    id: format!("e{}", { eid += 1; eid }),
                    kind: "references".into(),
                    from: doc_id.clone(),
                    to: a.clone(),
                    weight: Some(1.0),
                    note: Some("defined in".into()),
                    created_at: now.clone(),
                });
            }
        }
    }

    // 2) Cross-doc references
    let ref_re = Regex::new(r"\(\.{0,2}/?(phase\d{1,2}[a-z0-9-]*\.md|00-[a-z0-9-]+\.md)/?\)").unwrap();
    for d in parsed {
        let from_id = format!("DOC:{}", d.source);
        for cap in ref_re.captures_iter(&d.raw) {
            let rel = cap.get(0).unwrap().as_str()
                .trim_matches(|ch: char| ch == '(' || ch == ')' || ch == '.' || ch == '/')
                .to_string();
            let target = parsed.iter().find(|p| p.source.ends_with(&rel));
            if let Some(t) = target {
                let to_id = format!("DOC:{}", t.source);
                let k = format!("references|{}|{}", from_id, to_id);
                if seen_edges.insert(k.clone()) {
                    edges.push(GraphEdge {
                        id: format!("e{}", { eid += 1; eid }),
                        kind: "references".into(),
                        from: from_id.clone(),
                        to: to_id,
                        weight: Some(0.7),
                        note: Some("cross-doc".into()),
                        created_at: now.clone(),
                    });
                }
            }
        }
    }

    // 3) Explicit relational statements inside text
    let stmt_re = Regex::new(r"(?i)\b([A-Z]{2,5}-REQ-\d{3}|REQ-\d{3}|ADR-\d{4})\b[^.\n]{0,40}\b(implements|depends[_ ]on|supersedes|gated[_ ]by|caused[_ ]by|blocks|references)\b[^.\n]{0,40}\b([A-Z]{2,5}-REQ-\d{3}|REQ-\d{3}|ADR-\d{4})\b").unwrap();
    for d in parsed {
        for cap in stmt_re.captures_iter(&d.raw) {
            let a = cap.get(1).unwrap().as_str();
            let rel = cap.get(2).unwrap().as_str().to_lowercase().replace(' ', "_");
            let b = cap.get(3).unwrap().as_str();
            if nodes.contains_key(a) && nodes.contains_key(b) {
                let k = format!("{}|{}|{}", rel, a, b);
                if seen_edges.insert(k.clone()) {
                    edges.push(GraphEdge {
                        id: format!("e{}", { eid += 1; eid }),
                        kind: rel,
                        from: a.to_string(),
                        to: b.to_string(),
                        weight: Some(0.9),
                        note: Some("explicit statement".into()),
                        created_at: now.clone(),
                    });
                }
            }
        }
    }

    GraphPair {
        nodes: nodes.into_values().collect(),
        edges,
    }
}

pub struct GraphPair {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_doc_extracts_requirements() {
        let raw = r#"# Test

See [REQ-001](REQ-001) and GRF-REQ-002 in the next line.
ADR-0001 says so.
"#;
        let p = parse_doc("test.md", raw);
        assert_eq!(p.title, "Test");
        assert!(p.requirement_ids.contains(&"REQ-001".to_string()));
        assert!(p.requirement_ids.contains(&"GRF-REQ-002".to_string()));
        assert!(p.adr_ids.contains(&"ADR-0001".to_string()));
    }

    #[test]
    fn kind_of_maps_prefix_to_kind() {
        assert_eq!(kind_of("OPS-REQ-001"), "policy");
        assert_eq!(kind_of("AGT-REQ-001"), "agent");
        assert_eq!(kind_of("REQ-001"), "requirement");
        assert_eq!(kind_of("ADR-0001"), "adr");
    }

    #[test]
    fn docs_to_graph_dedupes_edges() {
        let a = parse_doc("a.md", "# A\nREQ-001 implements REQ-002\nREQ-001 implements REQ-002\n");
        let b = parse_doc("b.md", "# B\nREQ-001 implements REQ-002\n");
        let g = docs_to_graph(&[a, b]);
        let mut count = 0;
        for e in &g.edges {
            if e.from == "REQ-001" && e.to == "REQ-002" && e.kind == "implements" { count += 1; }
        }
        assert_eq!(count, 1, "duplicate edges should be deduped");
    }

    #[test]
    fn explicit_relations_normalize_to_underscore_kind() {
        let a = parse_doc("a.md", "# A\nREQ-001 depends on REQ-002\nREQ-001 gated_by REQ-002\n");
        let g = docs_to_graph(&[a]);
        let kinds: Vec<&str> = g.edges.iter().map(|e| e.kind.as_str()).collect();
        assert!(kinds.contains(&"depends_on"), "kinds were {kinds:?}");
        assert!(kinds.contains(&"gated_by"), "kinds were {kinds:?}");
    }
}