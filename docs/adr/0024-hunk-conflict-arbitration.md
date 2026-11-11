# ADR-0024: 冲突区域（Region）粒度仲裁 — Scope 层 + AI 建议对话框

| 字段 | 值 |
|---|---|
| **Status** | Proposed (2026-11-05) — 设计已定稿，实现分三 lane 落地 |
| **Supersedes** | (无) |
| **Superseded by** | (无) |
| **Authors** | Ulysses — Mavis 接手 agent |
| **Reviewers** | Ulysses (DDD Review, pending) |
| **Deciders** | Ulysses |
| **Tags** | gitgit, conflict, hunk, region, scope, ai-arbitration, merge, tauri, svelte, vault |

---

## 0. 标签约定 / Fact grading

本 ADR 的每一条事实性断言都带一个标签，含义固定：

| 标签 | 含义 |
|---|---|
| `[FACT]` | **本次实测**得到的结果。给出复现命令或复现步骤。 |
| `[INFERENCE]` | 由 `[FACT]` 推导，推理链在正文里写出来。 |
| `[PROPOSAL]` | 本 ADR 的设计提案，尚未实现。 |
| `[UNVERIFIED-FACT]` | 外部断言，本次未实测，落地 lane 必须补测。 |

> 本 ADR 只做设计。**没有 Rust、没有 Svelte、没有 workflow 改动。**
> 仓库的 ADR 先于代码评审（per lane brief 的「Rules」段）。

---

## 1. 背景 / Context

### 1.1 产品需求（原文，verbatim）

> 你要突破传统git以文件为单位判定冲突的局限，在ai并行开发的时代，应该以更小的单位判断冲突，并可以在文件内发生冲突时，弹窗允许用户判断是否使用填写好api key的ai智能取舍

拆成三个可执行命题：

1. **判定单位变小**：突破 git 的「文件为单位」判定冲突。
2. **弹窗让用户裁决**：文件内发生冲突时，弹窗让用户决定要不要交给「填好 api key 的 AI」。
3. **AI 是选项不是自动**：用户不选 AI，流程照常跑完。

### 1.2 本仓现状（实测）

- `[FACT]` 本仓**目前没有任何冲突处理代码**。全仓 `grep -i conflict`（`*.rs` / `*.ts` / `*.svelte`）只命中若干**无关**文本：`src/server/vault_versioned.rs:257`（`ON CONFLICT` 注释）、`src/remote/mod.rs:8`、`src/ai/registry.rs:5` 与 `:8`（「这两个计数不冲突」之类的散文），以及 `apps/gm-desktop/src-tauri/src/state.rs:286`、`:296`（描述「review session 撞车」的措辞）。**没有 merge driver、没有 index stage 读取、没有冲突区域解析。**
- `[FACT]` 本仓已有的 diff 机器是 `src/repo/status.rs`：`worktree_status`（`:504`）、`worktree_diff`（`:535`）、`DiffTarget` 三目标枚举（`:124`）、`git_output` 白名单 argv 助手（`:448`）。`git_output` 的注释明确写了「参数以 vector 传入，绝不拼 shell 字符串」，并且禁用 `--no-ext-diff` / `--no-textconv`（`:441-447`）。
- `[FACT]` 本仓已有的 AI 机器是 `src/ai/`：`AiProvider` trait（`src/ai/provider.rs:158`）、`ProviderRegistry` 五个预置（`src/ai/registry.rs:132-175`）、两条线路协议（openai-compatible / anthropic）、`sanitize` 不可信内容纪律（`src/ai/sanitize.rs:36`、`:299`、`:321`）、环境变量密钥规则（`src/ai/mod.rs:56` + 测试 `:151`）。
- `[FACT]` 本仓的桌面端已有 `/review` 路由，走流式 AI（`apps/gm-desktop/src/App.svelte:37-46`、`apps/gm-desktop/src/routes/Review.svelte`、`apps/gm-desktop/src/lib/stores/review.ts`、`apps/gm-desktop/src/lib/api/ai.ts`）。
- `[FACT]` 本仓的凭据库是 `Vault` trait（`src/server/vault.rs:187-222`）+ `VersionedVault` 子 trait（`src/server/vault_versioned.rs:185`，其中 `set_with_version` 在 `:199`）。
- `[FACT]` 桌面端目前**没有**任何让用户输入 AI key 的入口。`apps/gm-desktop/src/lib/i18n/en.ts:224` 的文案原话是「the key is never entered here」；`apps/gm-desktop/src-tauri/src/commands/ai.rs:11-18` 的不变量原话是「**The key comes from the environment only.**」。

> **这最后两条是本 ADR 要改的东西。** 不是改 `GITGIT_AI_API_KEY` 本身（它继续有效、继续是 CLI 的唯一来源），而是给**桌面端**增加第二个来源：版本化凭据库。§3.10 给出交互与优先级。

### 1.3 为什么要做（不是「因为 AI 来了」）

`[FACT]` 本仓历史的实测（§2.2）显示：一个发生冲突的文件，平均携带 **1.59 个互相独立的冲突区域**，**36%（8/22）** 的冲突文件带 **≥2 个**独立区域。也就是说 git 的「文件级」判定把 35 次裁决压成 22 次「这个文件冲突了」，用户要么在文件编辑器里手工拆，要么被告知「这个文件整体有冲突」。**更小的单位不是审美偏好，是把裁决次数从「文件数」降到「区域数」的实测收益。**

---

## 2. 实测台账 / Measurements

### 2.1 git 版本与 `git merge-file` 行为

环境：Windows / PowerShell 7.6.8 / `git version 2.41.0.windows.1`。

- `[FACT]` **CI 与开发环境的 git 下限是 2.41**：`docs/adr/0020-v0-gui-tauri-svelte.md:88` 记录「Git 2.41.0 ✅」（环境核实 2026-08-26 17:00 JST），`docs/plan/v0-tasks.md:358` 同源记录。本机 `git --version` 实测同为 `2.41.0.windows.1`。
- `[FACT]` `git merge-file` 在 2.41 的 usage 行**只有**这些选项：
  ```
  usage: git merge-file [<options>] [-L <name1> [-L <orig> [-L <name2>]]] <file1> <orig-file> <file2>
      -p, --stdout          send results to standard output
      --diff3               use a diff3 based merge
      --zdiff3              use a zealous diff3 based merge
      --ours / --theirs / --union
      --marker-size <n>     for conflicts, use this marker size
      -q, --quiet           do not warn about conflicts
      -L <name>             set labels for file1/orig-file/file2
  ```
  **`--object-id` 不在 2.41 的 usage 行里**，所以不能依赖它。
- `[FACT]` `git merge-tree` 在 2.41 的 usage 行里**有** `--write-tree`（实测 `git merge-tree -h`）。但 `merge-tree` 是**仓库级**操作（吃两个 branch/commit，吐 tree），而本产品需要的是**单路径的三个 blob**（可能是尚未进入 index 的路径）。见 §3.2 的取舍。
- `[FACT]` **`git merge-file --diff3 -p <ours> <base> <theirs>` 的退出码 = 冲突区域个数。** 实测：1 个区域 → exit 1；2 个区域 → exit 2；无冲突 → exit 0。
- `[FACT]` **`-L` 标签可用**，实测 `-L OURS -L BASE -L THEIRS` 在输出里分别贴到 ours / base / theirs 三侧。
- `[FACT]` **标记尺寸歧义是真的。** 实测：base 内容里有一行 `<<<<<<< not a real marker` 时，`--diff3` 的输出把这行**原样**排在真标记**之前**：
  ```
  alpha
  <<<<<<< not a real marker          <-- base 的正文，被 naive parser 当成标记
  <<<<<<< ...\ours.txt               <-- 真标记
  ```
  用 `--marker-size=24` 后，正文行仍是 7 字符，真标记是 24 字符，两者可区分：
  ```
  <<<<<<< not a real marker
  <<<<<<<<<<<<<<<<<<<<<<<<< ...\ours.txt
  ```
  ⇒ **解析器必须用 `--marker-size=24`，不能用「按 `<<<<<<<` 切」这种写法。**
- `[FACT]` **`merge-file` 只吃磁盘文件，不吃 blob。** 实测：三个 blob 必须先落成三个临时文件再传给 `merge-file`。这与 `src/repo/status.rs:616` 的 `empty_temp_file()`（`unique_temp` 约定、无 `tempfile` 依赖，per `:614-615`）是同一套做法。

