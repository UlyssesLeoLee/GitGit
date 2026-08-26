# 编码规范与指南 / Coding Guideline

> **[TEMPLATE]** 本文件为空白模板，使用时按章节填入项目实际内容。
> 标记 `[TEMPLATE]` 的章节为必填；标记 `[OPTIONAL]` 的章节按项目需要决定是否填写。
> 标记 `[REFERENCE]` 的章节为参考链接，指向现有设计文档，不在本文件展开。

**关联信息 / Metadata:**

| 项 | 值 |
|---|---|
| 流程任务编号 | 54 |
| 阶段 | 实现 — 编码 (PG) |
| 主要交付物 | 源代码 + 编码规范 |
| 责任人 | 实施工程师 |
| 关联设计文档 | [`../../architecture/tech-selection.md`](../../../architecture/tech-selection.md) §15 (强约束) · Rust API Guidelines |
| 模板版本 | v1.0 (2026-08-20) |
| 依据 | IPA 共通框架 2013 |

---


## 强约束（REQ-TS 强制）

| 编号 | 约束 | 验证方式 |
|---|---|---|
| REQ-TS-001 | 禁止 unsafe 块（除非明确 ADR 批准） | cargo geiger / 人工评审 |
| REQ-TS-002 | 禁止 .unwrap() / .expect()（仅 #[cfg(test)] 允许） | clippy::unwrap_used / 人工 |
| REQ-TS-003 | 禁止 GPL/AGPL 依赖 | cargo deny |
| REQ-TS-004 | 禁止 [TBD] 性能数字 | PR review |
| REQ-TS-005 | Error 必须用 thiserror + ? 传播 | clippy::result_large_err |

## 命名 / Naming

| 对象 | 风格 | 例 |
|---|---|---|
| 模块 / crate | kebab-case (目录) / snake_case (文件) | git-server.rs |
| 函数 | snake_case | create_node |
| 类型 | UpperCamelCase | NodeRepository |
| 常量 | SCREAMING_SNAKE | MAX_RETRY_COUNT |
| 错误类型 | Error 后缀 | NodeError |

## 错误处理 / Error Handling


[REFERENCE] 完整规约见 [`../../design/detailed-design/11-error-handling.md`](../../../design/detailed-design/11-error-handling.md)。


```rust
// ✅ 推荐：使用 thiserror + ?
#[derive(thiserror::Error, Debug)]
pub enum AppError {
    #[error("node not found: {0}")]
    NodeNotFound(NodeId),
    #[error("policy denied: {0}")]
    PolicyDenied(String),
}

// ❌ 禁止：
let val = option.unwrap();
```


## 注释 / Comments

- [ ] 公开 API 必须有 `///` 文档注释
- [ ] 复杂逻辑必须有 `//` 行内注释说明 why
- [ ] 禁止 `// TODO` 长期不解决（必须开 issue 链接）

## 测试 / Testing

- [ ] 新增函数必须同时新增单元测试
- [ ] 覆盖率 ≥ 80%（任务 65 退出条件）
- [ ] 测试函数名 `test_xxx` 风格

---

**导航 / Navigation:**
[← 流程总览](../../workflow.md) · [← 流程 README](../../README.md)
