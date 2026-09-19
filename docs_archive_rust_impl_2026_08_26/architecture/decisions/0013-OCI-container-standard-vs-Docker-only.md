# ADR-0013: Agent Workspace 容器标准 — OCI / containerd vs Docker-only

> **Status**: Proposed
> **Date**: 2026-08-23
> **Deciders**: SA + TL + EM
> **Consulted**: SEC + SRE
> **Informed**: 全部

## Context and Problem Statement

[基本设计 §2.3 物理架构](../../design/basic-design/02-architecture.md#23-物理架构-physical-architecture) 与 §3.5 Agent Runtime 都提到 Agent Workspace 隔离,但没有选定具体的容器运行时。候选:

- Docker Engine (Moby) — 成熟,开发者熟悉,但 daemon 体积大、需要特权
- containerd — CNCF 毕业,OCI 兼容,K8s 默认 CRI,daemon 轻
- Podman — rootless 默认,适合多租户,但与 K8s 集成度不如 containerd
- gVisor / Firecracker — 强隔离(沙箱级),但启动慢、镜像兼容性需验证

## Decision Drivers

- [DRIVER-1] 必须支持 K8s 部署(V1+ Cloud)— 与 [ADR-0006](../../architecture/decisions/0006-采用-OCI-容器Docker-containerd-+-K8s-部署-V1+-Cloud.md) 一致
- [DRIVER-2] MVP 阶段能在 Linux 单进程 Local-First 跑通
- [DRIVER-3] 沙箱强度(基本 OS 隔离即可,gVisor 推迟 V1+)
- [DRIVER-4] 多租户 App 不能越权访问其他 App 的资源(详设 §12)
- [DRIVER-5] 镜像格式标准 — OCI Image Manifest(详设 §12 OCI Plugin)

## Considered Options

1. **Option A — containerd (OCI) + runc** (推荐)
2. **Option B — Docker Engine**
3. **Option C — gVisor (runsc)**

## Decision Outcome

**Chosen option**: "Option A — containerd (OCI) + runc", because 与 ADR-0006 / ADR-0018 的 OCI 体系一致;V1+ 切 K8s 时零迁移成本;MVP 启动快(进程内嵌 containerd client)。

### 具体选型

| 阶段 | 容器运行时 | 备注 |
|---|---|---|
| MVP (Local) | 进程内嵌 `containerd` 1.7+ client + `runc` | 通过 `/run/containerd/containerd.sock` Unix socket |
| V0.5 (单 VM) | 同上 | 加 systemd unit 管理 |
| V1+ Cloud (K8s) | K8s 自带 containerd CRI | 平台不直接管理运行时 |
| V1+ 高安全 App | 切换 `runsc` (gVisor) per-App spec | V1+ 选项,非默认 |

### Consequences

**Good:**
- [+] 镜像与 K8s 通用,App 升级零迁移(详设 §12.3)
- [+] 进程内嵌比 Docker daemon 节省 ~100MB 内存
- [+] 与 [ADR-0018 OCI Plugin 包格式](#) 完整对齐
- [+] V1+ 可选 gVisor 而不破坏 App 兼容性

**Bad:**
- [-] containerd 文档比 Docker 少;开发者 onboarding 需补足
- [-] 镜像构建(开发期)仍推荐 Docker CLI(只是构建结果一致),团队需知道"build 用 docker,run 用 containerd"

### Confirmation

[CONFIRM] MVP 集成测试 (详设 §12 + [specs/test-specification.md §12](../../specs/test-specification.md#12-app-registry--plugin-loader)):
- TC-APP-001: 进程启动后 5s 内可拉取一个 OCI 镜像并启动容器
- TC-APP-002: App 进程内不能访问其他 App 容器文件系统(验证 namespace 隔离)
- TC-APP-003: App 进程 OOM 时被 containerd 检测并上报 Event,不污染 host

## Pros and Cons of the Options

### Option A — containerd + runc
[+] OCI 标准;K8s 兼容;轻量;V1+ 可升级到 gVisor
[-] 文档少;开发者需熟悉 ctr / nerdctl 工具

### Option B — Docker Engine
[+] 开发者熟悉;CLI 成熟
[-] daemon 体积大;V1+ 切 K8s 时迁移成本(虽然镜像格式兼容,但运行时 API 不同)

### Option C — gVisor (runsc)
[+] 强沙箱,适合多租户敌对环境
[-] 启动慢(~200ms vs runc ~30ms);部分 syscall 不支持(影响 Node.js / Go net raw 等);V1+ 选项更合适

## References

- [ADR-0006: 采用 OCI 容器 + K8s 部署 V1+ Cloud](../../architecture/decisions/0006-采用-OCI-容器Docker-containerd-+-K8s-部署-V1+-Cloud.md)
- [ADR-0018: OCI Plugin 包格式](#) (待写)
- [基本设计 §2.3 物理架构](../../design/basic-design/02-architecture.md#23-物理架构-physical-architecture)
- [基本设计 §3.5 Agent Runtime](../../design/basic-design/03-functional-design.md#35-agent-运行时子系统-agent-runtime-subsystem)
- [基本设计 §7.7 App 沙箱](../../design/basic-design/07-security-design.md#77-app-沙箱-app-sandbox)
- [详细设计 §04 Agent Runtime](../../design/detailed-design/04-agent-runtime.md)
- [详细设计 §12 App Registry & Plugin Loader](../../design/detailed-design/12-app-registry-and-plugin-loader.md)

## Revision History

| Date | Author | Change |
|---|---|---|
| 2026-08-23 | Mavis (AI 起草) | Initial draft (Proposed) |
| YYYY-MM-DD | — | Status → Accepted (待 SA + TL + EM 签核) |
