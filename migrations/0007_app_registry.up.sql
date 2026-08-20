-- 0007 — App 注册表 + 中心事件 (App 一级化, ADR-0004)
-- 详细设计：§12-app-registry-and-plugin-loader.md
CREATE TABLE apps (
    id TEXT PRIMARY KEY,                              -- app 名（namespace）
    manifest JSONB NOT NULL,                          -- app.yaml 内容
    manifest_version TEXT NOT NULL,                   -- 语义化版本
    status TEXT NOT NULL DEFAULT 'installed',          -- 'installed'|'enabled'|'disabled'|'upgrading'|'failed'
    db_role TEXT NOT NULL,                             -- App 沙箱 DB role (SEC-REQ-013 / AISEC-REQ-013)
    db_schema TEXT NOT NULL,                           -- App 私有 schema
    enabled_permissions JSONB NOT NULL DEFAULT '[]'::jsonb,
    forbidden_permissions JSONB NOT NULL DEFAULT '[]'::jsonb,
    installed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    tenant_id UUID NOT NULL
);

CREATE INDEX apps_status_idx ON apps (status);
CREATE INDEX apps_tenant_idx ON apps (tenant_id);

-- App 集群心跳
CREATE TABLE app_heartbeats (
    app_id TEXT NOT NULL REFERENCES apps(id) ON DELETE CASCADE,
    instance_id TEXT NOT NULL,                         -- 多实例
    last_heartbeat_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    status TEXT NOT NULL DEFAULT 'healthy',             -- 'healthy'|'degraded'|'failed'
    metadata JSONB NULL,
    PRIMARY KEY (app_id, instance_id)
);

CREATE INDEX app_heartbeats_liveness_idx ON app_heartbeats (last_heartbeat_at);

-- 中心事件订阅
CREATE TABLE event_subscriptions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    app_id TEXT NOT NULL REFERENCES apps(id) ON DELETE CASCADE,
    event_type TEXT NOT NULL,                            -- 支持通配符 'repo.*'
    filter JSONB NOT NULL DEFAULT '{}'::jsonb,
    target_url TEXT NULL,                                -- Webhook target
    target_proc TEXT NULL,                               -- 内部 proc name
    enabled BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    tenant_id UUID NOT NULL
);

CREATE INDEX event_subscriptions_app_idx ON event_subscriptions (app_id) WHERE enabled = true;

-- 死信队列
CREATE TABLE event_dlq (
    id BIGSERIAL PRIMARY KEY,
    event_id BIGINT NOT NULL REFERENCES events(id),
    subscription_id UUID NOT NULL REFERENCES event_subscriptions(id),
    error TEXT NOT NULL,
    failed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    retry_count INT NOT NULL DEFAULT 0,
    tenant_id UUID NOT NULL
);