### 2.2 本仓历史的 scope 调查（本次核心测量）

方法（在 `origin/dev` 上跑）：

1. 取全部 315 个 commit（其中 50 个 merge commit），只用 265 个非 merge commit。
2. 按文件聚合「被哪些非 merge commit 改过」→ 569 个文件被 ≥2 个 commit 改过 → 共 **3,323 个同文件 commit 对**。
3. 排除祖先关系（其中一个是另一个的祖先）→ **136 个分叉对**（divergent pair）。
4. 分叉对再按 merge base 取三份 blob：base = `git merge-base c1 c2`，ours = `c1:file`，theirs = `c2:file`。其中 31 对因 base 处无该文件被跳过 → **实测 105 对**。
5. 对每一对跑 `git merge-file --diff3 -p ours base theirs`，用**退出码**判冲突；再用 `git diff --unified=0` 取 base 侧改动行区间，映射到 scope。

**Scope 规则 v1（ADR 提案的规则）**：文件被切成连续 scope；scope 0 是前言（第一个 item 之前的所有行）；item 是**列 0** 的顶层声明行（Rust：`use/mod/fn/struct/enum/trait/impl/const/static/type/macro_rules!` + `#[`；TS/JS/Svelte：`import/export/function/class/interface/type/const/let/var/describe/it/test`）；item 正上方连续的注释 / `#[` 行**并入**该 item；无法切分的扩展名（`.md` / `.yml` / `.json` / `.ps1` / `.sh` / `.toml` …）**整文件算一个 scope**（保守兜底）。

**Scope 规则 v2（更细）**：同上，但 item 行**任意缩进**都算边界（v1 只认列 0）。实测 v2 把冲突文件的平均 scope 数从 **7.86 抬到 17.86**（`state.rs` 36→70、`review.test.ts` 18→83），即 v2 确实是更细的切分。

**结果（全部 `[FACT]`）**：

| 指标 | 数值 |
|---|---|
| 同文件 commit 对（全部，含祖先对） | 3,323 |
| 分叉对（neither-ancestor） | 136 |
| 实测对（base 处有该文件） | **105** |
| `git merge-file` 判**有冲突** | **22** |
| `git merge-file` 判**干净** | **83** |
| 冲突区域总数 | **35**（14 对 ×1 区 + 3 对 ×2 区 + 5 对 ×3 区） |
| 冲突文件带 ≥2 个独立区域 | **8 / 22 = 36%** |
| 每个冲突文件的平均区域数 | 1.59 |
| **v1：冲突对里 ours/theirs 改动 scope 相交** | **22 / 22** |
| **v1：冲突对里 scope 不相交（= 被 scope 层藏起来的冲突）** | **0** |
| **v2：同上** | 22 / 22 相交，**0** 被藏 |
| 干净对里 scope 相交（v1 / v2） | 83 / 83 |
| 我的「base 行区间重叠」代理口径判 line-disjoint | 2 / 105 |
| **那 2 对里，scope 层也判 disjoint 的** | **0 / 2**（2 对都是 line-disjoint **但** scope-overlap ⇒ scope 层比行区间代理更保守） |

**对这张表的诚实解读（这是本 ADR 最容易被读错的地方）**：

- `[FACT]` 「2 / 105 line-disjoint」**不是** git 的行级判定结果。git 的行级三方合并把 **83 / 105** 判为干净。我的行区间代理只有 2 对不重叠，原因是**同一锚点上的双方插入**（`@@ -319,0 @@` vs `@@ -319,0 @@`）被我算成重叠，而 git 把它们干净合并。⇒ **权威口径是 `merge-file` 的退出码**（22 冲突 / 83 干净），不是我的区间代理。
- `[FACT]` 「scope 不相交 = 0」在 v1 和 v2 下**都成立**：两种粒度下，22 个冲突对里 ours 与 theirs 改动到的 scope **全部相交**。
- `[INFERENCE]` 由此得到本 ADR 最关键的一条结论：**在当前两版 scope 规则下，scope 层一次都没有「缩小过」冲突集合。** 它把冲突**定位**到了 scope（平均 7.86 / 17.86 个 scope 可选），但**没有一次**判定为「这里其实没冲突」。
- `[INFERENCE]` 而「scope 不相交」在**干净对**里也是 0（83/83 全部相交）。这说明 scope 层作为「文件级过滤器」会比 git 更保守（git 判干净的它仍报相交）。⇒ **scope 层不能单独充当「这个文件到底冲不冲突」的判定器**，它必须作用在 `merge-file` 已经产出的**冲突区域**上（§3.9）。
- `[FACT]` 测量口径的三个已知偏差（必须写进 ADR，不能藏）：
  1. 测量是**对级**（整文件改动集合），不是**区域级**；区域级的细分是 §7 的后续测量项。
  2. 纯插入 hunk 归到**锚点行所在 scope**（保守方向：多报相交）。
  3. 105 对里 **44 对**落在不可切分扩展名（`.md` 16 / `.yml` 15 / `.json` 5 / `.ps1` 3 / `.sh` 2 / `.yaml` 2 / `.toml` 1），按定义整文件一个 scope；可切分的 61 对（`.ts` 35 / `.tsx` 11 / `.rs` 12 / `.svelte` 3）结论相同。

### 2.3 一次真实冲突 merge 的 index stage（实测）

在一个临时仓库里造出一次真冲突（`main` 与 `side` 各改 `f.txt` 的同一行）后实测：

- `[FACT]` `git status --porcelain=v1` 对未合并路径输出 `UU f.txt`。
- `[FACT]` `git ls-files -u` 列出三条 stage，每条带一个 blob OID：`1`=base、`2`=ours、`3`=theirs。
- `[FACT]` **`stage 1` 的内容与 `git merge-base` 处的该文件内容逐字节相同**（实测 `True`）。⇒ **进行中 merge 的 index stage 1 就是 merge base**，不需要再单独算一次 merge-base。
- `[FACT]` `git show :1:f.txt` / `:2:` / `:3:` 分别取到 base / ours / theirs 三份 blob。⇒ **「三个版本是 blob，不是进行中的 merge」这件事，在 index 里是现成的**（`git show :N:path`）。
- `[FACT]` 把这三份 blob 落盘后喂给 `git merge-file --diff3 --marker-size=24 -L OURS -L BASE -L THEIRS -p`，得到：

  ```
  line1
  <<<<<<<<<<<<<<<<<<<<<<<<< OURS
  OURS
  |||||||||||||||||||||||| BASE
  line2
  ========================
  THEIRS
  >>>>>>>>>>>>>>>>>>>>>>>> THEIRS
  line3
  ```
  退出码 1。

---

## 3. 决策 / Decision

### 3.0 单位：冲突 REGION，不是文件，也不是单行（已决，但在此论证）

**决策**：仲裁单位是**冲突区域（Region）** —— 一段连续的冲突文本，带它前后的上下文。

**为什么不是单行**：冲突在语义上通常是**一块**。两分支各改同一个函数的相邻两行，逐行让用户裁决会把「这函数该怎么写」拆成两个互不相干的决定，用户无法在不读全块的情况下做判断。`[INFERENCE]`

**为什么不是文件**：文件是 git 的粒度，也正是需求要突破的那一层。`[FACT]` §2.2 实测：35 个区域 vs 22 个文件，8 个文件（36%）含多个独立区域，文件粒度把裁决次数压低了 1.59 倍。

**为什么不是「hunk」**：git 的 hunk 是 diff 输出的**上下文窗口**，是展示单位，不是语义单位。区域是**冲突**的单位（两侧都改动了同一段 base 文本），两者不同。文件名里的 `hunk` 取的是「比文件更小的块」这层意思，实现上落在 **region** 上。

### 3.1 Q1：冲突区域从哪来？

**决策**：**用 `git merge-file` 产出合并文本，再解析冲突标记。** 不自己实现三方合并。

具体契约（`[PROPOSAL]`）：

1. 输入是三个 blob：ours / base / theirs。
   - 进行中 merge 的主流程：base = index stage 1，ours = stage 2，theirs = stage 3（`[FACT]` §2.3 实测 stage 1 == merge base）。
   - 「预览两分支」的辅助流程：base = `git merge-base ours theirs` 的该路径 blob。
