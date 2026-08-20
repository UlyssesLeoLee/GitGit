-- 0010 — 视图快照 (CTX-REQ-002 确切重建支持)
CREATE MATERIALIZED VIEW IF NOT EXISTS mv_node_type_counts AS
SELECT type, tenant_id, COUNT(*) AS n
FROM nodes
GROUP BY type, tenant_id
WITH NO DATA;

CREATE INDEX IF NOT EXISTS mv_node_type_counts_tenant_idx ON mv_node_type_counts (tenant_id);
