-- 0012 — 审计 (含 admin_audit 哈希链)
-- 详细设计：§13-admin-api §13.3 + SEC-REQ-011
-- 不可篡改：UPDATE / DELETE 触发器拒绝
CREATE TABLE audit_log (
    id BIGSERIAL PRIMARY KEY,
    actor_id UUID NOT NULL,
    actor_type TEXT NOT NULL DEFAULT 'user',            -- 'user'|'agent'|'admin'|'system'
    action TEXT NOT NULL,                                -- 'app.upgrade'|'kek.rotate' 等
    target_type TEXT NULL,
    target_id TEXT NULL,
    request_id UUID NULL,
    ip_addr INET NULL,
    user_agent TEXT NULL,
    data JSONB NOT NULL DEFAULT '{}'::jsonb,
    prev_hash BYTEA NULL,                                -- 哈希链前一项
    entry_hash BYTEA NOT NULL,                           -- 本项 hash = SHA256(prev_hash || data)
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    tenant_id UUID NOT NULL
);

CREATE INDEX audit_log_actor_idx ON audit_log (actor_id, created_at DESC);
CREATE INDEX audit_log_action_idx ON audit_log (action, created_at DESC);
CREATE INDEX audit_log_target_idx ON audit_log (target_type, target_id);
CREATE INDEX audit_log_tenant_idx ON audit_log (tenant_id);

-- 防篡改触发器
CREATE OR REPLACE FUNCTION audit_log_prevent_modify() RETURNS trigger AS $$
BEGIN
    RAISE EXCEPTION 'audit_log is append-only; UPDATE/DELETE forbidden';
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER audit_log_no_update BEFORE UPDATE ON audit_log
    FOR EACH ROW EXECUTE FUNCTION audit_log_prevent_modify();
CREATE TRIGGER audit_log_no_delete BEFORE DELETE ON audit_log
    FOR EACH ROW EXECUTE FUNCTION audit_log_prevent_modify();