2. 三个 blob 落成三个临时文件（`[FACT]` `merge-file` 只吃文件；复用 `src/repo/status.rs:616` 的 `unique_temp` 约定）。
3. 命令固定为：
   `git merge-file --diff3 --marker-size=24 -L OURS -L BASE -L THEIRS -p <ours> <base> <theirs>`
   - `--diff3`：保留 base 侧内容，用户判断「谁的改动更合理」时需要看到 base（§3.4）。
   - `--marker-size=24`：**必须**，否则 base 正文里的伪标记会让解析器 fooled（`[FACT]` §2.1）。
   - `-L`：让三侧标签无歧义（`[FACT]` §2.1）。
   - `-p`：结果走 stdout（usage 行原话 `send results to standard output`）。`[FACT]` 本 lane 上一轮测量记录：三个输入文件在调用前后内容不变（per lane brief 的已测台账）。
4. **退出码 = 区域个数**（`[FACT]` §2.1）。⇒ 区域数量**不需要**再数一遍标记；这是最省事也最不易错的一条。
5. 解析：按 24 字符标记切，产出 `Region { index, ours_text, base_text, theirs_text, base_line_range, merged_line_range }`。
   `[PROPOSAL]` 解析器必须**按标记宽度切**，不得按「包含 `<<<<<<<` 的行」切。

**为什么不用 `git merge-tree --write-tree`**（`[FACT]` 它在 2.41 可用）：它是**仓库级**的，参数是 branch/commit，输出是一棵 tree；它不解决「我要仲裁某一个路径的、可能还没进 index 的三个 blob」这个需求。而且它不给出 base 侧正文（没有 `--diff3` 等价物），而 base 侧正文是仲裁必需的。

**为什么不用 `git merge-index` / `git checkout --conflict=diff3`**：`[FACT]` 二者都以「索引里已有冲突」为前提，且 `checkout --conflict` 会**改写工作区文件**。本产品的第一交付只**读**冲突、不驱动 merge（§3.7、§4）。

**为什么自己算三方合并**：`[FACT]` 仓库的既定立场是「绝不重新实现高风险 Git 设施」（`src/server/subprocess.rs:3-6`）；三方合并（xdiff）正属于此列，且自己实现会引入 0-new-deps 冲突（xdiff 是 C）。

**什么会让我们改主意**：`[PROPOSAL]` 若 `merge-file` 在目标 git 版本上不再提供 `--marker-size`，或若我们需要 `merge-file` 给不出的信息（如「这个区域两侧改动是否语义等价」），则改为在 Rust 侧实现 diff3 的**区域切分**（仍然用 git 做差异计算，只自己做标记装配）。

### 3.2 Q2：merge base 要不要？

**决策**：**要，而且 base 是必需的，不是可选的。**

**论证**：

- `[FACT]` `merge-file` 是**三方**合并（`<file1> <orig-file> <file2>`）。没有 base，只有 ours 和 theirs，得到的只能是二方 diff / combined diff，**无法判定「谁替换了 base 的哪一段」**，也就无法产出冲突区域。
- `[INFERENCE]` 更产品化地说：base 侧正文是用户判断「谁的改动更合理」的**唯一参照**。只有 ours/theirs 的话，用户看到的是两个都「凭空出现」的版本，无法仲裁。
- `[FACT]` base 在主流程里**零成本**：`[FACT]` §2.3 实测，进行中 merge 的 index stage 1 **就是** merge base（内容逐字节相同）。所以「需不需要 merge base」这个问题在主流程的答案是：**需要，但已经在手边了**，不需要额外跑 `git merge-base`。
- `[PROPOSAL]` 辅助流程（用户还没 merge，只是想看看两分支会不会撞）才需要 `git merge-base ours theirs`。

**决策细化**：

- `[PROPOSAL]` base 的来源优先级：index stage 1 > `git merge-base <ours> <theirs>`。
- `[PROPOSAL]` **base 不可得时（任一 blob 缺失 / 路径在 base 处不存在 / 文件是二进制）**，该路径**不进入冲突仲裁**，直接标为「需要 git 原生处理」，对话框只显示提示，不假装能仲裁。

**什么会让我们改主意**：`[FACT]` **rename 场景已实测**（Lane A 的验收测试 `a_rename_conflict_is_left_to_git`，git 2.41.0.windows.1）：两侧把同一个文件改成不同名字时，未解决的 index 把**同一个 blob 的三个 stage 摊在三个不同路径上**——stage 1 在旧名字下，stage 2 / stage 3 各自在一边的新名字下。因此**没有一条路径同时带三个 stage**，三方合并不成立；「stage 1 = merge base」这句话在 rename 下只对**带着 stage 1 的那条路径**成立（该路径的 stage 1 blob 与 merge base 的 blob 逐字节相同）。Lane A 的处置：**这些路径不进入冲突仲裁**，直接标「需要 git 原生处理」（§3.2 的 base 不可得条款）。**其余合并策略（非 ort）仍未实测，那部分仍是 `[UNVERIFIED-FACT]`。**

### 3.3 Q3：AI 拿到什么？

**决策**：**一个区域一次请求**；system 段是本仓自己写的（trusted），user 段整块是**不可信**内容，走 `sanitize::sanitize_request`。

**送什么（`[PROPOSAL]`，逐项）**：

| 送入项 | 内容 | 信任级别 |
|---|---|---|
| system prompt | 「你是一个合并冲突仲裁器。只输出替换文本，不要解释、不要 markdown 围栏」+ 仲裁规则 | Trusted（`ChatMessage::system`） |
| repo 相对路径 | `src/foo.rs` | 随 user 段一起当不可信 |
| 文件语言 / 扩展名 | `rust` / `typescript` | 随 user 段一起当不可信 |
| **ours 侧** | 冲突区域在 ours 版本里的原文 | 不可信 |
| **base 侧** | 冲突区域在 base 版本里的原文 | 不可信 |
| **theirs 侧** | 冲突区域在 theirs 版本里的原文 | 不可信 |
| 上下文 | base 文件里冲突区域**前后各 ≤20 行**，**不含**其它区域 | 不可信 |

**为什么给 base**：`[INFERENCE]` 用户自己的判断依据就是 base；不给 base，AI 只能猜「这两个版本哪个更像对的」，而 base 才是「原来的样子」。不给 base 的 AI 建议，本质是让模型二选一 —— 而 §3.4 的方案里用户本来就能自己二选一，AI 就没有增量价值。

**为什么给上下文（各 ≤20 行）**：`[INFERENCE]` 冲突区域常常是一行（实测 §2.3 的例子就是单行冲突），孤立地给一行，模型无法判断这行在函数签名里、在 import 列表里、还是在字符串字面量里。20 行是**保守**取值，够看出上下文又不至于把整个文件塞进 prompt。`[FACT]` 本仓已有的同类夹紧是 `MAX_DIFF_CHARS = 60_000`（`src/ai/prompt.rs:78`）—— 本请求沿用同一个上限与同一条「被夹紧必须可见告知」的规则（`src/ai/prompt.rs:86-96`）。

**为什么整块当不可信**：`[FACT]` 冲突区域在「对方分支是别人的」时候是攻击者可控文本。`[FACT]` 本仓已有纪律：`Trust::Untrusted`（`src/ai/provider.rs:28-35`）、`ChatMessage::untrusted`（`:99-105`）、`UNTRUSTED_SYSTEM_PROMPT`（`src/ai/sanitize.rs:36-42`）、`wrap_untrusted`（`:299-308`）、`sanitize_request`（`:321`）。`[FACT]` `ChatMessage` 的 `trust` 默认值是 `Untrusted`（`src/ai/provider.rs:81-84`）—— 忘记分类的调用方走的是**保护**路径。

**必须复用本仓既有流程，而不是新写一套**：`[FACT]` `Invocation::run` 的顺序是「先拼 prompt，再 sanitize」（`src/ai/mod.rs:97-135`，注释在 `:93-96` 解释为什么 sanitize 必须在 prompt 之后）。本请求沿用同序：`[PROPOSAL]` 新的 `prompt::conflict(region)` 产出 `(system, untrusted_user)`，然后 `sanitize::sanitize_request(&mut req)`。

