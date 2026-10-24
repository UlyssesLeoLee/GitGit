# gm-console 工具链升级 runbook

> `[FACT]` 本文记录的 before 状态全部由实测得到,版本选型依据全部来自实际查询。
> **执行状态:2026-10-24 更新。** 六步工具链升级**尚未执行** —— 但
> `react-router` 6 → 7 **已经执行过**,其余四步仍未执行;「本轮未能执行」对
> **整份 runbook** 的描述已不再成立。逐项实测见「执行状态(2026-10-24 实测)」一节。
> 本文「Before(实测,2026-10-05)」小节是**带日期的快照**,执行时请重新核对数字,
> 不要盲信本文。

## 执行状态(2026-10-24 实测)

`[FACT]` 逐项核对当前树(`origin/dev` @ `255ab92`,`apps/gm-console`):

| 步骤 | 目标 | 2026-10-24 实测 | 状态 |
| --- | --- | --- | --- |
| `react-router` 6 → 7 | `^7.18.4` | `react-router@7.18.4` / `react-router-dom@7.18.4` | **已执行** |
| `vite` 5 → 7 | `^7.3.6` | `vite@5.4.21` | 未执行 |
| `vitest` 2 → 5 | `^5.0.3` | `vitest@2.1.9` | 未执行 |
| `@vitest/coverage-v8` 2 → 5 | `^5.0.3` | `@vitest/coverage-v8@2.1.9` | 未执行 |
| `@vitejs/plugin-react` 4 → 5 | `^5.2.0` | `@vitejs/plugin-react@4.7.0` | 未执行 |
| `braces` → tailwind 4 | tailwind 4 | `tailwindcss@3.4.19`,`braces@3.0.3` | 未执行,**且已不再由告警驱动**(见下) |

`[FACT]` `react-router` 6 → 7 这一步在 2026-10-05 之后执行过:实测
`apps/gm-console/package.json` 声明 `"react-router-dom": "^7.18.4"`,
`apps/gm-console/pnpm-lock.yaml` 钉住 `react-router@7.18.4` 与
`react-router-dom@7.18.4`。因此本文「Before」表里 `react-router 6.30.6` 那一行
只对 2026-10-05 成立,现在不再成立。

### `braces` 那一步的理由变了(告警已被撤回)

