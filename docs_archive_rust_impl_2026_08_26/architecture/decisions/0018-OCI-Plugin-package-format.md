# ADR-0018: OCI Plugin 包格式 (App / Plugin Image Manifest schema)

> **Status**: Proposed
> **Date**: 2026-08-23
> **Deciders**: SA + TL + EM
> **Consulted**: SEC + SRE + PO
> **Informed**: 全部

## Context and Problem Statement

[基本设计 §13.2 App 集群与可热插拔架构](../../design/basic-design/13-app-cluster-and-plugins.md) 与 [详设 §12 App Registry & Plugin Loader](../../design/detailed-design/12-app-registry-and-plugin-loader.md) 要求 App / Plugin 以"标准容器镜像"方式打包,但没有定义:

- 用什么 OCI manifest schema?
- App 元数据放哪一层(annotation / config / 单独 label)?
- 与 [ADR-0008 Plugin Manifest Schema (app.yaml)](../../architecture/decisions/0008-Plugin-App-Manifest-Schema-app.yaml-与版本兼容策略.md) 的关系?
- 多架构(amd64 + arm64)支持?

## Decision Drivers

- [DRIVER-1] 沿用 OCI Image Spec v1.1(标准,生态最广)
- [DRIVER-2] App metadata 不能仅靠 label(会被 `docker build` 误改)
- [DRIVER-3] 与 [ADR-0008](../../architecture/decisions/0008-Plugin-App-Manifest-Schema-app.yaml-与版本兼容策略.md) 兼容:`app.yaml` 仍然存在,但打包进 OCI 镜像
- [DRIVER-4] 多架构支持(V1+ Cloud 多节点异构)
- [DRIVER-5] 镜像签名(可选 V0.5+,与 [SEC-REQ-007 强认证](#) 协调)

## Considered Options

1. **Option A — OCI Image Manifest v1.1 + app.yaml 内嵌** (推荐)
2. **Option B — 自定义 CRD 格式,完全脱离 OCI**
3. **Option C — 仅用 OCI label, 不内嵌 app.yaml**

## Decision Outcome

**Chosen option**: "Option A — OCI Image Manifest v1.1 + app.yaml 内嵌", because 与 [ADR-0013 containerd](#) 生态一致;`app.yaml` 仍存在(避免打破 [ADR-0008](../../architecture/decisions/0008-Plugin-App-Manifest-Schema-app.yaml-与版本兼容策略.md));多架构通过 OCI Image Index 支持。

### 镜像结构

```text
platform-app-<name>:<version>
├── OCI Image Manifest v1.1 (sha256:xxx)
│   ├── config: "platform.app.v1" (mediaType: application/vnd.platform.app.config.v1+json)
│   ├── layers:
│   │   ├── 0: app.yaml          (mediaType: application/vnd.platform.app.yaml.v1+yaml)
│   │   ├── 1: bin/<executable>  (mediaType: application/vnd.oci.image.layer.v1.tar+gzip)
│   │   ├── 2: lib/...           (媒体类型同 1)
│   │   └── 3: assets/...        (可选)
│   └── annotations:
│       ├── "org.opencontainers.image.title": "platform-app-<name>"
│       ├── "org.opencontainers.image.version": "<semver>"
│       ├── "org.opencontainers.image.created": "<rfc3339>"
│       └── "platform.app/v1.spec": "sha256:<app.yaml 的 hash>"
└── (可选) OCI Image Index for multi-arch:
    ├── manifest.linux/amd64
    └── manifest.linux/arm64
```

### `app.yaml` 在镜像中的位置

| 字段 | 路径 | mediaType |
|---|---|---|
| 配置文件 | `/app.yaml` (镜像内绝对路径) | `application/vnd.platform.app.yaml.v1+yaml` |
| Manifest 引用 | OCI Manifest `annotations["platform.app/v1.spec"]` = `sha256:<app.yaml>` | — |

### 多架构支持

通过 OCI Image Index 实现:

```json
{
  "schemaVersion": 2,
  "mediaType": "application/vnd.oci.image.index.v1+json",
  "manifests": [
    {
      "mediaType": "application/vnd.oci.image.manifest.v1+json",
      "digest": "sha256:...",
      "platform": { "architecture": "amd64", "os": "linux" }
    },
    {
      "mediaType": "application/vnd.oci.image.manifest.v1+json",
      "digest": "sha256:...",
      "platform": { "architecture": "arm64", "os": "linux" }
    }
  ]
}
```

平台部署时根据 `runtime.GOARCH` 自动选择对应 manifest。

### 镜像签名 (V0.5+)

- 默认 **不要求** 签名(MVP / V0.5)
- V0.5+ 启用 [Cosign](https://github.com/sigstore/cosign) 签名:
  - 公钥随平台分发(平台仓库 `keys/cosign.pub`)
  - 私钥由 App 发布方持有
  - 平台在 `plugin-loader` 拉镜像前验证签名
  - 签名失败 → 拒绝 + 审计 event

### 与 [ADR-0008 app.yaml](../../architecture/decisions/0008-Plugin-App-Manifest-Schema-app.yaml-与版本兼容策略.md) 的关系

- `app.yaml` schema **不变**(沿用 [ADR-0008](../../architecture/decisions/0008-Plugin-App-Manifest-Schema-app.yaml-与版本兼容策略.md))
- 打包方式变化: 从"文件目录" → "OCI 镜像 + 内嵌 app.yaml"
- 加载时: 平台从 OCI 镜像 layer 0 读取 `app.yaml` 字符串,再用 [ADR-0008](../../architecture/decisions/0008-Plugin-App-Manifest-Schema-app.yaml-与版本兼容策略.md) 的 schema 校验

### Consequences

**Good:**
- [+] 与 OCI 生态完全一致,工具链复用(`docker push` / `oras` / `crane`)
- [+] `app.yaml` 仍存在,不破坏开发者心智模型
- [+] 多架构通过 OCI Image Index 标准化
- [+] 签名可插拔(MVP 跳过,V0.5+ 启用)

**Bad:**
- [-] OCI Image Manifest v1.1 字段较多,App 开发者需要了解 annotations
- [-] 多架构构建链(amd64 + arm64)需要 CI 配置,增加首次构建复杂度

### Confirmation

[CONFIRM] 详设 §12 App Registry & Plugin Loader + [specs/test-specification.md §12](../../specs/test-specification.md#12-app-registry--plugin-loader):
- TC-APP-010: 构建 `platform-app-hello:1.0.0` 并 push 到本地 OCI registry,能正确拉取
- TC-APP-011: 平台启动 Plugin Loader 后,自动发现并加载 `platform-app-hello:1.0.0`
- TC-APP-012: 多架构镜像 (amd64 + arm64) 在 `GOARCH=arm64` 节点上拉取时自动选 arm64 manifest
- TC-APP-013: 篡改镜像任一字节后,平台拒绝加载并审计 event
- TC-APP-014: V0.5+ Cosign 签名验证失败的镜像被拒绝

## Pros and Cons of the Options

### Option A — OCI Image Manifest v1.1 + app.yaml 内嵌
[+] 标准;工具链复用;多架构原生
[-] 镜像结构相对复杂

### Option B — 自定义 CRD,脱离 OCI
[+] 完全可控
[-] 失去 OCI 生态;违反"能标准协议不发明"原则

### Option C — 仅 OCI label
[+] 最简
[-] label 易被 `docker build` 误改;不可签名;元数据与镜像层分离

## References

- [ADR-0008: Plugin / App Manifest Schema (app.yaml) 与版本兼容策略](../../architecture/decisions/0008-Plugin-App-Manifest-Schema-app.yaml-与版本兼容策略.md)
- [ADR-0013: OCI 容器标准 vs Docker-only](#)
- [基本设计 §13.2 App 集群与可热插拔架构](../../design/basic-design/13-app-cluster-and-plugins.md)
- [详细设计 §12 App Registry & Plugin Loader](../../design/detailed-design/12-app-registry-and-plugin-loader.md)
- [OCI Image Spec v1.1](https://github.com/opencontainers/image-spec/blob/main/spec.md)
- [Cosign — OCI 签名](https://github.com/sigstore/cosign)

## Revision History

| Date | Author | Change |
|---|---|---|
| 2026-08-23 | Mavis (AI 起草) | Initial draft (Proposed) |
| YYYY-MM-DD | — | Status → Accepted (待 SA + TL + EM 签核) |
