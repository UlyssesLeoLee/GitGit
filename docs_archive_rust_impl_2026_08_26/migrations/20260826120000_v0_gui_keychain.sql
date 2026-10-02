-- V0 migration: GUI + keychain + AI/remote registry
-- Per ADR-0020 §2.6. Reversible by `sqlx migrate revert`.
--
-- Tables added:
--   * ai_call_cache       — AI provider response cache (key by diff hash)
--   * ai_review_runs      — one row per AI review invocation
--   * remote_configs      — per-repo remote provider configuration
--   * remote_sync_log     — per-sync attempt history
--   * user_prefs          — generic GUI preference store

-- AI call result cache. Same diff + same provider + same model + same prompt
-- MUST yield the same response, so caching is safe. The cache key is
-- (diff_hash, provider, model, prompt_hash). Storing input/output tokens
-- and estimated cost lets V1 build a usage dashboard.
CREATE TABLE IF NOT EXISTS ai_call_cache (
    id            BIGSERIAL    PRIMARY KEY,
    diff_hash     CHAR(64)     NOT NULL,
    provider      TEXT         NOT NULL,
    model         TEXT         NOT NULL,
    prompt_hash   CHAR(64)     NOT NULL,
    response      JSONB        NOT NULL,
    input_tokens  INTEGER,
    output_tokens INTEGER,
    cost_usd      NUMERIC(10, 6),
    created_at    TIMESTAMPTZ  NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS ai_call_cache_lookup_idx
    ON ai_call_cache (diff_hash, provider, model);

-- AI review runs. One row per invocation, regardless of cache hit/miss.
-- A "pending" row is written before the call starts; we update it on
-- completion. V1 may want a status enum CHECK constraint.
CREATE TABLE IF NOT EXISTS ai_review_runs (
    id            BIGSERIAL    PRIMARY KEY,
    repo_path     TEXT         NOT NULL,
    base_sha      CHAR(40)     NOT NULL,
    head_sha      CHAR(40)     NOT NULL,
    provider      TEXT         NOT NULL,
    model         TEXT         NOT NULL,
    status        TEXT         NOT NULL DEFAULT 'pending',
    started_at    TIMESTAMPTZ  NOT NULL DEFAULT now(),
    finished_at   TIMESTAMPTZ,
    finding_json  JSONB,
    error         TEXT
);
CREATE INDEX IF NOT EXISTS ai_review_runs_repo_idx
    ON ai_review_runs (repo_path, started_at DESC);

-- Per-repo remote provider configuration. A single repo may have several
-- remotes (origin, gitee-mirror, gitlab-pr-fork, ...) each tied to a
-- provider. The vault_key points at the Credential Vault entry that holds
-- the PAT / password for this remote. NULL vault_key = public/anonymous.
CREATE TABLE IF NOT EXISTS remote_configs (
    id                BIGSERIAL    PRIMARY KEY,
    repo_path         TEXT         NOT NULL,
    name              TEXT         NOT NULL,
    provider          TEXT         NOT NULL,
    base_url          TEXT         NOT NULL,
    auth_kind         TEXT         NOT NULL,
    vault_key         TEXT,
    last_sync_at      TIMESTAMPTZ,
    last_sync_status  TEXT,
    UNIQUE (repo_path, name)
);
CREATE INDEX IF NOT EXISTS remote_configs_repo_idx
    ON remote_configs (repo_path);

-- Sync history. One row per attempt (success or failure). We keep every
-- row; V1 may add a retention policy. Syncs are fast-forward only in V0
-- (per ADR-0020 §2.3); V1 will add cherry-pick / reject policies.
CREATE TABLE IF NOT EXISTS remote_sync_log (
    id            BIGSERIAL    PRIMARY KEY,
    repo_path     TEXT         NOT NULL,
    remote_name   TEXT         NOT NULL,
    src_sha       CHAR(40),
    dst_sha       CHAR(40),
    started_at    TIMESTAMPTZ  NOT NULL DEFAULT now(),
    finished_at   TIMESTAMPTZ,
    status        TEXT         NOT NULL,
    error         TEXT
);
CREATE INDEX IF NOT EXISTS remote_sync_log_repo_idx
    ON remote_sync_log (repo_path, started_at DESC);

-- Generic key-value preference store for GUI state. The value column is
-- JSONB so we can store typed shapes (numbers, arrays, objects) without
-- a fixed schema per key. V1 may want a typed column for hot keys.
CREATE TABLE IF NOT EXISTS user_prefs (
    key         TEXT         PRIMARY KEY,
    value_json  JSONB        NOT NULL,
    updated_at  TIMESTAMPTZ  NOT NULL DEFAULT now()
);
