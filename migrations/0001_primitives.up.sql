-- 0001 — 5 原语核心表
-- 详细设计：§01-data-layer.md §1.4.1
-- 关联 REQ：GRF-REQ-001, GRF-REQ-004, AISEC-REQ-009(a)

CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- nodes
CREATE TABLE nodes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    type TEXT NOT NULL,
    subtype TEXT NULL,
    schema_version INT NOT NULL DEFAULT 1,
    data JSONB NOT NULL,
    epistemic_status TEXT NULL,                    -- GRF-REQ-010
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by UUID NOT NULL,                      -- 引用 users (后续 migration)
    tenant_id UUID NOT NULL                        -- CLOUD-REQ-003
);

CREATE INDEX nodes_type_idx ON nodes (type);
CREATE INDEX nodes_tenant_idx ON nodes (tenant_id);
CREATE INDEX nodes_created_at_idx ON nodes (created_at DESC);
CREATE INDEX nodes_data_gin ON nodes USING GIN (data);

-- edges
CREATE TABLE edges (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    src_id UUID NOT NULL REFERENCES nodes(id) ON DELETE CASCADE,
    dst_id UUID NOT NULL REFERENCES nodes(id) ON DELETE CASCADE,
    type TEXT NOT NULL,
    data JSONB NOT NULL DEFAULT '{}'::jsonb,
    epistemic_status TEXT NULL,                    -- GRF-REQ-010
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    tenant_id UUID NOT NULL,
    UNIQUE (src_id, dst_id, type)
);

CREATE INDEX edges_src_idx ON edges (src_id);
CREATE INDEX edges_dst_idx ON edges (dst_id);
CREATE INDEX edges_type_idx ON edges (type);
CREATE INDEX edges_tenant_idx ON edges (tenant_id);

-- events (append-only, AISEC-REQ-009(a) DB role 分离)
CREATE TABLE events (
    id BIGSERIAL PRIMARY KEY,
    type TEXT NOT NULL,
    actor_id UUID NOT NULL,
    target_id UUID NULL,
    target_type TEXT NULL,
    data JSONB NOT NULL DEFAULT '{}'::jsonb,
    correlation_id UUID NULL,
    trace_id TEXT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    tenant_id UUID NOT NULL
);

CREATE INDEX events_actor_idx ON events (actor_id, created_at DESC);
CREATE INDEX events_target_idx ON events (target_id, created_at DESC);
CREATE INDEX events_correlation_idx ON events (correlation_id);
CREATE INDEX events_tenant_idx ON events (tenant_id);
CREATE INDEX events_type_idx ON events (type, created_at DESC);

-- policies
CREATE TABLE policies (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    type TEXT NOT NULL,                            -- 'rbac' | 'abac' | 'ai_specific' 等
    rules JSONB NOT NULL,
    priority INT NOT NULL DEFAULT 100,
    enabled BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    tenant_id UUID NOT NULL,
    UNIQUE (name, tenant_id)
);

CREATE INDEX policies_type_idx ON policies (type);
CREATE INDEX policies_enabled_idx ON policies (enabled) WHERE enabled = true;

-- views
CREATE TABLE views (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    type TEXT NOT NULL,                            -- 'graph' | 'codebase' | 'symbol' | 'agent_run' 等
    definition JSONB NOT NULL,
    materialized BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    tenant_id UUID NOT NULL,
    UNIQUE (name, tenant_id)
);

CREATE INDEX views_type_idx ON views (type);
