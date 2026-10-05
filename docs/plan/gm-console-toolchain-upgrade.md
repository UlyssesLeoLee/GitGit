# gm-console 工具链升级 runbook

> `[FACT]` 本文记录的 before 状态全部由实测得到,版本选型依据全部来自实际查询。
> **本轮未能执行**,原因是环境阻断,见文末。执行时请重新核对数字,不要盲信本文。

## 为什么要做

`apps/gm-console` 停留在 2026 年的旧工具链上,`pnpm audit` 报 **1 critical / 2 high / 7 moderate**。
这些全部位于 devDependency 链,**没有运行时暴露面** —— 攻击者需要一个能触达 dev server 的环境。
所以这是合规与卫生问题,不是「用户可被利用」的问题。优先级低于运行时漏洞,但仍然不该留着。

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
| high | braces | 3.0.3 | **无修复版** | GHSA-vfj7-8cjw-p6xm |
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
+    "@vitest/coverage-v8": "^5.0.3",
-    "vite": "^5.4.8",
-    "vitest": "^2.1.2"
+    "vite": "^7.3.6",
+    "vitest": "^5.0.3"
```

### 版本选型依据

- `[FACT]` **`@vitejs/plugin-react` 必须是 5.x,不能是 6.x。**
  `@vitejs/plugin-react@6.1.1` 的 peer 依赖要求 Vite 8,而 Vite 8 换用 rolldown,
  会强制 `build.minify` 迁移。`@vitejs/plugin-react@5.2.0` 的 peer 范围是
  `^4 || ^5 || ^6 || ^7 || ^8`,同时覆盖 Vite 7。原来的 4.7.0 不覆盖 Vite 7。
- `[FACT]` **停在 Vite 7,不要上 Vite 8。** `apps/gm-desktop` 已经因为同样的理由
  停在 7.3.6,两个 app 保持一致比各自最优更重要。
- `[FACT]` `react-router-dom@7.18.4` 是执行时 `pnpm view react-router-dom version` 返回的当前版本。
  `[FACT]` 该 app 的路由 API 面很小(`BrowserRouter` / `Routes` / `Route` / `Link` /
  `useParams` / `useNavigate` / `useSearchParams`),这些在 v7 里都还存在,
  所以这是一次低风险的 major 升级。`tests/unit/app-routing.test.ts` 是它的回归网。
- `[FACT]` esbuild 不需要显式升级。它是 vite 的传递依赖,vite 7 会带上 0.25+。

### `braces` 怎么办

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
