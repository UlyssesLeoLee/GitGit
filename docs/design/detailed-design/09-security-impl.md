# 09. 安全实现 / Security Implementation

## 9.1 目标 / Purpose

实现 [基本设计 §7 安全设计](../basic-design/07-security-design.md) 的具体技术：信封加密、KEK 管理、TLS、网络隔离、AISEC-REQ-009(a) DB role 强制（[01 数据层](01-data-layer.md) 详写 schema 层面）。

## 9.2 模块结构

```
internal/security/
├── envelope/
│   ├── envelope.go            # 加密/解密封装
│   ├── aes_gcm.go             # AES-256-GCM 实现
│   ├── dek.go                 # DEK 生成/轮换
│   └── kek.go                 # KEK 来源抽象
├── kek/
│   ├── env.go                 # 环境变量
│   ├── file.go                # 文件 (本地)
│   ├── kms_aws.go             # AWS KMS (V1+)
│   └── kms_gcp.go             # GCP KMS (V1+)
├── secrets/
│   ├── service.go             # Secrets 服务 API
│   ├── rotate.go              # 轮换
│   └── vault.go               # 备份恢复
├── tls/
│   ├── server.go              # TLS server 配置
│   ├── client.go              # TLS 客户端配置
│   ├── ca.go                  # CA 证书管理
│   └── mtls.go                # 双向 mTLS (Cloud)
├── network/
│   ├── policy.go              # 网络策略
│   ├── sandbox.go             # Agent Workspace 隔离
│   └── egress.go              # 出口控制
├── audit/
│   └── emitter.go             # 审计事件写入
├── access/
│   ├── middleware.go          # 认证中间件
│   ├── rbac.go                # 角色检查
│   └── abac.go                # 属性检查
├── aipolicy/
│   └── (见 03 策略引擎)
├── errors.go
└── security_test.go
```

## 9.3 信封加密实现

### 9.3.1 加密

```go
// [IMPL] internal/security/envelope/envelope.go

type EncryptedBlob struct {
    Ciphertext []byte
    Nonce      []byte  // 96 bits for GCM
    AAD        []byte  // additional authenticated data
    DEKID      string  // 用于支持 key rotation
    Version    int
}

func Encrypt(ctx context.Context, kekProvider KEKProvider, plaintext []byte, aad []byte) (EncryptedBlob, error) {
    // 1. 取 KEK
    kek, err := kekProvider.GetCurrentKEK(ctx)
    if err != nil { return EncryptedBlob{}, err }

    // 2. 生成新 DEK
    dek := make([]byte, 32)  // 256 bits
    if _, err := rand.Read(dek); err != nil { return EncryptedBlob{}, err }

    // 3. 用 DEK 加密明文 (AES-256-GCM)
    nonce := make([]byte, 12)
    if _, err := rand.Read(nonce); err != nil { return EncryptedBlob{}, err }
    block, _ := aes.NewCipher(dek)
    aead, _ := cipher.NewGCM(block)
    ciphertext := aead.Seal(nil, nonce, plaintext, aad)

    // 4. 用 KEK 包装 DEK
    kekNonce := make([]byte, 12)
    rand.Read(kekNonce)
    kekBlock, _ := aes.NewCipher(kek.Key)
    kekAEAD, _ := cipher.NewGCM(kekBlock)
    wrappedDEK := kekAEAD.Seal(nil, kekNonce, dek, []byte(kek.ID))

    // 5. 记录版本
    return EncryptedBlob{
        Ciphertext: ciphertext,
        Nonce:      nonce,
        AAD:        aad,
        DEKID:      kek.ID,
        Version:    1,
    }, nil
}
```

### 9.3.2 解密

```go
// [IMPL] internal/security/envelope/envelope.go
func Decrypt(ctx context.Context, kekProvider KEKProvider, blob EncryptedBlob, wrappedDEK []byte) ([]byte, error) {
    // 1. 取对应 KEK
    kek, err := kekProvider.GetKEKByID(ctx, blob.DEKID)
    if err != nil { return nil, err }

    // 2. 解包 DEK
    kekBlock, _ := aes.NewCipher(kek.Key)
    kekAEAD, _ := cipher.NewGCM(kekBlock)
    dek, err := kekAEAD.Open(nil, kekNonce, wrappedDEK, []byte(kek.ID))
    if err != nil { return nil, ErrWrappedDEKCorrupt }

    // 3. 用 DEK 解密数据
    block, _ := aes.NewCipher(dek)
    aead, _ := cipher.NewGCM(block)
    plaintext, err := aead.Open(nil, blob.Nonce, blob.Ciphertext, blob.AAD)
    if err != nil { return nil, ErrCiphertextCorrupt }
    return plaintext, nil
}
```

