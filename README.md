# GitGit

> 山寨的 Git —— 但真正在做的是《AI-Native Engineering Platform 需求定义书》

本仓库目前的主要内容不是代码，而是一份完整走完 **13 个阶段** 的产品需求调研与定义程序，产出对象是一个暂命名为 **AI-Native Engineering Platform** 的、Local-First / Cloud-Ready / Git-Native / AI-Native / Agent-Native / Graph-Native 的可自托管软件工程平台的正式需求定义书。

所有产出物位于 [`docs/requirements/`](docs/requirements/)。

---

## 这是什么

Git 管理代码版本；这个项目要调研并定义一个更进一步的平台——管理"软件工程状态"本身的版本。它管理的对象不止 Repository / Branch / Commit / Tag，还包括 Requirement、Issue、ADR、PR、Review、Test、CI、Artifact、Release、Deployment、Incident、Agent、AgentRun、Prompt、Skill、Policy 等，最终形成一个可查询的 **Engineering Knowledge Graph**。

整个需求定义过程严格区分**事实与设计**，全文使用统一标记：

| 标记 | 含义 |
|---|---|
| `[FACT]` | 有官方/一手来源支撑并标注访问日期的可验证事实 |
| `[UNVERIFIED-FACT]` | 有来源但未经一手验证（如仅见于第三方聚合站），不可直接当作确证事实使用 |
| `[INFERENCE]` | 基于已知事实的合理推论，未独立核实 |
| `[PROPOSAL]` | 本项目自己的设计主张，明确不归因于任何竞品 |
| `[TBD]` | 现阶段无法确认，留待后续调研/评审/决策 |

禁止把自己的设计构想包装成竞品已有的事实，也禁止为了让文档"看起来完整"而编造未经证实的数字（例如性能指标一律标注 `TBD – Benchmark Required`，而不是凭空给出数字）。

---

## 13 个阶段全景

```
Phase 1-5   竞品调研 & Gap 分析     ──▶ phase1-5-research.md
Phase 6     产品原语发现            ──▶ phase6-primitives.md
Phase 7     需求草拟（带ID）        ──▶ phase7-elicitation.md
Phase 8     55节正式需求定义书      ──▶ 00-requirements-definition.md
Phase 9     MVP 精简（最小完整闭环）──▶ phase9-mvp-reduction.md
Phase 10    架构设计（能删就删）    ──▶ phase10-architecture.md
Phase 11    红队评审（自我攻击）    ──▶ phase11-red-team.md
Phase 12    UX 红队评审             ──▶ phase12-ux-review.md
Phase 13    最终基线 Baseline v1.0  ──▶ phase13-final-baseline.md
```

每个阶段的产出都以独立文件保存、独立提交、经过 PR 走查后合入 `main`，任何阶段都可以单独回溯查阅推理过程，不是一次性生成的"完整幻觉"。

---

## 文档索引（`docs/requirements/`）

| 文件 | 内容概要 |
|---|---|
| [`phase1-5-research.md`](docs/requirements/phase1-5-research.md) | 对 GitHub、GitLab、Gitea/Forgejo、Bitbucket、Sourcegraph、OpenAI Codex、Claude Code、Cursor、Linear 等的深度竞品调研；能力矩阵；Commodity/Differentiator/Emerging/Experimental 四象限 Gap 分析 |
| [`phase6-primitives.md`](docs/requirements/phase6-primitives.md) | 从 Observation→Pattern→Constraint→Primitive→Interaction→Workflow→Capability 的涌现式推导；最终 MVP 原语集合为 **Node / Edge / Event / Policy / View**（Agent 作为 Node 子类型，Action/Evidence 延后，Intent/Context 被砍） |
| [`phase7-elicitation.md`](docs/requirements/phase7-elicitation.md) | 按 GIT / GRF / AGT / AI / CTX / SEC / CI / UX / OPS / CLOUD 十个前缀分类、带唯一 ID、含验收标准的原子需求草稿 |
| [`00-requirements-definition.md`](docs/requirements/00-requirements-definition.md) | **主文档**：55 节正式需求定义书 + ADR 待定清单附录（Baseline v1.0） |
| [`phase9-mvp-reduction.md`](docs/requirements/phase9-mvp-reduction.md) | 用"最小完整闭环"（Repository→Issue→AI Context→Agent Branch→Code Change→CI→AI Review→Human Approval→Merge→Graph Update）作为唯一标准，严格砍需求而非堆功能数量 |
| [`phase10-architecture.md`](docs/requirements/phase10-architecture.md) | Principal Architect 视角的架构评审：技术栈逐项评估、图存储方案决策、Git 存储实现方式决策、服务边界、Local→Cloud 迁移路径、"哪些组件被砍掉"清单 |
| [`phase11-red-team.md`](docs/requirements/phase11-red-team.md) | 从 17 个角度对整个方案发起真实攻击（是否只是 GitHub Clone、是否 AI 包装、是否过度工程、Agent 执行安全面、本地部署是否过重等），16 条发现 |
| [`phase12-ux-review.md`](docs/requirements/phase12-ux-review.md) | UX 红队评审："稳定骨架 + 涌现式上下文"是否真的可执行，Ambient AI 预算（80/15/5 目标）是否会退化成 Chat，9 条发现 |
| [`phase13-final-baseline.md`](docs/requirements/phase13-final-baseline.md) | **收官文档**：对全部 25 条红队/UX 发现逐一处置并实际修订主文档；三大护城河（Moats）分析；正面回答"如果 GitHub/GitLab 明天 AI 提升十倍，我们为何还存在"；宣告 Baseline v1.0 |

