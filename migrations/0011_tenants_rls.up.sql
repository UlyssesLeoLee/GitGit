-- 0011 — Tenant + Row-Level Security 启用 (CLOUD-REQ-003)
-- 详细设计：§01-data-layer.md §1.6
CREATE TABLE tenants (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL UNIQUE,
    plan TEXT NOT NULL DEFAULT 'self_hosted',           -- 'self_hosted'|'cloud_free'|'cloud_pro'|'enterprise'
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    status TEXT NOT NULL DEFAULT 'active'
);

-- 给核心表启用 RLS
ALTER TABLE nodes ENABLE ROW LEVEL SECURITY;
ALTER TABLE edges ENABLE ROW LEVEL SECURITY;
ALTER TABLE events ENABLE ROW LEVEL SECURITY;
ALTER TABLE policies ENABLE ROW LEVEL SECURITY;
ALTER TABLE views ENABLE ROW LEVEL SECURITY;

-- 策略：current_setting('app.tenant_id') 匹配
CREATE POLICY nodes_tenant_isolation ON nodes
    USING (tenant_id::text = current_setting('app.tenant_id', true));

CREATE POLICY edges_tenant_isolation ON edges
    USING (tenant_id::text = current_setting('app.tenant_id', true));

CREATE POLICY events_tenant_isolation ON events
    USING (tenant_id::text = current_setting('app.tenant_id', true));

CREATE POLICY policies_tenant_isolation ON policies
    USING (tenant_id::text = current_setting('app.tenant_id', true));

CREATE POLICY views_tenant_isolation ON views
    USING (tenant_id::text = current_setting('app.tenant_id', true));
