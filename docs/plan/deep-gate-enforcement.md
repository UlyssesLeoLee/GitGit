# 深门禁强制化 —— 把路径过滤从 workflow 层下沉到 job 层

> `[FACT]` 本文记录的事实全部由实测与官方文档原文得到。
> 标注约定：`[FACT]` 实测或文档原文；`[INFERENCE]` 由事实推出；
> `[PROPOSAL]` 建议；`[UNVERIFIED-FACT]` 已知但未验证。

## 结论先行

本仓 5 个 PR workflow 都是 **workflow 级 `paths:` 过滤**。分支保护里 required check
只有 `repo gate (always runs)`（来自唯一无过滤的 `gate.yml`）。结果：**其余深门禁
全是咨询性的——红了也挡不住合并。**

本 lane 把路径过滤**下沉到 job 内**，让这些作业在**每个 PR 上都报状态**，从而具备
被加进 branch protection 的资格。**本 lane 不改分支保护**（那是合并后的独立一步）。

## 问题：workflow 级过滤为什么让门禁无法被强制

`[FACT]` GitHub 官方文档 "Required status checks" 原文：

> "If a workflow is skipped due to path filtering, branch filtering, or a commit message,
> then checks associated with that workflow will remain in a 'Pending' state. If you have
> required checks, this will block the pull request from being merged."

`[FACT]` 也就是说 workflow 级 `paths:` 有两种结局，都不是「强制」：

1. 把这个作业设成 required check → 它在该跑的时候**永远 pending**，PR 永久卡死；
2. 不设成 required check → 它红不红**完全不影响合并**。

`[FACT]` 本仓正是第 2 种。`docs/plan/v0-tasks.md` 的决策日志在 2026-10-25 记着：
「深门禁（打包 / Rust 测试 / 审计 / 两个 ratchet）**仍是 path 过滤的，未被自动强制**」。

## 机制：job 级跳过报 Success

`[FACT]` GitHub 官方文档 "Using conditions to control job execution" 原文：

> "A job that is skipped will report its status as 'Success'. It will not prevent a pull
> request from merging, even if it is a required check."

`[FACT]` 同页 "Required status checks" 一节：

> "Required status checks must have a successful, skipped, or neutral status before
> collaborators can make changes to a protected branch."

`[INFERENCE]` 于是机制成立：**过滤放在 job 内，被跳过的 job 报 Success，满足 required
check；过滤放在 workflow 上，workflow 根本不跑，check 永远 pending。** 这就是本 lane
的全部内容。

## 本 lane 做了什么

`[FACT]` 新增 `scripts/ci/changed-areas.ps1`：算出本次变更命中了哪几个区域，
每个区域输出一个 `area=true|false` 的 GitHub step output。

`[FACT]` 新增 `scripts/ci/required-checks.json`：声明本仓认定的 required status check
context 列表（**作业名**，因为 GitHub 按作业的 `name:` 报 check）。

`[FACT]` 新增两个测试：

- `apps/gm-desktop/tests/unit/changed-areas-gate.test.ts` —— 脚本的行为；
- `apps/gm-desktop/tests/unit/workflow-gate.test.ts` —— workflow 的结构。

`[FACT]` 三个 workflow 的 `pull_request` 触发器去掉了 workflow 级 `paths:`，每个作业
内部加一步检测，重步骤加区域条件：

| workflow | 区域键 | 作业 |
| --- | --- | --- |
| `rust-backend.yml` | `rust_backend` | `fmt + clippy + test`、`dependency audit (RUSTSEC)`、`ST + IT regression (real git over HTTP)` |
| `gm-console.yml` | `gm_console` | `typecheck + lint + test + build` |
| `frontend-audit.yml` | `frontend_audit` | `frontend dependency advisories (npm ratchet)` |

`[FACT]` `push` 触发器**保留** workflow 级 `paths:`：直接推分支不是 PR，为每次推 dev
付全部门禁的钱买不到 PR 已经买到的东西。

`[FACT]` 检测步骤**放在作业内部**，不是单独一个 `detect` 作业。理由：如果检测是独立
作业、门禁 `needs:` 它，那么「检测失败」会**跳过**门禁，而被跳过的门禁报 Success ——
绿色放行，正是本 lane 要消灭的死锁。放在内部，检测失败就让**这个作业**变红。

`[FACT]` 诊断性的 `if: always()` 上传步骤**刻意不加**区域条件：它们存在的意义就是把
**失败**那一步的日志/覆盖率交出来，加了条件就把唯一有用的那次产出拿走了。

## 覆盖范围一个字节都没有变

`[FACT]` 五个区域的模式列表，就是原来 5 个 workflow 的 `paths:` 块**逐字**：

```
rust_backend:        src/**  Cargo.toml  Cargo.lock  scripts/**  .github/workflows/rust-backend.yml
gm_console:          apps/gm-console/**  src/server/api.rs  .github/workflows/gm-console.yml
gm_desktop:          apps/gm-desktop/**  .github/workflows/gm-desktop.yml
                     .github/workflows/release.yml  .github/workflows/gate.yml
                     Cargo.toml  Cargo.lock
                     scripts/advisory-baseline.json  scripts/check-advisories.ps1
gm_desktop_bundle:   apps/gm-desktop/**  .github/workflows/gm-desktop-bundle.yml
                     .github/workflows/gm-desktop.yml  Cargo.toml  Cargo.lock  src/**
frontend_audit:      apps/gm-desktop/**  apps/gm-console/**
                     scripts/frontend-advisory-baseline.json
                     scripts/frontend-advisory-check.ps1
                     .github/workflows/frontend-audit.yml
```