### 9.3.3 AAD 设计

**[PROPOSAL]** AAD 必须包含上下文信息（防止密钥被搬到其他位置使用）：

```go
// [IMPL] AAD 格式
type AAD struct {
    OwnerID   string  // 资源所有者 Node ID
    Type      string  // 'api_key','ssh_key','token'
    Version   int
    CreatedAt int64   // unix timestamp
}

func (a AAD) Encode() []byte {
    return []byte(fmt.Sprintf("v%d|owner=%s|type=%s|ts=%d",
        a.Version, a.OwnerID, a.Type, a.CreatedAt))
}
```

**反例：** 缺少 AAD 时，攻击者可以复制密文到其他资源条目。本设计强制 AAD。

## 9.4 KEK 提供者

### 9.4.1 环境变量（开发）

```go
// [IMPL] internal/security/kek/env.go
type EnvKEKProvider struct {
    envVarName string  // 'PLATFORM_KEK'
}

func (p *EnvKEKProvider) GetCurrentKEK(ctx context.Context) (*KEK, error) {
    encoded := os.Getenv(p.envVarName)
    if encoded == "" { return nil, ErrKEKNotConfigured }
    return decodeBase64KEK(encoded)
}

// 格式: base64(kek_id + "|" + 32-byte raw KEK)
// 例: "PLATFORM_KEK=primary|AAAA...32bytes..."
```

### 9.4.2 文件（生产 / 自托管）

```go
// [IMPL] internal/security/kek/file.go
type FileKEKProvider struct {
    path string  // '/var/lib/platform/kek.json', 0600
    mu   sync.Mutex
}

type KEKFile struct {
    Current string  // 当前 KEK ID
    Keys    map[string]Base64Key
}

func (p *FileKEKProvider) GetCurrentKEK(ctx context.Context) (*KEK, error) {
    p.mu.Lock()
    defer p.mu.Unlock()
    data, err := os.ReadFile(p.path)
    if err != nil { return nil, err }
    var kf KEKFile
    json.Unmarshal(data, &kf)
    return &KEK{ID: kf.Current, Key: kf.Keys[kf.Current]}, nil
}
```

### 9.4.3 AWS KMS（V1+）

```go
// [IMPL] internal/security/kek/kms_aws.go
type AWSKMSProvider struct {
    client   *kms.Client
    keyID    string  // 'arn:aws:kms:us-east-1:123:key/abcd...'
}

func (p *AWSKMSProvider) GetCurrentKEK(ctx context.Context) (*KEK, error) {
    // 1. 从 KMS 生成数据密钥 (不离开 HSM)
    out, err := p.client.GenerateDataKey(ctx, &kms.GenerateDataKeyInput{
        KeyId:   aws.String(p.keyID),
        KeySpec: types.DataKeySpecAes256,
    })
    if err != nil { return nil, err }
    return &KEK{
        ID:          p.keyID,
        Key:         out.Plaintext,  // 仅本次使用, 不持久化
        CiphertextBlob: out.CiphertextBlob,  // 持久化以恢复
    }, nil
}
```

## 9.5 Secrets 服务

### 9.5.1 API

