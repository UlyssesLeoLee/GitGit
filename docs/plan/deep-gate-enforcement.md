# 深门禁强制化 —— 把路径过滤从 workflow 层下沉到 job 层

> `[FACT]` 本文记录的事实全部由实测与官方文档原文得到。
> 标注约定：`[FACT]` 实测或文档原文；`[INFERENCE]` 由事实推出；
> `[PROPOSAL]` 建议；`[UNVERIFIED-FACT]` 已知但未验证。

## 结论先行

本仓 5 个 PR workflow 原先都是 **workflow 级 `paths:` 过滤**。分支保护里 required check
只有 `repo gate (always runs)`（来自唯一无过滤的 `gate.yml`）。结果：**其余深门禁
全是咨询性的——红了也挡不住合并。**

本 lane 把路径过滤**下沉到 job 内**，让这些作业在**每个 PR 上都报状态**，从而具备
被加进 branch protection 的资格。

`[FACT]` 截至本文（L1-b 完成时）：

- **L1-a**（`rust-backend.yml` / `gm-console.yml` / `frontend-audit.yml`）已落地，
  它的 5 个 context **已经在** `dev` / `main` 的 branch protection 里——父代理在
  PR #60 与 PR #61 上实测过；
- **L1-b**（本 lane，`gm-desktop.yml` / `gm-desktop-bundle.yml`）把另外 9 个 context
  全部**声明**在 `scripts/ci/required-checks.json` 里，但**还没有**进保护；
- 5 个 workflow 的 `pull_request` 触发器全部去掉了 workflow 级 `paths:`，14 个深门禁
  作业名全部登记在册，且每个名字在全部 7 个 workflow 文件里**恰好只有一个拥有者**
  （实测 `contexts=14  problems=0`）。

## 问题：workflow 级过滤为什么让门禁无法被强制

`[FACT]` GitHub 官方文档 "Required status checks" 原文：

> "If a workflow is skipped due to path filtering, branch filtering, or a commit message,
> then checks associated with that workflow will remain in a 'Pending' state. If you have
> required checks, this will block the pull request from being merged."

`[FACT]` 也就是说 workflow 级 `paths:` 有两种结局，都不是「强制」：

1. 把这个作业设成 required check → 它在该跑的时候**永远 pending**，PR 永久卡死；
2. 不设成 required check → 它红不红**完全不影响合并**。

`[FACT]` 本仓正是第 2 种。`docs/plan/v0-tasks.md` 的决策日志在 2026-10-25 11:37 JST
记着（**这是修订前的原文**，该条目后来被同日的两条追加记录更新过）：

「深门禁（打包 / Rust 测试 / 审计 / 两个 ratchet）**仍是 path 过滤的，未被自动强制**」。

`[INFERENCE]` 所以「当时记的是这句话」是事实；「现在仍然如此」不是——那一段的后半
已经改写，见该文件同日 21:50 与 22:35 的两条。

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

`[FACT]` **5 个** workflow 的 `pull_request` 触发器去掉了 workflow 级 `paths:`，每个
**生产者**作业内部加一步检测，重步骤加区域条件：

| workflow | 区域键 | 生产者作业 |
| --- | --- | --- |
| `rust-backend.yml` | `rust_backend` | `fmt + clippy + test`、`dependency audit (RUSTSEC)`、`ST + IT regression (real git over HTTP)` |
| `gm-console.yml` | `gm_console` | `typecheck + lint + test + build` |
| `frontend-audit.yml` | `frontend_audit` | `frontend dependency advisories (npm ratchet)` |
| `gm-desktop.yml` | `gm_desktop` | `cargo audit (tauri crate)`、`lint + check + test + build + cargo check`、`msi (per-user + per-machine)` |
| `gm-desktop-bundle.yml` | `gm_desktop_bundle` | `dmg (unsigned, ad-hoc app)`、`deb (unsigned)`、`appimage (unsigned)` |