`[FACT]` 这份列表被 `workflow-gate.test.ts` 钉住：**测试读的是脚本自己打印出来的
模式列表**，再与测试里记录的列表逐字比对。改了脚本的模式而不改测试 → 测试变红，
改动的 diff 里会看到「覆盖范围移动了」。

`[INFERENCE]` 覆盖范围不变 = 重构只改变「什么时候报状态」，不改变「什么被门禁覆盖」。

## 路径匹配语义：与 GitHub 对齐到什么程度

`[FACT]` GitHub 官方文档关于 `paths:` 模式：路径模式**从仓库根开始匹配整条路径**，
所以 `'Cargo.toml'` 只匹配仓库根的那个，不匹配 `apps/gm-console/Cargo.toml`；
`*` 不跨 `/`，`**` 跨 `/`；模式必须加引号。

`[FACT]` 本仓实测的 5 份列表里**一个通配符都没有**，只有两种形态：字面文件名，
和 `目录/**`。

`[FACT]` 脚本实现了这两种形态，遇到带 `*` / `?` / `!` 的模式**直接报错退出**，
而不是当字面量比较——后者会让一个门禁**看起来在工作、实际匹配错文件**。

`[UNVERIFIED-FACT]` **判定 `paths:` 是否匹配的组件不开源。** 同一个来源指出，
唯一可靠的答案是一次 push 之后读 Actions tab。因此「本地实现与 GitHub 行为等价」
是 `[INFERENCE]`，不是 `[FACT]`。真正的验证在「验证计划」一节。

## 验证计划（合并之后做，不在本 lane）

`[PROPOSAL]` 合并后由父代理做两件事：

1. 把 `scripts/ci/required-checks.json` 里声明的 5 个 context 加进 `dev` / `main` 的
   branch protection required checks。
2. 用**两个真实 PR** 实证：
   - 一个**只改 `docs/**`** 的 PR → 5 个 context 全部**出现**且为绿（不是 pending）；
   - 一个**改了 `src/**`** 的 PR → rust 门禁**真的跑起来**（看日志里有 cargo 的输出，
     不是空跑）。

`[FACT]` 加 required check 有一个自举问题需要按顺序处理：改 branch protection 的那个
PR 自己也必须满足新的 required check。所以顺序必须是「先合并让作业会报告的 PR，
再加 required check，再用后续 PR 验证」。

`[FACT]` 作业名一旦改名，等于换掉 required check，而仓里没有任何东西会察觉。
`workflow-gate.test.ts` 就是为此存在：它断言每个 context 名字在全部 workflow 文件里
**恰好出现一次**，且**不含 `${{`**。

## 风险与回滚

| 风险 | 缓解 |
| --- | --- |
| 检测失败 → 门禁红 | 刻意如此。检测失败说明变更集算不出来，此时「什么都没变」是最危险的读法。脚本对这种情况**非零退出**，不静默放行。 |
| 区域模式与 GitHub 语义有偏差 | 模式列表被测试钉住；语义偏差只能在 GitHub 上实测发现，所以验证计划里有 docs-only 与 src-touching 两个 PR。 |
| required check 名字对不上 → PR 卡死 | `workflow-gate.test.ts` 断言每个 context 恰好一个作业拥有。 |
| `needs:` 链把红色变成绿色 | L1-a 的 5 个作业都没有 `needs:`，测试对此有断言。 |
| 回滚 | 恢复 workflow 级 `paths:` 即可，**但必须同时**把 branch protection 的 required checks 改回去，否则 PR 会永久 pending。 |

`[FACT]` 本 lane 的 5 个作业里**没有任何一个带 `needs:`**。`gm-desktop.yml` 的
`msi-install` / `msi-launch` 带 `needs: msi` 是**有意为之**，属于 L1-b。

## 本 lane 明确没做的事

`[FACT]` **没有改分支保护。** `scripts/ci/required-checks.json` 里
`notes.enforcedOnGitHub` 记的是 `false`。这个提交之后，GitHub 上仍然只要求
`repo gate (always runs)`。文件里写的是**声明**，不是**强制**。

`[FACT]` **没有重构 `gm-desktop.yml` / `gm-desktop-bundle.yml`。** 它们的
`msi` / `msi-launch` 作业在 PR #57（2026-10-25 合并）里才稳定下来，和路径过滤
下沉一起改会让失败无法归因。它们是 L1-b；脚本已经包含全部 5 个区域，L1-b 只需套用。

`[FACT]` **没有动 `apps/gm-console/**`、`scripts/frontend-advisory-baseline.json`** ——
那是另一条 lane（PR #58，gm-console 工具链升级）正在改的文件。

## 一次被测出来的假阳性

`[FACT]` 本地跑 gm-desktop 完整测试套件时，`tests/unit/workflow-step-isolation.test.ts`
里 `release.yml verify-tag` 那条用例**偶发** 5s 超时。该用例会**真实 spawn bash
进程**执行 release.yml 的 verify-tag 步骤（release.yml 本 lane 根本没改）。

`[FACT]` 做了 A/B 重复实验：带本 lane 改动跑 4 次全过（耗时 6.98–10.25s），
把改动 stash 掉再跑 4 次也全过（7.45–8.50s）。**单次「stashed 通过 / 带改动失败」
是巧合，不是因果。** 结论：这是该用例 5s 超时在负载下的固有波动，与本 lane 无关，
本 lane 因此**没有**改它。

`[INFERENCE]` 这条正是「只跑一次不叫测量」的又一个例子：第一次的对照实验给出的
结论（"是本 lane 引起的"）是错的，重复实验才把它推翻。