**必须写进 UI 的事实**：`[FACT]` 脱敏是**启发式**的（`src/ai/sanitize.rs:18-23`：「defence in depth, not a guarantee」）。⇒ `[PROPOSAL]` 冲突对话框要显示「本次发送前替换了 N 处疑似机密」，与 `/review` 的 `review.redacted` 一致（`apps/gm-desktop/src/lib/i18n/en.ts:222`）。

**什么会让我们改主意**：若实测发现上下文 20 行导致模型给出的替换文本大量引入上下文里的符号（实测指标见 §7.2），则调整上下文行数或改为「整函数」。

### 3.4 Q4：AI 的答案怎么呈现、怎么校验？

**决策**：**流式到达 → 校验 → 以 diff 呈现 → 用户确认 → 才落地。AI 自己一个字都不写。**

#### 3.4.1 流式 vs 一次性

- `[FACT]` 本仓的 AI 传输是**流式**的：`AiProvider::send_stream` 返回 `mpsc::Receiver`（`src/ai/provider.rs:207-209`、`:175-206`），`StreamEvent::Token(String)`（`src/ai/stream.rs:128-130`），错误是**带内**的（`src/ai/provider.rs:192-197`：中途失败与干净结束是两种结果，UI 分不出来就会把截断的答案当成完整的）。
- `[FACT]` **Anthropic 没有流式实现**：`ProviderSpec::supports_streaming` 对 Anthropic 恒为 false（`src/ai/registry.rs:98-103`），且 `send_stream` 的默认实现是**拒绝**而不是回退（`src/ai/provider.rs:199-209`）。
- `[PROPOSAL]` 因此：**「问 AI」这个选项只在 `supports_streaming == true` 的 provider 上出现。** Anthropic 用户拿到 keep ours / keep theirs / 手工编辑，外加一句明说原因的话（沿用 `review.unsupported` 的措辞风格，`apps/gm-desktop/src/lib/i18n/en.ts:223`）。**不做「一次性兜底」** —— `[FACT]` 本仓的立场是「不能流式的 provider 必须说不能」（`src/ai/provider.rs:199-206`），悄悄回退成非流式会让用户盯着一块不动的 UI 以为卡死了。
- `[FACT]` 前端纪律照抄 `/review`：**订阅先于启动**（`apps/gm-desktop/src/lib/stores/review.ts:8-11`）、**非当前 sessionId 的事件丢弃**（`:152`）、**没有终止事件就等于失败**（`:19-20`）、**累积文本有上限** `REVIEW_MAX_TEXT_CHARS = 200_000`（`:36`）。

#### 3.4.2 答案的校验（在展示之前）

`[PROPOSAL]` 一条 AI 答案必须**全部**通过下列检查才允许出现在「建议」槽位：

| # | 检查 | 不通过时 |
|---|---|---|
| 1 | trim 后非空 | 拒绝，理由「空建议」 |
| 2 | **不含**任何冲突标记族（`<<<<<<<` / `|||||||` / `=======` / `>>>>>>>`，含 24 字符形态） | 拒绝，理由「建议里带冲突标记」 |
| 3 | 长度有界（≤ 区域 ours 侧行数 × 10 行，或 ≤ 20,000 字符，取小） | 拒绝，理由「建议过长」 |
| 4 | 不含 base 侧标记行 | 拒绝，理由「建议里混进了 base 段」 |
| 5 | 该建议**绑定**到它被展示的那个 region id | 拒绝（见 §3.8） |

`[INFERENCE]` 第 2 条是关键：模型很容易「帮忙」输出一整个带标记的文件；那不是解决冲突，是把问题原样搬回来。

#### 3.4.3 呈现

- `[PROPOSAL]` 建议**永远以 diff 呈现**：左 = ours 侧原文，右 = AI 建议（逐行 diff）。不是并排文本、不是「已解决」的静默替换。
- `[PROPOSAL]` **「接受」按钮在用户看过 diff 之前是 disabled 的。**（这是一个状态机约束，不是样式。）
- `[PROPOSAL]` 对话框始终给出三个**不依赖 AI** 的选项：保留 ours / 保留 theirs / 手工编辑。AI 建议是**第四个**选项，且**排在最后**。
- `[FACT]` 模型输出是不可信文本，进 DOM 走普通文本节点（Svelte 转义，`apps/gm-desktop/src/routes/Review.svelte:13-15` 明说该文件无 `{@html}`）。冲突对话框沿用同一约束。

#### 3.4.4 失败

| 失败 | 行为 |
|---|---|
| provider 报错 / 超时 | 该区域**保持未解决**；显示错误；用户可重试或手工解决。**绝不自动落库。** |
| 流没有终止事件 | 判为失败（`[FACT]` `src/ai/provider.rs:192-197`），不显示半截建议 |
| 校验不通过 | 建议被**标为「已拒绝」并附理由**；用户仍可手工编辑 |
| 用户中途关掉对话框 | 取消流（沿用 `ai_review_cancel` 的形状） |

`[PROPOSAL]` **没有任何一条失败路径会自动写文件。**

### 3.5 Q5：对话框放在哪？

**决策**：**新建 `/conflicts` 路由**（`apps/gm-desktop/src/routes/Conflicts.svelte`），对话框是路由内的一个组件，**不是** modal、**不是** `/review` 的扩展。

**论证**：

- `[FACT]` 本仓已有先例是**页面而非弹窗**，且理由就写在文件头上：`apps/gm-desktop/src/routes/Review.svelte:1-16`「Why a route and not the "push 前弹窗"… the V0 desktop shell has no push action to anchor a dialog to… So the dialog's content ships as a page of its own, and it is the same panel a dialog would host.」
- `[FACT]` `/review` 是「粘贴一段 diff 让 AI 点评」的工具，**没有仓库上下文**（`apps/gm-desktop/src/routes/Review.svelte:100-111` 只有一个 textarea + provider/model/baseUrl）。冲突仲裁需要：仓库、路径、区域集合、每区域三侧文本、每区域的落库状态。把它塞进 `/review` 会让 `/review` 同时承担两种不相干的心智模型。
- `[FACT]` 路由表是 `apps/gm-desktop/src/App.svelte:37-46` 的一个对象，hash router（`apps/gm-desktop/src/lib/router.ts`）。加一条 `/conflicts` 是**一行**的事。
- `[INFERENCE]` 但产品的动词是「弹窗」—— 折中：**路由是容器，对话框是容器里的面板**，用户从 `/repos/:name` 看到冲突列表点进去就落在 `/conflicts`，视觉上是一个居中面板。这与 `/review` 的「dialog 的内容以页面形式交付」是同一手法。

**复用清单（不新造轮子）**：

| 复用 | 位置 | 复用什么 |
|---|---|---|
| AI 事件桥 | `apps/gm-desktop/src/lib/api/ai.ts:19-23`、`:66-94` | 事件名常量 + `listenReviewEvents` 的「一次 attach 五个、失败全 detach」模式 |
| 流状态机 | `apps/gm-desktop/src/lib/stores/review.ts:47-79`、`:147-194` | `status` 状态机、sessionId 纪律、终止事件判失败、文本上限 |
| i18n | `apps/gm-desktop/src/lib/i18n/en.ts`、`zh-CN.ts` | 新增 `conflict.*` 键，两份目录**必须同步**（`apps/gm-desktop/tests/unit/i18n.test.ts` 覆盖 fallback 行为；本仓惯例是双语同增） |
| dev mock | `apps/gm-desktop/src/mocks/handlers.ts:488` | 新增 `conflict_regions` / `ai_conflict_start` / `ai_conflict_cancel` 的 mock 分支 |
| 错误边界 | `apps/gm-desktop/src/lib/components/ErrorBoundary.svelte` | 路由外层已有 ErrorBoundary（`apps/gm-desktop/src/App.svelte:135`） |
| 密钥入口 | `apps/gm-desktop/src/routes/Settings.svelte` | 新增一张「AI Provider Key」卡（§3.10） |

**什么会让我们改主意**：`[PROPOSAL]` 若后续产品形态变成「在 merge 提交前的一键预检」，则 `/conflicts` 仍是落点，只是在 `RepoDetail.svelte` 里加一个入口按钮。

### 3.6 Q6：结果怎么落地？

**决策**：**应用把「合并后的文件」写回工作区并 `git add` 该路径**，让 git 看到这个冲突已解决。**应用不驱动 merge 本身。**

**具体流程（`[PROPOSAL]`）**：

