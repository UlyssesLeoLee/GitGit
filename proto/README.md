# Protocol Buffers — 内部 gRPC 接口

5 个核心 proto 文件，覆盖平台内部服务间通信。

## 文件清单

| Proto | 服务 | 用途 | 关联 |
|---|---|---|---|
| `graph.proto` | GraphService | 5 原语操作（Node/Edge/Event/Traverse）| 详细设计 §02 |
| `policy.proto` | PolicyService | 策略评估 | 详细设计 §03 |
| `ai.proto` | AIGateway | AI 网关（多 provider）| 详细设计 §05 |
| `agent.proto` | AgentRuntime | Agent 启动 / 批准 / 事件流 | 详细设计 §04 |
| `internal.proto` | InternalService | 平台 <-> Admin 通信 | 详细设计 §13 |

## 构建

```bash
# 安装 buf
cargo install buf

# 生成 Rust 代码
buf generate

# 验证
buf lint
buf breaking --against '.git#branch=main'
```

## 命名约定

- Package: `gitgit.<domain>.v1`
- Service: 复数 + Service (GraphService)
- RPC: 动词 + 名词 (CreateNode, GetNode)
- 字段: snake_case
- 枚举: 全大写下划线 (USER_ACTIVE)