`[FACT]` 另有 3 个**消费者**作业——`msi installs and removes itself on a stock Windows
machine`、`the packaged application starts and presents a window`、`deb installs and
links on a stock Ubuntu`——它们不做检测、不带区域条件；每个只多了**一行** job 级
`if: needs.<生产者>.outputs.applies == 'true'`，其余部分与改动前逐字相同。它们
`needs:` 自己的生产者，生产者把区域判定作为 job output 发布给它们（见
「`needs:` 链为什么是安全的」一节）。

`[FACT]` `push` 触发器**保留** workflow 级 `paths:`：直接推分支不是 PR，为每次推 dev
付全部门禁的钱买不到 PR 已经买到的东西。

`[FACT]` 检测步骤**放在作业内部**，不是单独一个 `detect` 作业。理由：如果检测是独立
作业、门禁 `needs:` 它，那么「检测失败」会**跳过**门禁，而被跳过的门禁报 Success ——
绿色放行，正是本 lane 要消灭的死锁。放在内部，检测失败就让**这个作业**变红。

`[FACT]` 检测步骤之前必须 `fetch-depth: 0`：检测要把变更集与 `origin/<base>` 对比，
浅克隆没有那个 merge base。

`[FACT]` `gm-desktop.yml` 的 `build` 作业带 job 级
`defaults.run.working-directory: apps/gm-desktop`，所以它的检测步骤显式写了
`working-directory: ${{ github.workspace }}`。没有这一行，脚本会被当成
`apps/gm-desktop/scripts/ci/changed-areas.ps1` 而找不到，**每一个 PR 都把这个作业
弄红**——包括本 lane 存在的意义所在的那个 PR。

`[FACT]` 诊断性的 `if: failure()` / `if: always()` 上传步骤**刻意不加**区域条件：它们
存在的意义就是把**失败**那一步的日志/覆盖率交出来，加了条件就把唯一有用的那次产出
拿走了。

`[FACT]` 无条件的**产物**上传（`Upload perMachine MSI`、`Upload the .dmg` 等）**加**
区域条件，写作 `if: success() && steps.detect.outputs.<区域> == 'true'`。门禁被跳过
时还上传产物，等于往仓库里放一个没有任何门禁检查过的东西；而作业自身的结论在步骤
全被跳过时是 success，光靠 `success()` 放它不过去。

## 作业重名与发布通路的改名

`[FACT]` `release.yml` 里有 5 个作业的 `name:` 与 PR 通路的作业**逐字相同**：
`msi (per-user + per-machine)`、`dmg (unsigned, ad-hoc app)`、`deb (unsigned)`、
`deb installs and links on a stock Ubuntu`、`appimage (unsigned)`。

`[FACT]` GitHub **没有文档**说明「同一个提交上有多个同名 check run 时哪个算数」。
`release.yml` 又有 `workflow_dispatch` 触发器，所以一次手动发布运行真的可以把两个同名
check 放到同一个 SHA 上。

`[FACT]` 所以这 5 个**发布通路**的作业名加了 `release ` 前缀，作业 **key**
（`msi` / `dmg` / `deb` / `deb-install` / `appimage`）**一个字都没改**——`needs:` 与
`needs.<job>.outputs.*` 引用的是 key，不是显示名；产物名、artifact 名、
`draft-release` 消费的 outputs 也全部走 key 与 outputs。

`[INFERENCE]` **为什么改发布通路而不是 PR 通路**：PR 通路的这些名字是**已经出现在证据
里**的名字。**按显示名引用**它们的记录有——`README.md:152`、`README.md:328`、
`.github/CI.md:150`、`docs/plan/v0-tasks.md:69`（job `appimage (unsigned)`）、
`apps/gm-desktop/tests/unit/msi-install-gate.test.ts:13`、
`apps/gm-desktop/tests/unit/deb-install-gate.test.ts:383`、
`.github/workflows/gm-desktop.yml:681`（`# The Windows counterpart of the …`）——
它们描述的是 PR 的 run 里发生的事，改 PR 通路会让这些记录全部变成过时记述。

