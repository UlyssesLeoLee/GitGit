# F14-7 — 人类干系人正式签核环节缺口

> **缺口状态**: OPEN (Phase 16 收尾时仍未关闭)
> **影响范围**: 全部需要 PO / EM / SEC 签核的交付物
> **建议关闭时间**: 第一次实际有人担任 PO / EM / SEC 角色时

## 背景

Phase 14 IPA 合规评审发现 F14-7："**全流程无人类干系人正式签核环节**"。

完整 Phase 1-16 全部 13 阶段 + 2 补充阶段，签核位都空着或自动填默认值。

本项目（AI-Native Engineering Platform）的需求调研 + 设计 + 流程文档全过程由
AI 辅助（MiniMax Mavis）完成，**没有真实人类产品 owner / 业务方对结果负过责**。

## 受影响签核位

| 任务 | 签核方 | 当前状态 | 流程文档 |
|---|---|---|---|
| 21 要件批准 + Baseline | PO + EM + SEC | 空 | [process/templates/requirements/20-requirements-review.md](../process/templates/requirements/20-requirements-review.md) |
| 41 基本设计评审 (BD Review) | SA + EM + SEC + PO | 空 | [process/templates/design/41-bd-review.md](../process/templates/design/41-bd-review.md) |
| 52 详细设计评审 (DD Review) | TL + SA + EM | 空 | [process/templates/design/52-dd-review.md](../process/templates/design/52-dd-review.md) |
| 89 系统测试完成批准 | QA + EM | 空 | [process/templates/test/test-report.md](../process/templates/test/test-report.md) |
| 94 验收判定 | PO + EM | 空 | [process/templates/test/test-report.md](../process/templates/test/test-report.md) |
| 95 验收证书 | PO + EM + SEC | 空 | [process/templates/test/test-report.md](../process/templates/test/test-report.md) |
| 103 发布判定 (Go/No-Go) | PO + EM + SRE | 空 | [process/templates/release/103-go-nogo.md](../process/templates/release/103-go-nogo.md) |
| 145 项目完成判定 | PM + PO + EM | 空 | [process/templates/closure/145-completion.md](../process/templates/closure/145-completion.md) |
| 基础设计 23 任务 + 详细设计 11 任务 + 流程 80 模板 + ADR 9 项 | SA / TL / PO | **未签核** | 全部 docs/ |

## 实质影响

1. **质量保证链断裂**: 全部"PO 签核"位实际是 AI 自动通过
2. **需求真实性存疑**: Phase 7-13 的需求可能偏离真实业务需求
3. **架构决策权威性弱**: 9 个 ADR 实际是 EM + AI 共识，无 PO 视角
4. **治理责任真空**: 出问题时无法追溯"谁批准了这个决定"

## 建议关闭方案

### 立即（Phase 16 收尾时）

- [x] **承认缺口** — 本文档
- [x] **明确缺口影响范围** — 上述 9 类签核位
- [ ] **明确 1 个真实人类担任 PO** — 推荐：项目发起人本人（Ulysses Leo Lee）
- [ ] **明确 1 个真实人类担任 EM** — 推荐：当前实施负责人
- [ ] **明确 1 个真实人类担任 SEC** — 推荐：组织安全负责人
- [ ] **回填签核位** — 至少对 Baseline v1.0 完成追溯性签核

### 中期（V0.1 → V0.5）

- 建立"无人类签核不得合并"规则
- 引入 CODEOWNERS + PR 评审硬性要求
- F14-7 finding 的真正关闭：所有后续需求变更需人类签核

### 长期（V1+）

- 集成外部 SSO（OIDC / SAML）实现 PO 真实身份
- 建立"AI 起草 + 人类签核"的标准工作流

## 关联

- Phase 14 评审：[../requirements/phase14-ipa-compliance-review.md](../requirements/phase14-ipa-compliance-review.md) F14-7
- 实施前 QA：[rchitecture/qa-checklist.md](../architecture/qa-checklist.md) QA-001
- 流程文档：[process/glossary.md](../process/glossary.md) 角色定义
