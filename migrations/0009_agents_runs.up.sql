-- 0009 — Agent / CI 运行 (AGT-REQ + CI-REQ)
-- 详细设计：§04-agent-runtime.md
CREATE TABLE agents (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    type TEXT NOT NULL,                                  -- 'claude_code' | 'codex' | 'custom' 等
    config JSONB NOT NULL DEFAULT '{}'::jsonb,
    enabled BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    tenant_id UUID NOT NULL,
    UNIQUE (name, tenant_id)
);

CREATE TABLE agent_runs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    agent_id UUID NOT NULL REFERENCES agents(id),
    parent_run_id UUID NULL REFERENCES agent_runs(id),     -- AGT-REQ-009 (subagent)
    state TEXT NOT NULL DEFAULT 'pending',                 -- AGT-REQ-001 状态机
    input JSONB NOT NULL,
    output JSONB NULL,
    artifacts JSONB NULL,
    error JSONB NULL,
    started_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    finished_at TIMESTAMPTZ NULL,
    approved_by UUID NULL,                                  -- AGT-REQ-002 人类批准
    approved_at TIMESTAMPTZ NULL,
    correlation_id UUID NULL,
    tenant_id UUID NOT NULL
);

CREATE INDEX agent_runs_state_idx ON agent_runs (state);
CREATE INDEX agent_runs_agent_idx ON agent_runs (agent_id, started_at DESC);
CREATE INDEX agent_runs_correlation_idx ON agent_runs (correlation_id);

-- Agent credentials (AGT-REQ-004 短期 scoped)
CREATE TABLE agent_credentials (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    run_id UUID NOT NULL REFERENCES agent_runs(id) ON DELETE CASCADE,
    subject TEXT NOT NULL,
    scope JSONB NOT NULL,
    issued_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL,
    revoked BOOLEAN NOT NULL DEFAULT false
);

CREATE INDEX agent_creds_expires_idx ON agent_credentials (expires_at) WHERE revoked = false;