`[FACT]` `docs/plan/v0-tasks.md:236` 引用的是 PR 通路作业的 **key**
（`msi` / `dmg` / `deb` / `appimage`），不是显示名；改名不影响它，所以它是同一类
证据、但不是「会因为改名而过时」的那一类。

`[FACT]` 发布通路的作业名只出现在 tag 触发的 run 里，仓里没有任何文档 / 测试引用它们。

`[FACT]` 上面这些行号在本文落笔时**逐条核对过**：每一行都确实写着它被引来证明的
那句名字。一个指错行的引用，正是让记录不再可信的那种东西。

`[FACT]` 改完之后，`required-checks.json` 里那 14 条 context 就各自只有一个拥有者
（实测 `problems=0`）。

### 保真门禁：它自己就是「无法被强制」的原因之一

`[FACT]` `release.yml` 的 `deb` 作业里有一道保真门禁 "The ported install proof still
matches the CI job it was ported from"：它把 `release.yml` 的 `deb-install` 作业块与
`gm-desktop-bundle.yml` 的同名作业块**逐行比对**（忽略注释与空行），不同就让发布失败。
这是 `release.yml` 里**唯一**一处保真比对。

`[FACT]` 实测（2026-10-25）：改动之前两份作业块**完全相同**（`ok … 76 lines compared`）；
给发布通路的 `name:` 加上 `release ` 前缀之后，那道门禁**变红**，差异恰好是一行：

```
-    name: deb installs and links on a stock Ubuntu
+    name: release deb installs and links on a stock Ubuntu
```

`[INFERENCE]` **这道门禁自己就是「.deb 安装证明无法被强制」的原因之一。** 它要求两个
作业块**逐字相同**，于是它**强制**两个作业带上**同一个名字**；而两个作业报同一个
check 名，正是 GitHub 无法消歧义的情形——required context 会有两个可能的拥有者，
哪个算数没有文档。一道为了守住副本诚实性的门禁，反而把这份证明钉死在「无法被要求」
的状态上。

`[FACT]` 所以这道门禁现在断言**两件事**：

1. 两个作业块（**去掉 `name:` 行之后**）**逐行相同**——容器镜像、job 级 shell、每一个
   `run:` 体，判定一点都没有放松；
2. 两个作业的**显示名必须不同**——发布通路那份带 `release ` 前缀，于是 PR 通路那个
   作业是该 check 名的**唯一**拥有者。

`[INFERENCE]` 这不是把门禁放松，是把「标签」从「被门禁顺带约束的东西」变成「被门禁
主动盯住的东西」：以前名字相同是门禁的**副作用**，现在名字相同是门禁的**红灯**。

`[FACT]` 实测三种情形（2026-10-25，本机，直接从 `release.yml` 里抽出那一步的脚本来跑）：

| 情形 | 退出码 | 输出 |
| --- | --- | --- |
| 两份作业块一致、名字不同 | 0 | `ok release.yml deb-install matches gm-desktop-bundle.yml (75 lines compared)`，并打印两个名字 |
| 两份名字相同 | 1 | `both named "..."`，并说明 GitHub 无法消歧义 |
| 行为漂移（把 job 级 `shell: bash` 改成 `sh`） | 1 | `has drifted from the one in gm-desktop-bundle.yml` + unified diff，漂移的行被指名 |

`[FACT]` `apps/gm-desktop/tests/unit/deb-install-gate.test.ts` 读的是作业的 **key**
（`DOC.jobs['deb-install']`），不是它的 `name:`，所以这道门禁的语义变了，那条测试
不需要改。

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

## `needs:` 链为什么是安全的

`[FACT]` GitHub 官方文档 "Using jobs in a workflow" 原文：