```go
// [IMPL] internal/security/secrets/service.go

type Service struct {
    db        *pgxpool.Pool
    kek       KEKProvider
    audit     *audit.Emitter
    policy    *policy.Engine
}

type Secret struct {
    ID          uuid.UUID
    OwnerID     uuid.UUID
    SecretType  string
    Plaintext   []byte  // 仅内存, 绝不持久化
    Metadata    map[string]string
    CreatedAt   time.Time
    RotatedAt   *time.Time
    ExpiresAt   *time.Time
}

func (s *Service) Create(ctx context.Context, ownerID uuid.UUID, secretType string, plaintext []byte, expiresAt *time.Time) (*Secret, error) {
    actor := ActorFromContext(ctx)

    // 1. Policy
    allowed, err := s.policy.Evaluate(ctx, PolicyInput{
        Subject: actor, Action: "secrets.create",
        Resource: ResourceRef{Type: "secrets", ID: ownerID.String()},
    })
    if !allowed { return nil, ErrPolicyDenied }

    // 2. 加密
    aad := envelope.AAD{OwnerID: ownerID.String(), Type: secretType, Version: 1, CreatedAt: time.Now().Unix()}
    blob, err := envelope.Encrypt(ctx, s.kek, plaintext, aad.Encode())
    if err != nil { return nil, err }

    // 3. 写 secrets schema
    wrappedDEK, _ := s.kek.WrapDEK(blob.DEK, blob.DEKID)
    secretID := uuid.Must(uuid.NewV7())
    _, err = s.db.Exec(ctx, `
        INSERT INTO secrets.secrets (id, owner_id, secret_type, encrypted_blob, dek_wrapped, iv, aad, kek_id, expires_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
    `, secretID, ownerID, secretType, blob.Ciphertext, wrappedDEK, blob.Nonce, blob.AAD, blob.DEKID, expiresAt)
    if err != nil { return nil, err }

    // 4. 审计
    s.audit.EmitAsync(ctx, "secret.created", actor, map[string]any{
        "secret_id": secretID, "owner_id": ownerID, "type": secretType, "expires_at": expiresAt,
    })

    return &Secret{
        ID: secretID, OwnerID: ownerID, SecretType: secretType,
        Plaintext: plaintext, CreatedAt: time.Now(), ExpiresAt: expiresAt,
    }, nil
}

func (s *Service) Get(ctx context.Context, secretID uuid.UUID) (*Secret, error) {
    actor := ActorFromContext(ctx)

    // 1. Policy
    allowed, err := s.policy.Evaluate(ctx, PolicyInput{
        Subject: actor, Action: "secrets.read",
        Resource: ResourceRef{Type: "secrets", ID: secretID.String()},
    })
    if !allowed { return nil, ErrPolicyDenied }

    // 2. 读 DB
    var (
        ownerID     uuid.UUID
        secretType  string
        ciphertext  []byte
        wrappedDEK  []byte
        iv          []byte
        aad         []byte
        kekID       string
        createdAt   time.Time
    )
    err = s.db.QueryRow(ctx, `
        SELECT owner_id, secret_type, encrypted_blob, dek_wrapped, iv, aad, kek_id, created_at
        FROM secrets.secrets WHERE id = $1
    `, secretID).Scan(&ownerID, &secretType, &ciphertext, &wrappedDEK, &iv, &aad, &kekID, &createdAt)
    if err != nil { return nil, err }

    // 3. 解密
    plaintext, err := envelope.Decrypt(ctx, s.kek, envelope.EncryptedBlob{
        Ciphertext: ciphertext, Nonce: iv, AAD: aad, DEKID: kekID,
    }, wrappedDEK)
    if err != nil { return nil, err }

    // 4. 审计
    s.audit.EmitAsync(ctx, "secret.read", actor, map[string]any{
        "secret_id": secretID, "owner_id": ownerID,
    })

    return &Secret{
        ID: secretID, OwnerID: ownerID, SecretType: secretType,
        Plaintext: plaintext, CreatedAt: createdAt,
    }, nil
}
```

### 9.5.2 轮换

```go
// [IMPL] internal/security/secrets/rotate.go

func (s *Service) Rotate(ctx context.Context, secretID uuid.UUID) error {
    secret, err := s.Get(ctx, secretID)
    if err != nil { return err }

    // 1. 用旧 DEK 解密得到明文
    plaintext := secret.Plaintext

    // 2. 用新 DEK 重加密
    aad := envelope.AAD{OwnerID: secret.OwnerID.String(), Type: secret.SecretType, Version: secret.Version+1, CreatedAt: time.Now().Unix()}
    newBlob, err := envelope.Encrypt(ctx, s.kek, plaintext, aad.Encode())
    if err != nil { return err }

    wrappedDEK, _ := s.kek.WrapDEK(newBlob.DEK, newBlob.DEKID)

    // 3. 原子更新
    _, err = s.db.Exec(ctx, `
        UPDATE secrets.secrets
        SET encrypted_blob = $1, dek_wrapped = $2, iv = $3, aad = $4,
            kek_id = $5, version = version + 1, rotated_at = now()
        WHERE id = $6
    `, newBlob.Ciphertext, wrappedDEK, newBlob.Nonce, newBlob.AAD, newBlob.DEKID, secretID)
    return err
}
```

## 9.6 TLS

### 9.6.1 服务端 TLS

```go
// [IMPL] internal/security/tls/server.go

func NewServerTLS(certFile, keyFile string) (tls.Certificate, error) {
    cert, err := tls.LoadX509KeyPair(certFile, keyFile)
    if err != nil { return tls.Certificate{}, err }
    return cert, nil
}

func NewServerTLSConfig(cert tls.Certificate, requireClientCert bool) *tls.Config {
    return &tls.Config{
        Certificates: []tls.Certificate{cert},
        MinVersion:   tls.VersionTLS12,
        CipherSuites: []uint16{
            tls.TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384,
            tls.TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384,
            tls.TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305,
        },
        ClientAuth: func() tls.ClientAuthType {
            if requireClientCert { return tls.RequireAndVerifyClientCert }
            return tls.NoClientCert
        }(),
    }
}
```

### 9.6.2 客户端 TLS

```go
// [IMPL] internal/security/tls/client.go
func NewClientTLSConfig(caFile string) *tls.Config {
    caCert, _ := os.ReadFile(caFile)
    caCertPool := x509.NewCertPool()
    caCertPool.AppendCertsFromPEM(caCert)
    return &tls.Config{
        RootCAs:    caCertPool,
        MinVersion: tls.VersionTLS12,
    }
}
```

### 9.6.3 mTLS (Cloud)

```go
// [IMPL] internal/security/tls/mtls.go
// Cloud 部署: Platform <-> PostgreSQL 之间用 mTLS
// 由 cert-manager + service mesh 提供, 本进程不直接管理证书
```

## 9.7 网络隔离

### 9.7.1 Local：loopback / unix socket

```go
// [IMPL] internal/security/network/local.go
func LocalNetworkPolicy() {
    // 1. Platform 监听 127.0.0.1, 不绑定 0.0.0.0
    // 2. PostgreSQL 走 unix socket (默认)
    // 3. Agent Workspace 容器用 bridge 网络, 相互隔离
}
```

### 9.7.2 Cloud：NetworkPolicy

```yaml
# k8s/agent-workspace-netpol.yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: agent-workspace-isolation
spec:
  podSelector:
    matchLabels:
      app: platform-agent-workspace
  policyTypes:
    - Ingress
    - Egress
  ingress:
    # 仅接受来自 Platform process 的入站
    - from:
        - podSelector:
            matchLabels:
              app: platform
      ports:
        - protocol: TCP
          port: 8080
  egress:
    # 仅允许出站到白名单域名
    - to:
        - namespaceSelector:
            matchLabels:
              name: platform-allowed-egress
      ports:
        - protocol: TCP
          port: 443
```

### 9.7.3 Egress 控制

**[PROPOSAL]** Agent / Platform 出站必须经 allowlist；V1 引入：

| 类别 | 默认 | MVP | V1 |
|---|---|---|---|
| AI provider | 仅配置过的 | ✓ | ✓ |
| Git 协议 (clone remote) | allowlist | 用户配置 | 用户配置 |
| 任意 web | deny | ✓ | ✓ |
| SMTP | deny | 用户配置 | 用户配置 |
| IM webhook (Slack 等) | 用户配置 | 用户配置 | 用户配置 |

## 9.8 审计集成

**[PROPOSAL]** 所有安全相关操作都写 Event：

| Event | 触发 |
|---|---|
| `secret.created` | 创建密钥 |
| `secret.read` | 读取密钥（脱敏） |
| `secret.rotated` | 轮换 |
| `secret.deleted` | 删除 |
| `kek.rotated` | KEK 轮换（管理员操作）|
| `tls.cert.rotated` | TLS 证书轮换 |
| `policy.evaluated` | 每次 Policy 评估（[03 策略引擎](03-policy-engine.md)）|

## 9.9 AISEC-REQ-009(a) 强化

**[PROPOSAL]** 除了 [01 数据层 §1.3](01-data-layer.md#13-db-role-分离aisec-req-009a-mvp-必填) 的 DB role 分离外，再加几层防御：

### 9.9.1 应用层强化

```go
// [IMPL] 启动时检查: 进程使用的 DB role 不能有 events 表的 UPDATE/DELETE
func EnforceEventsImmutability(db *pgxpool.Pool) error {
    var hasUpdate bool
    err := db.QueryRow(context.Background(), `
        SELECT has_table_privilege(current_user, 'events', 'UPDATE')
    `).Scan(&hasUpdate)
    if err != nil { return err }
    if hasUpdate {
        return fmt.Errorf("FATAL: current DB role has UPDATE on events, AISEC-REQ-009(a) violated")
    }
    return nil
}
```

### 9.9.2 启动时强制检查

**[PROPOSAL]** Platform 进程启动时**必须**调用此检查；失败即 panic。CI 集成测试也跑此断言。

### 9.9.3 备份路径例外

**[PROPOSAL]** 仅 `platform_owner` 角色（在管理任务中显式使用）可以 UPDATE/DELETE events。`platform_runtime` 永久 INSERT-only。

## 9.10 输入验证 / Input Validation

```go
// [IMPL] 全平台统一输入验证
import "github.com/go-playground/validator/v10"

var validate = validator.New(validator.WithRequiredStructEnabled())

func ValidateRequest(s any) error {
    if err := validate.Struct(s); err != nil { return ErrValidation }
    return nil
}

// 使用 struct tag
type CreateRepoRequest struct {
    Name       string `validate:"required,min=1,max=100,regexp=^[a-z0-9-]+$"`
    OrgID      string `validate:"required,uuid"`
    Visibility string `validate:"required,oneof=public private internal"`
}
```

**反 SQL 注入：** 所有 DB 查询用参数化（pgx 默认）；存储过程名走白名单（`c.registry.Get`）。

**反 NoSQL 注入：** JSONB 属性在写入前用 JSON Schema 校验（[02 图谱引擎 §2.4](02-graph-engine.md#24-类型注册表-type-registry)）。

## 9.11 错误定义

```go
// [IMPL] internal/security/errors.go
var (
    ErrKEKNotConfigured       = NewError("kek_not_configured", 500, "KEK not configured (env/file/kms)")
    ErrCiphertextCorrupt      = NewError("ciphertext_corrupt", 500, "ciphertext failed integrity check")
    ErrWrappedDEKCorrupt      = NewError("wrapped_dek_corrupt", 500, "wrapped DEK failed to unwrap")
    ErrKEKUnknown             = NewError("kek_unknown", 500, "KEK ID not found in provider")
    ErrSecretExpired          = NewError("secret_expired", 410, "secret has expired, must be rotated")
    ErrTLSConfigInvalid       = NewError("tls_config_invalid", 500, "TLS configuration invalid")
    ErrPolicyDenied           = NewError("security_policy_denied", 403, "security policy denied")
    ErrValidation             = NewError("validation_failed", 400, "input validation failed")
    ErrAISEC009Violated       = NewError("aisec009a_violated", 500, "AISEC-REQ-009(a) DB role separation violated at startup")
)
```

## 9.12 性能预算

| 操作 | 目标 |
|---|---|
| Encrypt 4KB | < 1ms |
| Decrypt 4KB | < 1ms |
| Secret create (含 DB 写) | < 30ms |
| Secret get (含 DB 读 + decrypt) | < 20ms |
| Secret rotate | < 50ms |
| 启动时 AISEC 检查 | < 1s |

## 9.13 测试

| 测试 | 目标 |
|---|---|
| 单元 | 加密/解密往返、AAD 强制 |
| 集成 (testcontainers) | DB role 启动检查 |
| 安全 | 已知攻击向量（密钥重放、跨资源移植）|
| 密钥轮换 | 旧 DEK 仍可解（向后兼容期）|

---

**导航 / Navigation:**
[← 08. API 处理器](08-api-handlers.md) · [README](README.md) · [10. 可观测性 →](10-observability.md)
