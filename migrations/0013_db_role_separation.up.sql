-- 0013 — DB Role 分离 (AISEC-REQ-009(a) MVP 必填)
-- 详细设计：§01-data-layer.md §1.3

-- 平台 role: 完整 DML
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'gitgit_app') THEN
        CREATE ROLE gitgit_app LOGIN PASSWORD NULL;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'gitgit_audit_readonly') THEN
        CREATE ROLE gitgit_audit_readonly;
    END IF;
END$$;

-- 平台主账号：完整 DML
GRANT CONNECT ON DATABASE current_database() TO gitgit_app;
GRANT USAGE ON SCHEMA public TO gitgit_app;
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO gitgit_app;
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO gitgit_app;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO gitgit_app;

-- events 表：append-only role
-- gitgit_app 默认可以 INSERT events；但 UPDATE / DELETE 拒绝
-- 通过触发器已在 0012 拒绝
REVOKE UPDATE, DELETE ON events FROM gitgit_app;
GRANT INSERT, SELECT ON events TO gitgit_app;

-- audit_log：只读给非 admin
GRANT SELECT ON audit_log TO gitgit_audit_readonly;
REVOKE INSERT, UPDATE, DELETE ON audit_log FROM gitgit_audit_readonly;