> "If a job fails or is skipped, all jobs that need it are skipped unless the jobs use a
> conditional expression that causes the job to continue."

`[FACT]` 于是「上游红 → 下游被跳过 → 下游报 Success」这条通路**确实存在**：如果下游
作业的依赖不是 required context，上游失败会把下游跳过，下游报 Success，于是红色变成
绿色，放行合并。

`[FACT]` 因此 `workflow-gate.test.ts` 里那条断言换成了更强、也更准的不变量：

> **一个 required context 如果有 `needs:`，那么它 `needs:` 的每一个作业也必须是
> required context。**

`[INFERENCE]` 这条不变量恰好堵住那个洞：上游生产者**本身就是 required context**
（`msi (per-user + per-machine)`、`deb (unsigned)` 都在 14 条 context 里），它红就
**自己**挡住合并，下游绿不绿无所谓。三个消费者作业的上游分别是 `msi`、`msi`、
`deb`，全部是 required context，所以这条不变量对它们成立。

`[FACT]` 旧的那条断言（「拥有 required context 的作业**不得**有 `needs:`」）已经被删掉：
`msi-install` / `msi-launch` / `deb-install` **必然**有 `needs:`（它们要上游的产物），
而且它们要成为 required context。对它们断言「不得有 `needs:`」是错的。

`[FACT]` 消费者作业因此**不需要**检测步骤，也**不带**区域条件：区域判定属于它
`needs:` 的生产者；消费者若自己再判一次，就是同一个判定有两个出处，两者可能不一致。
结构测试对此有单独一条断言：消费者作业**不得**带检测步骤，**也不得**在步骤上带区域
条件——一个没有检测步骤的消费者若带上区域条件，那个条件永远为假，每一步都会被跳过，
作业会报绿而**什么都没测**。

### 消费者需要的是「发布的判定」，不只是一个 `needs:`

`[FACT]` **第二次：被跳过的不是生产者，而是生产者的步骤。** 生产者作业本身**从不**
被跳过：区域不匹配时它照常运行（checkout 与检测步骤真的执行），其余步骤被逐个
跳过，作业结论是 **success**。

`[FACT]` GitHub 官方文档 "Using jobs in a workflow"：「If a job fails or is skipped, all
jobs that need it are skipped」——依赖**成功**并不会跳过它的下游。

`[INFERENCE]` 所以 `needs:` 链**不足以**跳过消费者：消费者会照常运行，然后因为找不到
从未上传的产物而失败——在**与桌面无关的 PR 上**让三个 required context 变红。

`[FACT]` 因此消费者需要的是生产者**发布的**区域判定，不只是一个 `needs:`：

- `msi` 发布 `applies: ${{ steps.detect.outputs.gm_desktop }}`；
- `deb` 发布 `applies: ${{ steps.detect.outputs.gm_desktop_bundle }}`（**加在**它已有的
  `version` 输出旁边，不是替换它）；
- `msi-install` / `msi-launch` / `deb-install` 各带一条 job 级
  `if: needs.<生产者>.outputs.applies == 'true'`。

`[INFERENCE]` job 级 `if:` 为假 → 作业**被跳过** → 被跳过的作业报 "Success"——这正是
required check 在门禁不适用时需要的东西。

`[FACT]` 生产者**失败**时 `applies` 为空，消费者同样被跳过并报 Success——而生产者
本身是 required context，它红就**自己**挡住合并。这正是上一节那条不变量所允许的形状：
依赖方是 required context，所以「上游红 → 下游被跳过 → 报 Success」不构成放行。

`[FACT]` 结构测试把这条链钉住：消费者必须有 job 级条件、必须要求
`needs.<生产者>.outputs.applies == 'true'`、生产者必须真的发布 `applies`、而且
`applies` 必须就是区域判定本身。实测（2026-10-25）四种变异**全部**让它变红：消费者
去掉 job 级条件、生产者不再发布 `applies`、生产者的 `applies` 发了别的值、消费者某一步
带上了一个它根本算不出来的区域条件。

