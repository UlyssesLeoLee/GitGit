-- 0003 — 权限表 (RBAC + ABAC)
-- 详细设计：§03-policy-engine.md §3.7-3.8
CREATE TABLE roles (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    description TEXT NULL,
    tenant_id UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (name, tenant_id)
);

CREATE TABLE permissions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    action TEXT NOT NULL,                            -- 'repo.create' 等
    resource_type TEXT NOT NULL,                     -- 'repository' 等
    description TEXT NULL,
    UNIQUE (action, resource_type)
);

CREATE TABLE role_permissions (
    role_id UUID NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    permission_id UUID NOT NULL REFERENCES permissions(id) ON DELETE CASCADE,
    PRIMARY KEY (role_id, permission_id)
);

CREATE TABLE user_roles (
    user_id UUID NOT NULL,
    role_id UUID NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    tenant_id UUID NOT NULL,
    granted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    granted_by UUID NOT NULL,
    expires_at TIMESTAMPTZ NULL,
    PRIMARY KEY (user_id, role_id, tenant_id)
);

CREATE INDEX user_roles_user_idx ON user_roles (user_id);
CREATE INDEX user_roles_tenant_idx ON user_roles (tenant_id);