1. 读 index stage 1/2/3 → 跑 `merge-file` → 拿到带标记的合并文本 + 区域列表。
2. 每个区域由用户（或 AI 建议 + 用户确认）得到一个**替换文本**（ours / theirs / AI 建议 / 手工编辑）。
3. 把合并文本里每个冲突段替换成对应替换文本 → 得到**干净的合并文件**。
4. 写回工作区该路径 → `git add <path>`。
5. git 的 index 恢复成普通单 stage 条目；`git status` 不再报 `UU`（`[FACT]` §2.3 实测未合并路径在 porcelain v1 里就是 `UU`）。

**诚实说明（这是本节最重要的一句）**：

> `[INFERENCE]` **git 仍然是文件粒度的。** index 里依然只有一条路径条目；git 的合并算法没有变，git 也不知道「区域」是什么。gitgit 改的是**仲裁单位**：把「这个文件冲突了，你去解」换成「这个文件里有 N 个区域，第 i 个区域我们这么定了」。**不是**把 git 换成别的合并器。
>
> `[FACT]` 本仓目前**没有**任何写入 / 暂存路径：全仓 `git add` 只出现在测试夹具里（`src/repo/status.rs:772`、`:1057`、`:1085` 等），生产代码没有。所以「写回 + 暂存」是**新增**能力，且必须走既有白名单 argv 助手（`src/repo/status.rs:448`）的路子，**绝不能**拼 shell 字符串（`:441-447` 的理由）。
>
> `[FACT]` `git add <path>` 在「路径含空格 / 非 ASCII / 以 `-` 开头」时的行为**已实测**（Lane A 的验收测试 `a_path_with_spaces_and_non_ascii_can_be_staged`，git 2.41.0.windows.1）：`git add -- <path>` 三种路径都能正常暂存（`git status --porcelain=v1` 里三行都是 `M ` 开头，即已进 index）。**一个会咬人的细节**：git 默认把非 ASCII 路径打印成八进制转义（`"\303\274n\303\257code.txt"`），断言「路径是否出现在 status 里」必须加 `-c core.quotePath=false`，否则会误判成「没暂存」。本仓的 `validate_rel_path` 先例（`src/repo/status.rs:413-437`）照常复用。

**替代方案与取舍**：

| 方案 | 结论 |
|---|---|
| 应用写回文件 + `git add` | **采纳**。用户可用 `git checkout --ours/--theirs` 随时推翻。 |
| 应用直接写 index 的 stage 0 条目 | 否决。绕过 git 的 index 完整性检查，风险与收益不匹配。 |
| 用 `git merge-index` 驱动 | 否决。超出第一交付范围（§4.3），且需要 `GIT_INDEX_FILE` 等环境技巧。 |
| 只显示 diff，不落库 | 否决。那只是个查看器，不是「让流程走完」。 |

### 3.7 Q7：失败与边界

#### 3.7.1 没有 key

`[FACT]` 本仓已有 `AiReviewNoKey` 错误（`apps/gm-desktop/src-tauri/src/commands/ai.rs:225`，测试在 `:590-601`）。

`[PROPOSAL]` 冲突对话框在无 key 时**仍然完全可用**：保留 ours / 保留 theirs / 手工编辑三项照常。「问 AI」这一项**显示但 disabled**，并说明原因与「key 在哪里填」。**对话框不因为没有 AI 而降级。**

#### 3.7.2 AI 不得解决它没被展示的冲突

- `[PROPOSAL]` 请求**一次只带一个区域**。多区域绝不合并成一个请求（否则模型会跨区域「统一风格」，而那正是越权）。
- `[PROPOSAL]` 每个 region 带一个**不可伪造的 id**：`sha256(repo 相对路径 ‖ base blob oid ‖ 区域序号 ‖ 区域 ours/base/theirs 文本)` 的前 16 字节十六进制。
- `[PROPOSAL]` 建议回传时必须带同一个 region id；Rust 侧比对，不一致 ⇒ **丢弃**该建议（§3.4.2 第 5 条）。
- `[INFERENCE]` 这条堵住的是「模型串台」：流式通道复用、用户快速切换区域、上一个流的迟到 token —— 都不可能落到别的区域头上。

#### 3.7.3 可审计

`[PROPOSAL]` 每次仲裁产生一条审计记录，字段：

| 字段 | 说明 |
|---|---|
| `region_id` | 上面定义的 hash |
| `path` | repo 相对路径 |
| `provider` / `model` | **服务端报告的 model**，不是请求的 model（沿用 `/review` 的纪律：`apps/gm-desktop/src/lib/stores/review.ts:167-176`） |
| `redactions` | 发送前被替换的疑似机密数（`[FACT]` `src/ai/mod.rs:126-132`） |
| `prompt_sha256` | 送出的（已脱敏）prompt 的摘要 |
| `answer_sha256` / `answer_text` | 模型回答的摘要与（有界的）原文 |
| `choice` | `ours` / `theirs` / `ai` / `manual` |
| `applied_at` | 落地时间戳 |

`[PROPOSAL]` **API key 绝不进审计记录、绝不进日志、绝不进 flag**（§3.10）。`[FACT]` 既有防线：`sanitize::scrub_secret(&e.to_string(), &api_key)`（`apps/gm-desktop/src-tauri/src/commands/ai.rs:260`、`:356`）—— 新命令的错误路径必须同样过这道筛。

#### 3.7.4 边界清单（`[PROPOSAL]`）

| 情形 | 行为 |
|---|---|
| 二进制文件 | 无区域。标「需 git 原生处理」，对话框不假装能仲裁 |
| 一侧新增、一侧删除（add/delete 冲突） | 无三方 base。不进入区域仲裁 |
| 两侧内容相同 | 无区域 |
| 区域内 ours 与 theirs 文本相同 | 区域存在但两侧等价；默认建议「保留 ours」，AI 不必问 |
| 区域数 > 1 | 逐区域仲裁；区域列表常驻可见 |
| 用户手工编辑 | 编辑框预填 ours 侧文本，可编辑；保存前做与 AI 建议**同一套**校验（§3.4.2 的 2/3/4 条） |
| scope 切分不确定 | 回退「整文件一个 scope」⇒ 报冲突（§3.8） |
| 用户中途关页 | 流取消；已落地的区域保持已落地，未落地的保持未落地 |

### 3.8 Scope 层：规则、安全性质、以及**为什么第一交付只做定位不做抑制**

#### 3.8.1 规则（v1，`[PROPOSAL]`）

- 文件被切成**连续**的 scope，**覆盖文件的每一行**。scope 0 = 前言。
- item = 列 0 的顶层声明行（§2.2 的关键字表）；item 正上方连续的注释 / `#[` 行并入该 item。
- 不可切分的语言 ⇒ 整文件一个 scope。
- **切分必须覆盖整文件**。覆盖不了 ⇒ 整文件一个 scope。

> `[FACT]` 这条不是洁癖：§2.2 的测量脚本**第一版**就栽在这两处 —— 「item 前面的 `use ...;` 让 item 被跳过」和「前言行没被任何 scope 覆盖」。`[FACT]` 修法就是上面这句：**没有覆盖 ⇒ 整文件一个 scope**。

#### 3.8.2 安全性质（不可协商）

> **Scope 层只允许在「有把握」时缩小冲突集合。任何不确定，一律回退到「整文件一个 scope」= 报冲突。**
>
> `[INFERENCE]` **过度切分会把真冲突藏起来，这是危险方向。** 一个真实冲突若被切成两个不相交的 scope，scope 层就会说「这里没冲突」—— 用户看不到它，合并继续，最后带着一个错误的文件进主干。**宁可多报，不可漏报。**

因此必须有一条**硬性测试**：

> `[PROPOSAL]` **必测用例**：两个**相交**的 scope ⇒ 仍然报为冲突。
> `[PROPOSAL]` **必测用例**：scope 切分没能覆盖文件 ⇒ 报为冲突（不得因「切不出来」而沉默）。

#### 3.8.3 实测结果与**第一交付的范围收窄**（本 ADR 最诚实的一段）

`[FACT]` §2.2 实测：

- v1（列 0 item）：22 个冲突对里 ours/theirs 改动 scope **全部相交**（22/22）；**抑制次数 = 0**。
- v2（任意缩进 item，平均 scope 数 7.86 → 17.86）：**同样 22/22 相交，抑制次数 = 0**。
- 干净对里 v1/v2 也是 83/83 相交。