`[FACT]` 三个消费者作业**只加了那一行**，没有加注释——它们的解释放在两处：生产者
发布这个值的 `outputs:` 块上方（那里说明了它为什么被发布），以及本节。而真正防止
有人把这一行当成冗余删掉的，是那条结构测试：删掉它，测试立刻变红。

## 路径匹配语义：与 GitHub 对齐到什么程度

`[FACT]` GitHub 官方文档关于 `paths:` 模式：路径模式**从仓库根开始匹配整条路径**，
所以 `'Cargo.toml'` 只匹配仓库根的那个，不匹配 `apps/gm-console/Cargo.toml`；
`*` 不跨 `/`，`**` 跨 `/`；模式必须加引号。

`[FACT]` 本仓实测的 5 份列表里**一个通配符都没有**，只有两种形态：字面文件名，
和 `目录/**`。

`[FACT]` 脚本实现了这两种形态，遇到带 `*` / `?` / `!` 的模式**直接报错退出**，
而不是当字面量比较——后者会让一个门禁**看起来在工作、实际匹配错文件**。

`[FACT]` 脚本用 `git diff --no-renames` 取变更集：git 默认的改名检测**只报改名的
目标路径**，把文件从 `src/**` 移走会让它从变更集里消失，Rust 门禁于是被跳过——
而那正是这个脚本要防的失败。

`[UNVERIFIED-FACT]` **判定 `paths:` 是否匹配的组件不开源。** 同一个来源指出，
唯一可靠的答案是一次 push 之后读 Actions tab。因此「本地实现与 GitHub 行为等价」
是 `[INFERENCE]`，不是 `[FACT]`。真正的验证在「验证计划」一节。

## 验证计划（合并之后做，不在本 lane）

`[FACT]` L1-a 的 5 个 context **已经**在 `dev` / `main` 的 branch protection 里，父代理
在 PR #60 与 PR #61 上实测过。

`[PROPOSAL]` 合并 L1-b 之后，父代理做两件事：

1. 把本文件新增的 9 个 context 加进 `dev` / `main` 的 branch protection required checks；
2. 用**两个真实 PR** 实证：
   - 一个**只改 `docs/**`** 的 PR → 14 个 context 全部**出现**且为绿（不是 pending）；
   - 一个**改了 `src/**`** 的 PR → rust 门禁**真的跑起来**（看日志里有 cargo 的输出，
     不是空跑）。

`[FACT]` 加 required check 有一个自举问题需要按顺序处理：改 branch protection 的那个
PR 自己也必须满足新的 required check。所以顺序必须是「先合并让作业会报告的 PR，
再加 required check，再用后续 PR 验证」。

`[FACT]` 作业名一旦改名，等于换掉 required check，而仓里没有任何东西会察觉。
`workflow-gate.test.ts` 就是为此存在：它断言每个 context 名字在全部 workflow 文件里
**恰好出现一次**，且**不含 `${{`**。

`[FACT]` **YAML 能解析 ≠ GitHub 接受。** GitHub 的 workflow 解析器是它自己的，
只能靠 PR 上线验证。

## 风险与回滚

