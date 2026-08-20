-- 0006 — 协调结构 (Outbox + Saga + 跨 App 通信)
-- 详细设计：§07-app-coordination.md
CREATE TABLE coord_sagas (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    state TEXT NOT NULL DEFAULT 'pending',            -- 'pending'|'running'|'compensating'|'completed'|'failed'
    steps JSONB NOT NULL,                              -- 步骤定义
    current_step INT NOT NULL DEFAULT 0,
    context JSONB NOT NULL DEFAULT '{}'::jsonb,
    error JSONB NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    completed_at TIMESTAMPTZ NULL,
    tenant_id UUID NOT NULL
);

CREATE INDEX coord_sagas_state_idx ON coord_sagas (state) WHERE state IN ('pending', 'running', 'compensating');
CREATE INDEX coord_sagas_tenant_idx ON coord_sagas (tenant_id);

-- outbox (event 待发布)
CREATE TABLE coord_outbox (
    id BIGSERIAL PRIMARY KEY,
    event_id BIGINT NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    target_apps TEXT[] NOT NULL DEFAULT '{}',          -- 空 = 广播给所有
    published_at TIMESTAMPTZ NULL,
    retry_count INT NOT NULL DEFAULT 0,
    next_retry_at TIMESTAMPTZ NULL,
    last_error TEXT NULL,
    tenant_id UUID NOT NULL
);

CREATE INDEX coord_outbox_pending_idx ON coord_outbox (next_retry_at NULLS FIRST) WHERE published_at IS NULL;

-- cross-app call audit
CREATE TABLE coord_call_log (
    id BIGSERIAL PRIMARY KEY,
    caller_app TEXT NOT NULL,
    callee_app TEXT NOT NULL,
    procedure_name TEXT NOT NULL,
    args JSONB NOT NULL,
    result_status TEXT NOT NULL,
    started_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    finished_at TIMESTAMPTZ NULL,
    correlation_id UUID NULL,
    tenant_id UUID NOT NULL
);

CREATE INDEX coord_call_log_caller_idx ON coord_call_log (caller_app, started_at DESC);