`[INFERENCE]` **结论：在本仓历史的这 105 个分叉对上，两版 scope 规则一次都没有缩小过冲突集合。**

`[PROPOSAL]` 因此，第一交付的 scope 层**只做两件事**：

1. **定位**：把每个冲突区域标到它所属的 scope（以及它是否跨越多个 scope），让用户在对话框里知道「这场冲突发生在哪个函数 / 哪个测试块」。
2. **守门**：当 scope 切分不确定时**拒绝缩小**，回退成「整文件一个 scope」，即报冲突。

**第一交付不做**：用 scope 层**抑制**冲突区域。理由不是设计不好，是**没有实测支撑**：在 105 个分叉对上抑制次数为 0，在这种样本上开一个「会隐藏冲突」的口子，是拿真实的合并事故换一个还没被证明的收益。

**抑制能力何时才解锁？** `[PROPOSAL]` 必须先有下面这组测量（§7.2 的门槛）：

- 在**更大**的分叉样本上（§7.2 给了取样方法）测出 v1/v2 的**真抑制次数 > 0**；
- 且在这些被抑制的样本里，**人工复核确认「那确实不是冲突」**的比例 ≥ 95%；
- 且**假阴性 = 0**（没有任何一个被抑制的区域其实是真冲突）。

达不到就不解锁。**这正是本仓反复在打的那场仗：一个没测过的能力不许被宣称。**

#### 3.8.4 scope 层与「区域层」的关系（澄清一个容易混的点）

`[INFERENCE]` scope 层**不是**「这个文件到底冲不冲突」的判定器。§2.2 实测已经说明：作为文件级过滤器，它比 git 更保守（83 个 git 判干净的对它全报相交）。它正确的作用位置是：

```
merge-file 产出冲突区域  →  scope 层给每个区域定位 scope  →  区域层逐区域仲裁
```

即 scope 层作用在**已经存在的冲突区域**上，回答「这场冲突发生在哪个 scope」，而不是**替代** git 判定「有没有冲突」。

### 3.9 API key：vault 与 `GITGIT_AI_API_KEY` 的交互与优先级

**现状（`[FACT]`）**：

- `[FACT]` `pub const API_KEY_ENV: &str = "GITGIT_AI_API_KEY";`（`src/ai/mod.rs:56`），模块注释写明理由：「Kept out of the CLI flag surface on purpose: an API key passed as an argument is visible in `ps` output and lands in shell history.」（`:52-55`）
- `[FACT]` 测试 `api_key_is_an_env_var_not_a_flag`（`src/ai/mod.rs:151-158`）断言的是**常量名的形状**（全大写 + 下划线），**不是**「key 只能来自环境变量」。⇒ **增加第二个来源不会破坏这条测试**，因为规则是「**永不作为 flag**」，vault 不是 flag。
- `[FACT]` 桌面端目前只读环境变量：`read_api_key_env()`（`apps/gm-desktop/src-tauri/src/commands/ai.rs:165-170`），命令入口传 `read_api_key_env()`（`:398`），缺 key 时报 `AiReviewNoKey`（`:225`）。
- `[FACT]` 桌面端已有版本化凭据库通道：`vault_set` → `VersionedVault::set_with_version`（`apps/gm-desktop/src-tauri/src/commands/vault.rs:86-94`；上游 `src/server/vault_versioned.rs:199`）。

**决策（`[PROPOSAL]`）**：

1. **key 在设置里输入 → 落到版本化 vault。** 键名空间：`ai/<provider>`（例：`ai/openai`）。
2. **优先级：vault 优先，环境变量兜底。**
   ```
   resolve_ai_key(provider):
       vault.get("ai/<provider>")  →  非空则用它
       否则 std::env::var("GITGIT_AI_API_KEY") → 非空则用它
       否则 None → AiReviewNoKey
   ```
   **为什么 vault 优先**：vault 是**显式、版本化、可轮换、跨会话**的用户资产（`[FACT]` ADR-0022 的版本化契约，`src/server/vault_versioned.rs:185-199`）；环境变量是**进程级的环境默认值**，通常来自 CI 或启动脚本。用户在应用里填了 key，就意味着「这是我为这个应用配的凭据」，环境变量只是「没配时的兜底」。反过来（env 优先）会让用户在 UI 里填了 key 却仍然发到别处 —— 那是更坏的意外。
   **注意 CLI 不受影响**：`gitai` 没有 UI，仍然只读 `GITGIT_AI_API_KEY`（`src/ai/mod.rs:97-135` 的 `Invocation::run` 签名就收 `api_key: &str`）。**本决策只改桌面端。**
3. **key 从不回流到 webview。**
   - `[PROPOSAL]` 新增 Tauri 命令 `ai_key_set(provider, key)`：写入 vault，**只返回版本号**，不返回 key。
   - `[PROPOSAL]` **不提供** `ai_key_get`：AI 调用时 Rust 侧直接从 vault 取，**在内存里**用完即弃。
   - `[FACT]` 现有 `vault_get`（`apps/gm-desktop/src-tauri/src/commands/vault.rs:79-84`）**会把值返回给前端**，所以 `ai/` 命名空间必须从通用 vault 列表里排除，或至少设置页永不对它调 `vault_get`。**[PROPOSAL] 落地要求**：`ai/*` 键在 vault UI 里**只列名不取值**。
4. **key 永不进 flag、永不进日志。**
   - `[FACT]` 既有防线：`scrub_secret`（`apps/gm-desktop/src-tauri/src/commands/ai.rs:260`、`:356`）把所有离开该模块的错误串过一遍。**新命令必须同样过这道筛**。
   - `[PROPOSAL]` **不得**在 `tracing` 字段、审计记录、错误消息里出现 key；新增一条门禁测试：对 `ai_key_set` / `ai_conflict_start` 的所有错误路径做断言（key 串不出现）。
5. **设置页的输入框**：`type="password"`、`autocomplete="off"`（沿用 `apps/gm-desktop/src/routes/Settings.svelte:108-111` 的 admin password 写法），保存后**清空输入框**（`:59` 已有同样做法）。

**这与仓库既有不变量的关系（必须写明，因为它是冲突点）**：

`[FACT]` `apps/gm-desktop/src-tauri/src/commands/ai.rs:11-18` 的不变量原话包括「a key in the webview is a key in a file the user can open」。
`[INFERENCE]` 这条顾虑**在 `/review` 里成立**（key 由环境提供，前端从不接触）。一旦允许用户在设置页输入 key，key **必然**短暂经过 webview —— 无法完全避免。
`[PROPOSAL]` 缓解：key 在 webview 里只存在于 `<input type=password>` 的绑定值，**不写入任何 store**（不进 `svelte/store`）、**不写 localStorage**、**保存后立即清空**、**任何时候都不被读回**（没有 `ai_key_get`）。
`[INFERENCE]` 这条缓解把「key 落在磁盘上被人打开」降级为「key 在内存里存在过一瞬间」。这是本 ADR 能做到的上限；**若用户不接受这个上限，唯一剩下的选择就是维持「key 只来自环境变量」**。§8 把这条列为**需要拍板的一条**。

### 3.10 实现拆分（见 §4）

---

## 4. 实施拆分 / Implementation split

三条 lane，各自可独立评审、独立合并、独立 revert。

### Lane A — `feat/conflict-regions`（Rust 核心：检测 + 区域 + scope）

**范围**：

- 新模块 `src/repo/conflict.rs`：
  - 读 index stage 1/2/3（`git ls-files -u` / `git show :N:path`）
  - 落三个临时文件 → 跑 `git merge-file --diff3 --marker-size=24 -L … -p`（走 `src/repo/status.rs:448` 的白名单 argv 助手模式）
  - 解析区域（按 24 字符标记宽度切）
  - scope 切分（v1 规则 + 覆盖检查 + 回退）
  - 类型：`ConflictSet { path, base_oid, regions: Vec<Region> }`、`Region { id, index, ours, base, theirs, base_range, scope: ScopeRef, confidence }`
- 写回路径：`write_resolved(path, content)` + `git add <path>`（同样走白名单 argv）
- **不改** `src/ai/`，**不改** 前端。

**验收测试**：

