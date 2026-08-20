# 用户认证流程

> 详细设计：[`../09-security-impl.md`](../09-security-impl.md) §9.2 (JWT) / §9.3 (信封加密)
> 关联 REQ：SEC-REQ-001 (身份认证) / SEC-REQ-004 (短期凭证)
> 关联流程：任务 62 (UT) / 69 (ITa) / 83 (ST)

## 登录时序图（含 MFA）

```mermaid
sequenceDiagram
    actor User
    participant CLI as gitgit CLI / Web UI
    participant API as Auth API
    participant DB as PostgreSQL
    participant TOTP as TOTP Verifier

    User->>CLI: 输入 username + password
    CLI->>API: POST /auth/login
    activate API
    API->>DB: SELECT user, password_hash
    alt 密码错误
        API-->>CLI: 401 Unauthorized
    else 密码正确
        API->>DB: SELECT mfa_enabled, mfa_secret_encrypted
        alt MFA 未启用
            API->>DB: 生成 refresh_token, INSERT session
            API-->>CLI: 200 OK (access + refresh JWT)
        else MFA 已启用
            API-->>CLI: 200 OK (mfa_required=true, mfa_token=...)
            CLI->>API: POST /auth/mfa { code: "123456" }
            API->>TOTP: verify(code, secret)
            alt TOTP 错误
                API-->>CLI: 401 MfaInvalid
            else TOTP 正确
                API->>DB: 生成 refresh_token, INSERT session
                API-->>CLI: 200 OK (access + refresh JWT)
            end
        end
    end
    deactivate API
```

## JWT 签发 / 验证

```mermaid
sequenceDiagram
    participant Client
    participant API
    participant Crypto as JWT Signer (ed25519)

    Client->>API: 任意请求 + Bearer access_token
    activate API
    API->>Crypto: jwt.verify(token, public_key)
    alt token 过期
        Crypto-->>API: Err(ExpiredSignature)
        API-->>Client: 401 TokenExpired
    else token 签名错
        Crypto-->>API: Err(InvalidSignature)
        API-->>Client: 401 TokenInvalid
    else token 有效
        Crypto-->>API: Ok(Claims)
        API->>API: Policy 评估 (subject_id, role)
        alt 策略允许
            API-->>Client: 200 OK
        else 策略拒绝
            API-->>Client: 403 Forbidden
        end
    end
    deactivate API
```