| 风险 | 缓解 |
| --- | --- |
| 检测失败 → 门禁红 | 刻意如此。检测失败说明变更集算不出来，此时「什么都没变」是最危险的读法。脚本对这种情况**非零退出**，不静默放行。 |
| 区域模式与 GitHub 语义有偏差 | 模式列表被测试钉住；语义偏差只能在 GitHub 上实测发现，所以验证计划里有 docs-only 与 src-touching 两个 PR。 |
| required check 名字对不上 → PR 卡死 | `workflow-gate.test.ts` 断言每个 context 恰好一个作业拥有；发布通路的 5 个重名作业已改名（见上）。 |
| 两个作业报同一个 check 名 → required check 无法消歧义 | 发布通路那 5 个作业名带 `release ` 前缀；`release.yml` 的保真门禁现在**断言**两个名字不同；结构测试断言每个 context 恰好一个拥有者。 |
| `needs:` 链把红色变成绿色 | 结构测试断言：**required context 若有 `needs:`，它需要的每一个作业也必须是 required context**。上游生产者本身就是 required context，它红就自己挡住合并。 |
| 消费者作业在区域不适用时仍然运行 | **已修**：生产者把区域判定作为 job output 发布，消费者用 job 级 `if:` 要求它；区域不适用时消费者被跳过并报 Success。结构测试钉住这条链的每一环。 |
| 消费者步骤上带了算不出来的区域条件 | 结构测试断言消费者**不得**在步骤上带区域条件——那会让每一步都被跳过，作业报绿而什么都没测。 |
| 回滚 | 恢复 workflow 级 `paths:` 即可，**但必须同时**把 branch protection 的 required checks 改回去，否则 PR 会永久 pending。 |

## 本 lane 明确没做的事

`[FACT]` **本 lane（自己）没有改分支保护。** 保护是在 lane 之外、由父代理改的，
分两次：`repo gate (always runs)` 加 L1-a 的 5 条在 PR #60 / PR #61 上实测过，
L1-b 的 9 条在 PR #62 上实测过（那 9 条在 #62 上全部真跑并通过）。**2026-10-25
23:20 JST 起，`dev` / `main` 的 required check 是 `repo gate (always runs)` 加
全部 14 条 context**，只 PATCH 了 `required_status_checks`，其余开关前后逐一核对
未变。

`[FACT]` **这一节写作时的旧事实已经变了**：本段原本写着「14 条 context 只是声明，
其中 9 条还没有进保护」。**那句现在不成立**，所以改在这里，而不是留着它让读的人
以为桌面与打包的检查是可选的。

`[FACT]` **三个消费者作业（`msi-install` / `msi-launch` / `deb-install`）各只加了
一行 job 级 `if:`**，其余部分与改动前逐字相同——它们不做检测、不带区域条件。
那一行是必需的，理由见「`needs:` 链为什么是安全的」一节。

`[FACT]` **PR #63 上实测过这一行**：那个 PR 只改了两个不属于任何区域的文件，于是
`msi-install` / `msi-launch` / `deb-install` 三个消费者**整体被跳过**（GitHub 记作
`skipping`），而它们的三个生产者 `msi` / `deb` / `dmg` 是「照常运行、步骤全跳过、
报绿」（9–16s）。这正是「不适用 → 报状态 → 不挡合并」的完整形态。

`[FACT]` **没有动 `apps/gm-console/**`、`scripts/frontend-advisory-baseline.json`** ——
那是另一条 lane（PR #58，gm-console 工具链升级）正在改的文件。

`[FACT]` **没有动 `scripts/ci/changed-areas.ps1` 的模式列表。** 它是覆盖范围的防漂移
锚点，改了就等于改了「门禁覆盖什么」。

`[FACT]` **没有改任何 `needs:` 链、作业 key、artifact 名**，也没有削弱、跳过或删除
任何既有检查——保真门禁的改动是**加**了一条断言（两个名字必须不同），不是减一条。

## 本 lane 实测发现的两处「文档行为 ≠ 设计假设」，以及它们怎么修的

`[FACT]` 这两处都是**实测**得到的，不是推测。两处都**不是**本 lane 的设计错误，
而是本 lane 依赖的 GitHub 文档行为与设计的前提不一致——两处都已修掉。

`[INFERENCE]` 两处的共同形状值得写下来：**一个被记录的 GitHub 行为，在设计里被当成了
另一个行为。** 第一处把「作业被跳过」当成了「作业的步骤被跳过」，第二处把「两个作业块
逐字相同」当成了「两个作业块的行为相同」。文档里被引用的行为与被设计的行为不是一回事，
所以设计依赖的每一条文档行为都值得逐字核对。