`[FACT]` 本文与 `docs/reports/2026-10-05-frontend-deps/README.md` 引用的
`braces` 告警是 `GHSA-vfj7-8cjw-p6xm`。它**已于 2026-10-23 从 npm 告警库撤回**
(发布于 2026-09-18,标题原文即 "Withdrawn Advisory: braces vulnerable to
stack-exhaustion denial of service through deeply nested patterns")。

`[FACT]` 仍存在的另一条 `braces` 告警 `GHSA-grv7-fg5c-xmjg`(high,
`vulnerable_versions: < 3.0.3`)**不适用**于本仓 —— 本仓钉的是 `braces@3.0.3`。
实测:向 npm bulk 端点
`POST https://registry.npmjs.org/-/npm/v1/security/advisories/bulk` 提交
`{"braces":["3.0.3"]}` 返回 `{}`,今天的 `pnpm audit` 也不再报 `braces`。

`[INFERENCE]` 因此「`braces` 没有修复版,所以唯一出路是上 tailwind 4」这条
**理由**现在不成立了 —— 没有任何告警在逼这次迁移。

> `[FACT]` **但下面「`braces` 怎么办」一节里的迁移评估本身仍然成立,与告警无关**,
> 所以保留原文不改。tailwind 3 → 4 仍可因为别的原因值得做(例如 CSS-first 配置、
> 体积、性能);只是它不再是任何告警的**必要条件**了。

### 本轮(2026-10-24)实际执行了什么、没执行什么

`[FACT]` **没有执行**本文「执行步骤」里的工具链升级:`vitest` 2 → 5、`vite` 5 → 7、
`@vitest/coverage-v8` 2 → 5、`@vitejs/plugin-react` 4 → 5 四步都**仍未执行**。
它们各自需要一次 major 版本迁移,应按本文的六道门禁单独执行、单独验证。

`[FACT]` **执行了**本文「执行步骤」第 3 步的六道门禁,用来验证本 lane 自己对
`apps/gm-console` 的改动(删除未使用的表单依赖栈 + 一条 `pnpm.overrides` 钉住
已修复的 `postcss-selector-parser`)。2026-10-24 实测,全部 `TZ=UTC`:

| 门禁 | 命令 | 退出码 |
| --- | --- | --- |
| typecheck | `corepack pnpm@9.15.9 typecheck` | 0 |
| lint | `corepack pnpm@9.15.9 lint` | 0 |
| test | `corepack pnpm@9.15.9 test` | 0(17 files / 221 tests) |
| test:coverage | `corepack pnpm@9.15.9 test:coverage` | 0(94.11% lines / 87.69% branches / 82.39% functions) |
| format:check | `corepack pnpm@9.15.9 format:check` | 0 |
| build | `$env:VITE_ENABLE_MOCKS='false'; corepack pnpm@9.15.9 build` | 0 |

`[FACT]` **网络阻断已不再存在**:2026-10-24 本机到 `registry.npmjs.org` 的
`pnpm install` / `pnpm audit` / `pnpm view` 全部正常。原文末尾「本轮为何未执行」
记录的是 **2026-10-05** 那一轮的状态,原文保留,不再适用于现在。

`[FACT]` 一个仍然存在的环境陷阱:`tests/unit/format.test.ts` 里
`formatUnixMs(0)` 断言结果含 `1970`。本机时区是 `America/Tijuana`(UTC-8),
epoch 被格式化成 `12/31/1969`,断言不成立 —— **这是时区依赖,不是代码缺陷**,
CI runner 是 UTC 所以绿。本轮所有本机验证均在 `TZ=UTC` 下运行。

## 为什么要做

`apps/gm-console` 停留在 2026 年的旧工具链上。`[FACT]` 2026-10-24 实测
`pnpm audit` 报 **15 条:3 critical / 2 high / 9 moderate / 1 low**
(2026-10-05 当时是 10 条:1 critical / 2 high / 7 moderate)。
`[FACT]` **其中没有运行时暴露面** —— 15 条全部位于 devDependency 链:按 audit
输出的依赖 `paths`,第一条命中的直接依赖都是 `vite` / `vitest` /
`@vitest/coverage-v8`。`[FACT]` 需要注意的是 `package.json` 曾经把 `zod`
声明成**生产依赖**,而它带着一条 HIGH 告警(GHSA-mrhx-c7xc-m3w5)—— 但源码级
import 扫描显示它在 `src/` 与 `tests/` 里**一次都没被导入**,Vite 因此从未把它
打进产物;2026-10-24 它连同 `react-hook-form` / `@hookform/resolvers`
(同一条死掉的表单校验栈)一起被删除,修法是**删掉一个没人用的依赖**,不是迁移。
所以这是合规与卫生问题,不是「用户可被利用」的问题。优先级低于运行时漏洞,
但仍然不该留着。

同仓的 `apps/gm-desktop` 已经完成同一次升级并且 CI 全绿,它是本次的目标状态参考。

## Before(实测,2026-10-05)

| 项 | 值 | 怎么测的 |
| --- | --- | --- |
| 测试 | 194 passed / 15 files / 0 skipped | `corepack pnpm@9.15.9 test` |
| 覆盖率 | lines 93.63% / branches 87.1% / functions 81.34% | `corepack pnpm@9.15.9 test:coverage` |
| 门槛 | lines & statements 70, functions 70, branches 60 | `vite.config.ts`,CI **确实**在跑 `test:coverage` |
| 依赖 | vitest 2.1.9 / coverage-v8 2.1.2 / vite 5.4.8 / plugin-react 4.7.0 / react-router 6.30.6 | `pnpm list --depth 0` |
| 告警 | 1 critical / 2 high / 7 moderate,共 575 个依赖 | `pnpm audit --json` |

### 逐条告警与目标版本

| 严重度 | 包 | 当前 | 修复版本 | advisory |
| --- | --- | --- | --- | --- |
| critical | vitest | 2.1.9 | >=3.2.6 | GHSA-5xrq-8626-4rwp |
| high | vite | 5.4.21 | >=6.4.3 | GHSA-fx2h-pf6j-xcff |
| high | braces | 3.0.3 | **已撤回(2026-10-23)** | GHSA-vfj7-8cjw-p6xm |
| moderate | @vitest/mocker | 2.1.9 | >=4.1.11 | GHSA-82fw-gwwq-j7x9 |
| moderate | vitest | 2.1.9 | >=4.1.11 | GHSA-82fw-gwwq-j7x9 |
| moderate | esbuild | 0.21.5 | >=0.25.0 | GHSA-67mh-4wv8-2f99 |
| moderate | react-router | 6.30.6 | >=7.18.0 | GHSA-wrjc-x8rr-h8h6 |
| moderate | react-router | 6.30.6 | >=7.18.0 | GHSA-337j-9hxr-rhxg |
| moderate | vite | 5.4.21 | >=6.4.2 | GHSA-4w7w-66w2-5vf9 |
| moderate | vite | 5.4.21 | >=6.4.3 | GHSA-v6wh-96g9-6wx3 |

注意 vitest 有两条不同 advisory:critical 那条只要 >=3.2.6,mocker 那条要 >=4.1.11,
`>=3.2.6` 单独满足不了第二条。取 5.x 一并解决。

## 目标状态

照抄 `apps/gm-desktop/package.json` 里已验证可用的版本:

```diff
-    "react-router-dom": "^6.27.0",
+    "react-router-dom": "^7.18.4",
-    "@vitejs/plugin-react": "^4.3.2",
-    "@vitest/coverage-v8": "^2.1.2",
+    "@vitejs/plugin-react": "^5.2.0",
+    "@vitest/coverage-v8": "^5.0.5",
-    "vite": "^5.4.8",
-    "vitest": "^2.1.2"
+    "vite": "^7.3.7",
+    "vitest": "^5.0.5"
```

> `[FACT]` **2026-10-24 更新:上面的 `vite` / `vitest` 目标版本已随
> `apps/gm-desktop` 前移。** 原文写的 `^7.3.6` / `^5.0.3` 是 2026-10-05 当时
> `gm-desktop` 的地板;2026-10-24 修复 `vite@7.3.6` 的 HIGH 告警
> (GHSA-587h-7wmj-h5pm 等)需要 `>=7.3.7`,`vitest` / `@vitest/coverage-v8`
> 也升到了 5.0.5,所以「照抄 gm-desktop」现在得到的是 `^7.3.7` / `^5.0.5`。
> `[FACT]` 执行时以当日 `gm-desktop` 的实际地板为准,不要照抄本文的数字。

### 版本选型依据

- `[FACT]` **`@vitejs/plugin-react` 必须是 5.x,不能是 6.x。**
  `@vitejs/plugin-react@6.1.1` 的 peer 依赖要求 Vite 8,而 Vite 8 换用 rolldown,
  会强制 `build.minify` 迁移。`@vitejs/plugin-react@5.2.0` 的 peer 范围是
  `^4 || ^5 || ^6 || ^7 || ^8`,同时覆盖 Vite 7。原来的 4.7.0 不覆盖 Vite 7。
- `[FACT]` **停在 Vite 7,不要上 Vite 8。** `apps/gm-desktop` 已经因为同样的理由
  停在 7.3.7(2026-10-24 更新),两个 app 保持一致比各自最优更重要。
- `[FACT]` `react-router-dom@7.18.4` 是执行时 `pnpm view react-router-dom version` 返回的当前版本。
  `[FACT]` 该 app 的路由 API 面很小(`BrowserRouter` / `Routes` / `Route` / `Link` /
  `useParams` / `useNavigate` / `useSearchParams`),这些在 v7 里都还存在,
  所以这是一次低风险的 major 升级。`tests/unit/app-routing.test.ts` 是它的回归网。
- `[FACT]` esbuild 不需要显式升级。它是 vite 的传递依赖,vite 7 会带上 0.25+。

### `braces` 怎么办

> `[FACT]` **2026-10-24 更正:这一节的前提已经变了,但结论里的迁移评估仍然有效。**
> 原文写的「`braces` 的 `patched_versions` 字面写的是 `<0.0.0` —— **没有任何已发布
> 版本修好了它**」是**已撤回**的 `GHSA-vfj7-8cjw-p6xm` 自己的文本。那条告警已于
> **2026-10-23** 从 npm 告警库撤回。仍存在的 `GHSA-grv7-fg5c-xmjg` 的区间是
> `< 3.0.3`,而本仓钉的是 `braces@3.0.3`,**不适用**。
> `[FACT]` 实测:向 npm bulk 端点提交 `{"braces":["3.0.3"]}` 返回 `{}`;
> 2026-10-24 的 `pnpm audit` 也不再报 `braces`。
> `[INFERENCE]` 所以「上 tailwind 4」不再被任何告警**强制**。但下面这段迁移风险
> 评估与告警无关,仍然成立,所以保留。

**原文(迁移评估,与告警无关,保留):**

`braces` 的 `patched_versions` 字面写的是 `<0.0.0` —— **没有任何已发布版本修好了它**。
唯一出路是迁移到 Tailwind CSS v4,因为 v3 的 `chokidar` / `fast-glob` / `micromatch`
链才引入它。

`[FACT]` 迁移风险评估(已实际扫描过源码):
- `src/` 下只有**一个** CSS 文件
- 组件里**没有**用到 v4 会破坏的 utility
- `tailwind.config.js` 和 `postcss.config.js` 需要按 v4 的 `@import "tailwindcss"` +
  CSS-first 配置改写

所以这是一次**独立、低风险**的迁移,应该和上面的工具链升级**分开做**,
不要捆在一起 —— 否则一旦 v4 样式出问题,无法判断是 vitest 5 带来的还是 tailwind 4 带来的。

## 执行步骤

前置:`registry.npmjs.org` 必须可达。Windows 上先验证,不要装到一半才发现:

```powershell
try { (Invoke-WebRequest -Uri "https://registry.npmjs.org/vite" -Method Head -UseBasicParsing -TimeoutSec 20).StatusCode }
catch { "BLOCKED: $($_.Exception.Message)" }
```

1. `cd apps/gm-console`,用 `corepack pnpm@9.15.9 install` 重新生成 lockfile
   —— **lockfile 绝不能手改**
2. 核对实际装上的版本:`corepack pnpm@9.15.9 list vitest vite --depth 0`
   (`pnpm install` 打印 "Already up to date" 不代表依赖可用,见下)
3. 依次跑六道门禁,记下每道的退出码:

   ```powershell
   corepack pnpm@9.15.9 typecheck
   corepack pnpm@9.15.9 lint
   corepack pnpm@9.15.9 test
   corepack pnpm@9.15.9 test:coverage
   corepack pnpm@9.15.9 format:check
   $env:VITE_ENABLE_MOCKS='false'; corepack pnpm@9.15.9 build
   ```

4. `corepack pnpm@9.15.9 audit --json`,记录前后告警数
5. 提交时 **`package.json` 和 `pnpm-lock.yaml` 必须同一个 commit**

## 三个坑

- **`package.json` 和 `pnpm-lock.yaml` 不能分开提交。** `.github/workflows/gm-console.yml`
  跑的是 `pnpm install --frozen-lockfile`,两者不一致会直接把 CI 打红,而且是在
  装依赖那一步红,后面的门禁全被 skip。
- **不要相信 "Already up to date"。** 它只比对 lockfile 与 manifest,不看
  `node_modules` 是否真的可解析。`apps/gm-desktop` 主工作树就长期停在
  vitest 2.1.9(而 manifest 写 5.0.3),导致一次本地「289 passed」其实跑的是旧版。
  用 `pnpm list <pkg> --depth 0` 核对实际版本。
- **覆盖率门槛不能为了这次升级而下调。** 升级前是 93.63% lines,离 70 的门槛很远,
  有充足余量;真掉了说明有测试没跑起来,那是 bug 不是余量问题。

## 本轮为何未执行

> `[FACT]` **本节记录的是 2026-10-05 那一轮的状态,原文保留。** 2026-10-24 实测:
> 本机到 `registry.npmjs.org` 的网络**不再被阻断**,`pnpm install` / `pnpm audit` /
> `pnpm view` 全部正常,所以「环境阻断」不再是执行本文升级步骤的理由。当前状态
> 见「执行状态(2026-10-24 实测)」一节。

`[FACT]` 2026-10-05 执行时,本机到 `registry.npmjs.org` 的 TLS 连接被中间设备拦截:

```
HEAD https://registry.npmjs.org/vite           -> The SSL connection could not be established
HEAD https://registry.npmjs.org/react-router-dom -> The SSL connection could not be established
HEAD https://registry.npmjs.org/vitest          -> The SSL connection could not be established
```

`pnpm` 偶尔能挤过去(同一时间窗内 `pnpm audit` 和一次 `pnpm view` 都成功过,
`pnpm view react-router-dom version` 返回了 `7.18.4`),但 `pnpm install` 需要拉
400 多个包的 tarball,在这个条件下十分钟零进展,store 目录数与 lockfile 都没有变化。

因此本轮**没有提交任何 `apps/gm-console` 的改动**。本文记录的 before 数据与版本
选型依据仍然有效,网络恢复后可直接照此执行。

## `--offline` 能做什么、不能做什么

`[FACT]` 上面「网络被拦」这个结论下得太宽,需要分两件事:

**能:`--offline` 从本地 store 装出当前 lockfile 需要的依赖。**
2026-10-05 实测,在一个全新 worktree 里:

```powershell
cd apps/gm-desktop          # 没有任何 node_modules 的新 worktree
corepack pnpm@9.15.9 install --frozen-lockfile --offline   # Done in 11.5s
```

11.5 秒装完,vitest 5.0.3 就位,四道门禁随后全绿。也就是说**开 lane worktree
不再需要先解决网络**,这一点此前被连带低估了。

**不能:升级本身。** 上面的升级要引入**新版本**,而新版本的 tarball 不在本地
store 里,`--offline` 会直接失败而不是退化成慢速下载。所以本文「执行步骤」第 1 步
的 `pnpm install`(不带 `--offline`)依然需要 registry 可达 —— 那个前置条件没有被
绕过,只是不再阻塞「开 worktree」这件事。

**结论:别把 `--offline` 当成这次升级的解法。** 它解决的是 lane 准备工作,不是
升级本身。升级仍然等网络。

