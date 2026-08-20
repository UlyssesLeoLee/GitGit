-- 0005 — 密钥存储 (SEC-REQ-005, SEC-REQ-010 信封加密)
-- 三层信封：DEK (per-secret) -> KEK (per-tenant) -> MasterKey (KMS / file)
CREATE TABLE secrets (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    encrypted_dek BYTEA NOT NULL,                    -- DEK encrypted by KEK
    nonce BYTEA NOT NULL,                             -- chacha20poly1305 nonce
    ciphertext BYTEA NOT NULL,                        -- encrypted value
    aad JSONB NOT NULL DEFAULT '{}'::jsonb,            -- additional authenticated data
    kek_version INT NOT NULL,                         -- KEK rotation version
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    rotated_at TIMESTAMPTZ NULL,
    tenant_id UUID NOT NULL,
    UNIQUE (name, tenant_id)
);

CREATE INDEX secrets_tenant_idx ON secrets (tenant_id);

-- KEK metadata
CREATE TABLE keks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    version INT NOT NULL,
    encrypted_by_master BOOLEAN NOT NULL,
    algorithm TEXT NOT NULL,                           -- 'chacha20poly1305' 等
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    retired_at TIMESTAMPTZ NULL,
    tenant_id UUID NOT NULL,
    UNIQUE (version, tenant_id)
);
