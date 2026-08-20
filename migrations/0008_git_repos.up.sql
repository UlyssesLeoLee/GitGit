-- 0008 — Git 仓库 (GIT-REQ-001 / 002)
CREATE TABLE git_repositories (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    namespace TEXT NOT NULL,                            -- 'org/repo'
    name TEXT NOT NULL,
    description TEXT NULL,
    visibility TEXT NOT NULL DEFAULT 'private',         -- 'public'|'private'|'internal'
    default_branch TEXT NOT NULL DEFAULT 'main',
    storage_path TEXT NOT NULL,                          -- bare repo 路径
    object_format TEXT NOT NULL DEFAULT 'sha1',          -- 'sha1' or 'sha256'
    size_bytes BIGINT NOT NULL DEFAULT 0,
    lfs_enabled BOOLEAN NOT NULL DEFAULT false,
    hooks JSONB NOT NULL DEFAULT '{}'::jsonb,             -- pre-receive / post-receive 等
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    pushed_at TIMESTAMPTZ NULL,
    tenant_id UUID NOT NULL,
    UNIQUE (namespace, name, tenant_id)
);

CREATE INDEX git_repos_tenant_idx ON git_repositories (tenant_id);
CREATE INDEX git_repos_pushed_at_idx ON git_repositories (pushed_at DESC);
