# 流程文档总览 / Process Documentation

> **[TEMPLATE]** 本文件为空白模板，使用时按章节填入项目实际内容。
> 标记 `[TEMPLATE]` 的章节为必填；标记 `[OPTIONAL]` 的章节按项目需要决定是否填写。
> 标记 `[REFERENCE]` 的章节为参考链接，指向现有设计文档，不在本文件展开。

**关联信息 / Metadata:**

| 项 | 值 |
|---|---|
| 流程任务编号 | — |
| 阶段 | 全流程 |
| 主要交付物 | 流程文档索引 |
| 责任人 | 工程负责人 |
| 关联设计文档 | [`workflow.md`](workflow.md) (150 阶段详细) · [`../design/README.md`](../design/README.md) |
| 模板版本 | v1.0 (2026-08-20) |
| 依据 | IPA 共通框架 2013 |

---


本目录 [`docs/process/`](../) 包含 150 阶段工作流与配套模板、治理文档、ADR 等。


## 目录结构 / Directory Layout


```
docs/process/
├── README.md                    # 本文件
├── workflow.md                  # 150 阶段详细
├── glossary.md                  # 项目用语集（PM / 流程专业）
├── role-matrix.md               # RACI 责任矩阵
├── decision-log.md              # 决策日志模板
├── risk-register.md             # 风险登记模板
└── templates/                   # 13 阶段模板（共 14 子目录）
    ├── upstream/                # 01-09 超上流（9 模板）
    ├── requirements/            # 10-21 需求（1 模板）
    ├── design/                  # 22-52 设计（5 模板）
    ├── implementation/          # 53-58 实现（6 模板）
    ├── test/                    # 59-95 测试（4 综合模板）
    ├── migration/               # 96-101 迁移（6 模板）
    ├── release/                 # 102-108 发布（7 模板）
    ├── operations/              # 109-117 运维（9 模板）
    ├── maintenance/             # 118-126 维护（9 模板）
    ├── quality/                 # 127-130 质量（4 模板）
    ├── management/              # 131-144 管理（14 模板）
    └── closure/                 # 145-150 终结（6 模板）
```


## 使用方式 / How to Use


### 1. 项目启动时


- 拷贝 `templates/upstream/05-project-kickoff.md` → 填写项目章程
- 拷贝 `templates/management/131-project-plan.md` + `132-wbs.md` → 制定项目计划
- 拷贝 `templates/management/135-risk.md` → 建立初始风险登记


### 2. 各阶段开始时


- 查 [`workflow.md` §1 阶段总览](workflow.md) 确认本阶段任务编号 + 交付物
- 拷贝对应阶段目录下的模板到项目工作目录，填写实际内容
- 评审通过后归档到 `docs/releases/vX.Y/` 或 `docs/archive/`


### 3. 跨阶段需要时


- 角色 / 责任查 [`role-matrix.md`](role-matrix.md)
- 用语统一查 [`glossary.md`](glossary.md)
- 架构决策查 [`../../../../architecture/decisions/`](../../../../architecture/decisions/)
- 风险跟踪查 [`risk-register.md`](risk-register.md)


## 模板标记规约 / Template Tagging

| 标记 | 含义 |
|---|---|
| [TEMPLATE] | 必填章节，使用时按项目实际内容填写 |
| [OPTIONAL] | 可选章节，按项目需要决定是否填写 |
| [REFERENCE] | 参考链接，指向现有设计文档，不在本文件展开 |
| [TBD] | 现阶段无法确定，留待后续调研 / 评审 / 决策 |
| [FACT] | 可验证事实 |
| [PROPOSAL] | 本项目主张 |
| [IMPL] | 实现层细节（详细设计用） |

## 与现有设计文档的关系 / Relation to Design Docs


所有模板**主体不重复**现有基本设计 / 详细设计的内容，而是通过链接引用。这样：


- 模板简短（80-150 行），可作为 git 友好的 PR 单元
- 实际内容只在基本设计 / 详细设计 / 实施前 QA 表中维护，避免一处更新多处不一致
- 模板的章节结构本身就是该交付物的检查清单（Checklist）


## 维护原则 / Maintenance Principles

- [ ] 模板改动走 PR + 至少 1 名工程负责人 review
- [ ] 模板章节结构变更必须同步更新 workflow.md §1 的对应行
- [ ] 已发出（已分发给项目组使用）的模板不删除，只加 deprecation 标记
- [ ] 跨项目复用：模板是项目无关的；项目特定内容写在项目自己的工作目录

---

**导航 / Navigation:**
[← 流程总览](workflow.md) · [← 流程 README](README.md)
