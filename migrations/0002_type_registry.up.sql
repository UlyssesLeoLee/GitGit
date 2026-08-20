-- 0002 — 类型注册表 (GRF-REQ-002)
CREATE TABLE type_registry (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    type_name TEXT NOT NULL,
    schema_ref TEXT NOT NULL,                      -- 指向 JSON Schema 存储位置
    schema_content JSONB NOT NULL,                 -- 实际 JSON Schema 内容
    version INT NOT NULL DEFAULT 1,
    deprecated BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (type_name, version)
);

CREATE INDEX type_registry_active_idx ON type_registry (type_name) WHERE deprecated = false;
