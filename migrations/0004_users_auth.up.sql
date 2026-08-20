-- 0004 — 用户与认证 (SEC-REQ-001)
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username TEXT NOT NULL,
    email TEXT NOT NULL,
    password_hash TEXT NULL,                        -- argon2id
    webauthn_credentials JSONB NULL,                -- WebAuthn public keys
    mfa_enabled BOOLEAN NOT NULL DEFAULT false,
    mfa_secret_encrypted BYTEA NULL,                -- TOTP secret (encrypted)
    status TEXT NOT NULL DEFAULT 'active',          -- 'active' | 'disabled' | 'locked'
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_login_at TIMESTAMPTZ NULL,
    tenant_id UUID NOT NULL,
    UNIQUE (username, tenant_id),
    UNIQUE (email, tenant_id)
);

CREATE INDEX users_email_idx ON users (email);
CREATE INDEX users_tenant_idx ON users (tenant_id);

-- sessions
CREATE TABLE sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    refresh_token_hash TEXT NOT NULL,
    issued_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL,
    revoked BOOLEAN NOT NULL DEFAULT false,
    user_agent TEXT NULL,
    ip_addr INET NULL,
    tenant_id UUID NOT NULL
);

CREATE INDEX sessions_user_idx ON sessions (user_id);
CREATE INDEX sessions_expires_idx ON sessions (expires_at) WHERE revoked = false;