1. 真实临时仓库夹具（沿用 `src/repo/status.rs:614-622` 的 `unique_temp` 约定，**不引入 `tempfile` 依赖**）：
   a. 造出一个**双区域**冲突 ⇒ 断言**恰好 2 个区域**，且每个区域的 ours/base/theirs 文本**逐字**正确；
   b. 断言 `merge-file` 的**退出码**等于区域数（§2.1 实测的性质被固化成断言）；
   c. 断言 region id 稳定（同输入两次算出的 id 相同）。
2. **标记歧义用例**（§2.1 的实测复现）：base 正文里含一行 `<<<<<<< not a real marker` ⇒ 断言**仍然只有 1 个区域**，且 base 侧文本包含那行伪标记。
3. **scope 相交 ⇒ 仍报冲突**（§3.8.2 的硬性测试）。
4. **scope 覆盖不全 ⇒ 报冲突**（回退到整文件一个 scope）。
5. **二进制路径 ⇒ 不产生区域**，返回明确的「需 git 原生处理」。
6. `git add` 之后 `git status --porcelain=v1` 不再报 `UU`（§2.3 的性质）。

### Lane B — `feat/conflict-ai`（Rust AI 仲裁 + key 解析）

**范围**：

- `src/ai/prompt.rs` 新增 `conflict(region_ctx) -> Vec<ChatMessage>`（system trusted + user untrusted）。
- `apps/gm-desktop/src-tauri/src/commands/ai.rs` 新增 `ai_conflict_start` / `ai_conflict_cancel`：**复用** `/review` 的 `ReviewManager` / `TauriEmitter` / `pump` 形状与事件模型（`src/ai/provider.rs:175-206`、`apps/gm-desktop/src/lib/api/ai.ts:19-23`）。
- **建议校验**（§3.4.2 的 5 条）在 Rust 侧实现。
- **key 解析**（§3.9）：`vault["ai/<provider>"]` → `GITGIT_AI_API_KEY` → `None`。
- 新增 `ai_key_set` Tauri 命令（写 vault，**不返回值**）。
- `ai/*` 键从通用 vault 取值路径排除。

**验收测试**：

1. **prompt 形状**：区域文本**不得**出现在 system 段；必须出现在且仅出现在**一个** `Trust::Untrusted` 段（照抄 `src/ai/prompt.rs:104-125` 的既有断言形状）。
2. **脱敏**：区域里塞 `api_key = "super-secret-value-1234"`，断言送出的 request 里该值已被 `[REDACTED]` 替换（照抄 `apps/gm-desktop/src-tauri/src/commands/ai.rs:981-993` 的既有夹具形状）。
3. **校验拒绝**：假 provider 回一句含 `<<<<<<<` 的答案 ⇒ 建议被判为**已拒绝**并带理由，**且没有**任何写入发生。
4. **区域绑定**：回传的 region id 与请求不符 ⇒ 建议被丢弃。
5. **key 优先级**：vault 有值 ⇒ 用 vault 的；vault 空且 env 有 ⇒ 用 env 的；两者都空 ⇒ `AiReviewNoKey`。
6. **key 不外泄**：对 `ai_key_set` / `ai_conflict_start` 的所有错误路径断言 key 串**不出现**（`scrub_secret` 覆盖）。
7. **Anthropic 拒绝**：`supports_streaming == false` 的 provider ⇒ 请求在**发网之前**被拒（照抄 `src/ai/provider.rs:146-149` 的既有立场）。

### Lane C — `feat/conflict-ui`（Svelte：`/conflicts` 路由 + 对话框）

**范围**：

- `apps/gm-desktop/src/routes/Conflicts.svelte`（新路由）+ `apps/gm-desktop/src/lib/components/ConflictDialog.svelte`。
- `apps/gm-desktop/src/lib/stores/conflict.ts`（新 store：照 `src/lib/stores/review.ts` 的状态机纪律）。
- `apps/gm-desktop/src/lib/api/conflict.ts`（新桥接：照 `src/lib/api/ai.ts`）。
- `apps/gm-desktop/src/App.svelte` 加 `/conflicts` 路由 + 侧栏入口。
- i18n：`en.ts` + `zh-CN.ts` **同步**新增 `conflict.*` 键。
- `apps/gm-desktop/src/mocks/handlers.ts` 加 mock 分支（照 `:488` 的形状，且**不得**在 anthropic 上假装能流）。
- 设置页加「AI Provider Key」卡（写入 vault）。

**验收测试（vitest）**：

1. 无 key ⇒ 「问 AI」选项**存在但 disabled**，而 keep ours / keep theirs / 手工编辑**可用**。
2. AI 建议到达但用户**没看 diff** ⇒ 「接受」按钮 disabled；看过 diff ⇒ 变可用。
3. provider 失败 ⇒ 显示错误，区域**保持未解决**，**没有**写入发生。
4. 建议被校验拒绝 ⇒ 展示拒绝理由，**没有**写入发生。
5. 区域切换 ⇒ 旧流迟到的 token **不**串到新区域（sessionId 纪律）。

### Lane D — 文档 / 门禁（可与 C 并行）

- 本 ADR 的状态更新（实测落地后把 `[PROPOSAL]` 改成 `[FACT]`）。
- `[PROPOSAL]` 新增门禁：capability surface（`apps/gm-desktop/tests/unit/capability-surface-gate.test.ts`）必须包含新 Tauri 命令。

**依赖关系**：A →（B 与 C 并行）→ D 收口。**B 和 C 不互相阻塞**（C 可以在 A/B 之前用 mock 开工，这正是 `apps/gm-desktop/src/mocks/handlers.ts` 存在的意义）。

---

## 5. 后果 / Consequences

### 5.1 正面

- `[FACT]` 裁决次数从「文件数」降到「区域数」：本仓实测 35 区域 / 22 文件，**1.59×**；36% 的冲突文件带 ≥2 个独立区域。
- `[FACT]` 区域来源可**逐字**验证：`merge-file` 的退出码就是区域数（§2.1 实测），不需要自己数标记。
- `[FACT]` base 零成本：进行中 merge 的 index stage 1 **就是** merge base（§2.3 实测逐字节相同）。
- `[FACT]` 密钥纪律不破：规则是「永不作为 flag」，vault 不是 flag，既有测试 `api_key_is_an_env_var_not_a_flag`（`src/ai/mod.rs:151`）不受影响。
- `[FACT]` 零新依赖：git 能力走既有 subprocess 助手（`src/server/subprocess.rs:130`、`src/repo/status.rs:448`），AI 能力走既有 `src/ai/`。

### 5.2 负面 / 代价

- **Scope 层第一交付不抑制任何冲突**（§3.8.3）。它只定位 + 守门。**这是有意的**：实测抑制次数为 0（§2.2），在零样本上开「会隐藏冲突」的口子不值得。
- **多一个临时文件三件套**：`merge-file` 只吃文件（§2.1 实测），每次仲裁要落三个临时文件。
- **桌面端的 key 模型变了**：从「环境变量唯一」变成「vault 优先 + env 兜底」（§3.9）。这是对 `apps/gm-desktop/src-tauri/src/commands/ai.rs:11-18` 那条不变量的**收窄**，需要显式拍板（§8）。
- **没有 merge driver**：应用不启动 merge，也不写 index 的 stage 0。用户仍然要 `git commit` 完成合并（§3.6）。

### 5.3 推翻 / 备选（被否决的方案）

| 备选 | 为什么不选 |
|---|---|
| 自己实现三方合并 / diff3 区域切分 | 违反仓库立场「绝不重实现高风险 Git 设施」（`src/server/subprocess.rs:3-6`）；且会撞 0-new-deps 规则 |
| `git merge-tree --write-tree` 作区域来源 | `[FACT]` 2.41 可用，但它是仓库级、吃 branch/commit、吐 tree，不给单路径三 blob，也不给 base 侧正文 |
| `git checkout --conflict=diff3` | `[FACT]` 它**改写工作区文件**，而第一交付只读冲突、不驱动 merge |
| 逐行（line）仲裁 | `[INFERENCE]` 冲突是语义块，逐行裁决会产出无意义的结果（§3.0） |
| 用 scope 层直接判「文件冲不冲突」 | `[FACT]` 实测它比 git 更保守（83/83 干净对仍报相交），会误报 |
| AI 直接写文件 / 自动应用建议 | 违反「AI 是建议、用户确认」的决定 |
| key 进 flag / 进日志 / 存 localStorage | 违反仓库既有规则（`src/ai/mod.rs:52-56`、`src/ai/sanitize.rs:18-23`） |