---

## 关键结论速览

### 三大护城河（诚实版本，非营销话术）

1. **统一可查询的工程图谱** —— 留存型护城河而非获客型护城河：每一次循环迭代都在积累图数据，让后续查询和溯源越来越值钱，但新团队接入时图谱是空的，第一天没有价值。
2. **真正的自托管 + 云一致性 + Agent 原生执行** —— 自托管一致性部分持久可信；但"Agent 原生执行"的能力优势建立在前沿模型质量之上,会随所有厂商模型进步而被稀释。
3. **人机统一审计轨迹** —— 三者中最可能随 AI 能力提升而**变得更有价值**的护城河（Agent 越自主，统一审计越重要），但也是技术上最容易被竞争对手直接复制的一个。

三者都**没有真正的网络效应**——项目坦诚承认，没有为了让故事好看而编造。

### 终极判断题

> 如果 GitHub 和 GitLab 明天把 AI 能力提升十倍，这个产品为什么还存在？

诚实回答：会继续存在，但只对真正看重自托管控制权、含 Agent 的审计严谨性、以及积累的工程图谱——胜过原始 AI 能力本身——的那部分组织而言；会是一个更小、更持久的细分市场，而不是 GitHub/GitLab 的大众替代品。且明确承认：这个论证目前只停留在纸面，尚未有一个真正跑起来的 MVP 验证过。

### MVP 范围

经过 Phase 9 的"最小完整闭环"测试和 Phase 13 红队/UX 评审后的两轮压力测试，最终 MVP 需求数为 **43 条**（历经砍、加、再加的往复调整，过程可追溯）。

### 红队/UX 发现处置统计（共 25 条）

| 处置结果 | 数量 |
|---|---|
| 已接受并实际修复主文档 | 18 |
| 已接受但延后至 V1/V2 | 4 |
| 已驳回（附论证） | 2 |
| 需人类决策（无法靠分析解决） | 1 |

全部 4 条 Critical 级发现（3 条红队 + 1 条 UX）均已获得实际修复,而非被归入"待办事项"敷衍了事。

---

## 阅读建议

- 只想看结论：直接看 [`phase13-final-baseline.md`](docs/requirements/phase13-final-baseline.md) 的 §4（三大护城河）、§5（终极问题）、§6（Baseline 声明）。
- 想看完整正式需求：看 [`00-requirements-definition.md`](docs/requirements/00-requirements-definition.md)，目录附有全部 55 节链接。
- 想看某个具体判断是怎么来的：按上面的阶段顺序从 Phase 1 往后读，每个阶段都写明了方法、证据来源和推理过程。
- 想看这个项目"骗不骗人"：重点看 Phase 11（红队评审）和 Phase 12（UX 红队评审）——这两个阶段的存在就是为了防止前面的阶段自说自话。

---

## 状态

**Baseline v1.0**（Phase 1–13 全部完成）。明确排除在本基线范围之外的内容：具体实现代码、UI 线框图/原型、法律与许可证审查、benchmark 实测数据、去竞争市场定位（Go-to-Market）决策、正式 ADR 文档产出物。这些留待后续阶段处理。