### 一、消费者作业：被跳过的不是生产者，而是生产者的步骤

`[FACT]` brief 的 §4.2 假设：「上游因为区域不匹配而被**跳过**时，消费者作业也会被
跳过，而被跳过的作业报 Success」。这条假设**不成立**。

`[FACT]` 区域不匹配时，**生产者作业本身并没有被跳过**——它跑了 checkout 和检测步骤，
其余步骤被 `if:` 跳过，所以作业的结论是 **success**，而不是 skipped。

`[FACT]` GitHub 官方文档 "Using jobs in a workflow"：「If a job fails or is skipped, all
jobs that need it are skipped」——依赖**成功**时，下游**照常运行**。

`[INFERENCE]` 后果（修好之前）：区域不适用时，`msi-install` / `msi-launch` 会去下载
`gm-desktop-msi-*` 这个 pattern，`deb-install` 会去下载
`gm-desktop-deb-${{ needs.deb.outputs.version }}`；而那些产物这一轮**根本没有上传**
（无条件的产物上传已经带上了区域条件）。`actions/download-artifact@v4` 找不到产物时
**失败**，所以这三个 context 会在**与桌面无关的 PR 上变红**。

`[FACT]` **已修**：生产者把区域判定作为 job output 发布，消费者用 job 级 `if:` 要求它
（详见「消费者需要的是「发布的判定」，不只是一个 `needs:`」一节）。结构测试钉住这条链
的每一环，四种变异实测全部变红。

### 二、保真门禁：它自己就是「无法被强制」的原因之一

`[FACT]` 见上文「保真门禁：它自己就是「无法被强制」的原因之一」一节。那道门禁原本
要求两个作业块**逐字相同**，于是它强制两个作业带上**同一个名字**——而两个作业报同一个
check 名，正是 GitHub 无法消歧义的情形。那道门禁现在**改**成：去掉 `name:` 行之后
仍然逐行相同，**并且**两个名字必须不同。

## 一次被测出来的假阳性，以及本 lane 自己的重复实验

`[FACT]` 本地跑 gm-desktop 完整测试套件时，`tests/unit/workflow-step-isolation.test.ts`
里 `release.yml verify-tag` 那几条用例**偶发** 5s 超时（实测 5448ms / 6264ms /
7203ms）。该用例会**真实 spawn bash 进程**执行 release.yml 的 verify-tag 步骤。

`[FACT]` 做了**重复** A/B（同一台机器、同样负载，各跑 3 次完整套件）：

- **A = 本 lane 的工作树**（最终树）：3 次全部 exit 1，失败的都是上面那几条超时用例
  （三次分别 2 条、1 条、1 条失败）；
- **B = 把 `release.yml` 换回 HEAD 版本**（即不含本 lane 的改名）：3 次也全部 exit 1，
  失败的是**同样**那几条超时用例，**外加** `each name exactly one job, and no job twice`
  ——因为不改名时那 5 个 context 各有两个拥有者。

`[INFERENCE]` 结论：超时**在有本 lane 改动和没有本 lane 改动的两侧都出现**，所以与本
lane 无关；而「恰好一个拥有者」那条断言**只有**在本 lane 改名之后才成立——那正是
改名本身要证明的东西。

`[FACT]` 另外单独跑那一个测试文件（负载更低），在**最终**的工作树上各跑 6 次：
带本 lane 改动 **5 次过、1 次失败**；换回 HEAD 的 `release.yml` **也是 5 次过、1 次失败**。
两侧失败率相同。

`[FACT]` 本 lane **没有**改那个测试，也没有放宽它的超时——它是预存在的、负载相关的
波动。

`[INFERENCE]` 这正是「只跑一次不叫测量」的又一个例子：单次「stashed 通过 / 带改动失败」
是巧合，不是因果；重复实验才说得清。