---

## 6. 验证 / Verification（本 lane 实际跑过的）

| # | 验证 | 结果 |
|---|---|---|
| 1 | `git --version` | `2.41.0.windows.1`；与 `docs/adr/0020-v0-gui-tauri-svelte.md:88` 记录的下限一致 ✅ |
| 2 | `git merge-file -h` / `git merge-tree -h`（2.41 usage 行） | `merge-file` 有 `--diff3` / `--marker-size` / `-L` / `-p`，**无** `--object-id`；`merge-tree` **有** `--write-tree` ✅ |
| 3 | `merge-file` 退出码 = 区域数 | 1 区 → 1；2 区 → 2 ✅ |
| 4 | `-L` 标签 | ours/base/theirs 三侧标签正确 ✅ |
| 5 | 标记歧义 + `--marker-size=24` | 伪标记行原样出现在真标记前；24 字符标记可区分 ✅ |
| 6 | 本仓历史 scope 调查 | 105 个分叉对；22 冲突（35 区域）/ 83 干净；v1 与 v2 均 **0 抑制** ✅ |
| 7 | 真实冲突 merge 的 index stage | `UU` + 三 stage；**stage 1 == merge base 内容**（逐字节）✅ |
| 8 | `merge-file` 喂 stage blob | 产出 diff3 区域，退出码 1 ✅ |
| 9 | 全仓冲突代码普查 | 生产代码**零**冲突处理实现 ✅ |
| 10 | 0-new-deps | 本 lane 未引入任何依赖；设计所用能力均为既有模块 ✅ |

**本 lane 未跑**：任何编译 / 测试 / 构建（设计 lane，无代码）。

---

## 7. 后续 / Follow-ups（本 ADR 不做，但给了测量方法）

### 7.1 区域级 scope 测量

§2.2 的测量是**对级**（整文件改动集合）。区域级的细化测量（每个区域单独判 scope 相交/不相交）是 Lane A 的产物之一，方法：解析 `merge-file` 输出时同步维护 base 行游标，把每个区域映射到 base 行区间，再映射到 scope。

### 7.2 更大的分叉样本（scope 抑制能力的解锁门槛）

- `[PROPOSAL]` 取样方法：在**更大的**历史样本上重复 §2.2 的流程（同一脚本、同一规则），目标样本量 ≥ 1,000 个分叉对。当前仓只有 136 个分叉对 —— **样本太小，不足以证伪**。
- `[PROPOSAL]` 解锁门槛（三条同时满足才允许 scope 层抑制）：
  1. 真抑制次数 **> 0**；
  2. 被抑制样本经人工复核，「确实不是冲突」的比例 **≥ 95%**；
  3. **假阴性 = 0**。

### 7.3 AI 建议的可靠性

- `[INFERENCE]` **本 ADR 对 AI 建议的可靠性不作任何未测量的断言。**
- `[PROPOSAL]` 要让「问 AI」从「一个选项」升级成「默认建议」，必须先有实测：
  - 在真实冲突样本上，模型给出的替换文本被**人工判定为正确**的比例；
  - 模型**引入新错误**（相对 keep-ours / keep-theirs 的基线）的比例；
  - 跨 provider 的差异（`[FACT]` 五个预置里四个能流、anthropic 不能，`src/ai/registry.rs:98-103`）。
- 在这组数字出来之前，UI 里 AI 建议**永远排在 keep-ours / keep-theirs 之后**，且**必须以 diff 呈现**。

### 7.4 其它已知空白

- `[UNVERIFIED-FACT]` rename/rename 冲突、递归合并策略之外的 stage 1 语义（§3.2）—— Lane A 必须补测。
- `[UNVERIFIED-FACT]` `git add <path>` 在含空格 / 非 ASCII / 以 `-` 开头的路径上的行为 —— Lane A 必须补测并复用 `validate_rel_path`（`src/repo/status.rs:413`）。
- `[PROPOSAL]` 二进制文件的冲突：gitgit 第一交付不仲裁（§3.7.4）。

---

## 8. 需要拍板的一条（其余决策已在本 ADR 内定稿）

`[INFERENCE]` §3.9 的第 3 条让 key **短暂经过 webview**（用户在设置页输入）。仓库既有不变量明写「a key in the webview is a key in a file the user can open」（`apps/gm-desktop/src-tauri/src/commands/ai.rs:15-16`）。

**建议（推荐项）**：接受 §3.9 的缓解（不进 store、不落盘、不可读回、保存即清空），并在设置页明写「key 会经过界面内存」。**备选**：维持「key 只来自环境变量」，桌面端不允许输入 —— 那样产品的「填好 api key」这半句就落不了地。

---

## 9. 引用 / References

**本仓代码**

- `src/ai/mod.rs:52-56`（`API_KEY_ENV` 与其理由）、`:97-135`（`Invocation::run` 的先拼 prompt 再 sanitize）、`:151-158`（`api_key_is_an_env_var_not_a_flag`）
- `src/ai/provider.rs:28-35`（`Trust`）、`:81-84`（默认 `Untrusted`）、`:99-105`（`ChatMessage::untrusted`）、`:142-154`（`ProviderCapabilities`）、`:175-209`（流式契约与拒绝立场）
- `src/ai/registry.rs:98-103`（Anthropic 不支持流式）、`:132-175`（五个预置）
- `src/ai/sanitize.rs:18-23`（脱敏是启发式）、`:36-42`（`UNTRUSTED_SYSTEM_PROMPT`）、`:299-308`（`wrap_untrusted`）、`:321`（`sanitize_request`）
- `src/ai/prompt.rs:5-8`（system/user 分离的理由）、`:78`（`MAX_DIFF_CHARS`）、`:86-96`（夹紧与可见告知）、`:104-125`（既有断言形状）
- `src/ai/stream.rs:128-130`（`StreamEvent::Token`）
- `src/ai/diff.rs:38-70`（白名单 argv 的 `git_output`）
- `src/server/subprocess.rs:3-6`（不重实现 Git 设施）、`:130-150`（`run_capture`）
- `src/repo/status.rs:124-128`（`DiffTarget`）、`:413-437`（`validate_rel_path`）、`:441-447` + `:448`（`git_output` 白名单）、`:504`（`worktree_status`）、`:535-578`（`worktree_diff`）、`:614-622`（`unique_temp` 约定）
- `src/server/vault.rs:187-222`（`Vault` trait）
- `src/server/vault_versioned.rs:185-199`（`VersionedVault` + `set_with_version`）
- `apps/gm-desktop/src-tauri/src/commands/ai.rs:11-18`（key 只来自环境的不变量）、`:165-170`（`read_api_key_env`）、`:225`（`AiReviewNoKey`）、`:260`、`:356`（`scrub_secret`）、`:398`、`:590-601`（既有测试形状）
- `apps/gm-desktop/src-tauri/src/commands/vault.rs:79-84`（`vault_get` 回流前端）、`:86-94`（`vault_set` → `set_with_version`）
- `apps/gm-desktop/src/lib/api/ai.ts:19-23`、`:66-94`（事件桥与 detach 纪律）
- `apps/gm-desktop/src/lib/stores/review.ts:8-20`、`:36`、`:147-194`（状态机纪律）
- `apps/gm-desktop/src/routes/Review.svelte:1-16`（页面而非弹窗的先例与理由）、`:13-15`（无 `{@html}`）
- `apps/gm-desktop/src/App.svelte:37-46`（路由表）、`:135`（ErrorBoundary）
- `apps/gm-desktop/src/lib/i18n/en.ts:203-227`（`review.*` 键族）、`:224`（`review.noKey`：「the key is never entered here」）
- `apps/gm-desktop/src/mocks/handlers.ts:488`（`ai_review_start` mock 形状）
- `apps/gm-desktop/src/routes/Settings.svelte:59`、`:107-112`（admin password 的清空 / password 输入框写法）
- `apps/gm-desktop/tests/unit/i18n.test.ts`（i18n 行为测试）
- `docs/adr/0020-v0-gui-tauri-svelte.md:88`（Git 2.41.0 环境核实）
- `docs/plan/v0-tasks.md:358`（同源环境记录）
- `docs/adr/0022-v0-credential-vault-versioned.md`（版本化 vault 契约）

**外部**

- git 2.41 的 `git merge-file` / `git merge-tree` usage 行（本机实测，见 §2.1）
