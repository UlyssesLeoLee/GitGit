# Tauri 桌面端打包验证报告

日期：2026-10-03
范围：`apps/gm-desktop`（Tauri 2 + Svelte 5）
分支：`lane/h-tauri-build`

## 结论

`[FACT]` 此前本仓**没有任何证据表明桌面端可以打包**——`cargo tauri build` 从未被成功执行过，`apps/gm-desktop/package.json` 里的 `tauri` 脚本调用的是一个**未安装**的 CLI。

`[FACT]` 修复后打包成功，产出两个真实安装包。

## 阻塞点（按发现顺序）

### 1. `@tauri-apps/cli` 未声明为依赖 —— 真实阻塞

`[FACT]` `package.json` 的 `devDependencies` 里没有 `@tauri-apps/cli`，`pnpm-lock.yaml` 里也没有。直接执行打包脚本的实际输出：

```
'tauri' 不是内部或外部命令
exit 1
```

`[FACT]` 这意味着 `pnpm tauri build` / `pnpm tauri dev` 在任何干净检出的机器上都**不可能成功**。此前的 CI（`.github/workflows/gm-desktop.yml`）只跑 `cargo check`，从未触及 CLI，因此从未暴露该问题。

修复：`devDependencies` 增加 `"@tauri-apps/cli": "2"`。实测安装到 2.12.1。

### 2. WiX 工具链需要联网下载

`[FACT]` 首次尝试时下载 `wix314-binaries.zip` 遭遇 `net::ERR_CONNECTION_CLOSED` 而中断。重试后成功。

`[TBD]` 该下载步骤依赖网络可达性。本机代理端口在本轮期间发生过变化（7897 → 10808），该中断与代码无关，但**在网络受限环境中打包会失败**，这一点未做进一步固化。

### 3. 图标为 1×1 占位文件 —— 非阻塞，但是产品缺陷

`[FACT]` `src-tauri/icons/` 下 6 个文件全部是占位图：

| 文件 | 字节数 | 实际内容 |
|---|---|---|
| `32x32.png` / `128x128.png` / `icon.png` / `tray.png` | 69 | 1×1 PNG |
| `icon.ico` / `icon.icns` | 76 | 仅含头部的占位容器 |

`[FACT]` **打包不因此失败** —— WiX 接受这些文件并产出了可安装的 `.msi`。但安装后程序在任务栏、开始菜单和「应用和功能」里都会显示为空白默认图标。

`[TBD]` 正式发布前需要真实图标资源。这不是工程阻塞，是素材缺口，本报告不做臆造。

### 4. `bundle.targets` 跨平台混列

`[FACT]` `tauri.conf.json` 的 `targets` 为 `["msi", "dmg", "appimage", "deb"]`。Tauri 会按宿主平台自动过滤，因此 Windows 上只产出 `msi`，未报错。**未改动** —— 保留跨平台声明是对的，改成平台专属会让另一端的发布流程失效。

## 实测证据

命令（在 `apps/gm-desktop` 下）：

```
$env:CARGO_TARGET_DIR = "E:\DevCache\cargo\target-lane-h"
.\node_modules\.bin\tauri.cmd build --bundles msi
```

关键输出：

```
Finished `release` profile [optimized] target(s) in 4m 35s
   Built application at: E:\DevCache\cargo\target-lane-h\release\gm-desktop.exe
Info Patching ...\gm-desktop.exe with bundle type information: msi
Downloading https://github.com/wixtoolset/wix3/releases/download/wix3141rtm/wix314-binaries.zip
Info validating hash
Info extracting WIX
Running candle for "...\wix\x64\main.wxs"
Running light to produce ...\bundle\msi\gitgit Desktop_0.1.0_x64_en-US.msi
Running light to produce ...\bundle\msi\gitgit Desktop_0.1.0_x64_zh-CN.msi
Finished 1 bundle at:
```

产物：

| 文件 | 字节数 | SHA-256 |
|---|---|---|
| `gitgit Desktop_0.1.0_x64_en-US.msi` | 6,709,248 | `72B902C976C777B1136BB0CCD06107FD01E0EE7E2FF756F30022CD1439C31978` |
| `gitgit Desktop_0.1.0_x64_zh-CN.msi` | 6,705,152 | `7F8DD2EEFE45517FB2D141898ACE20206FF4223198A6183879638079E7570730` |

`[FACT]` 双语 MSI 均产出，因为 `tauri.conf.json` 的 `bundle.windows.wix.language` 配置为 `["en-US", "zh-CN"]`。

## 编译告警（未处理，需决策）

`[FACT]` `cargo build --release` 对 `gm-desktop` lib 产生 **12 条告警**，全部是 `dead_code` / `unused_variables` / `unused_imports`，集中在 `src/state.rs` 与 `src/commands/repos.rs`：

- `state.rs:40` `DEFAULT_VAULT_DIR` 从未使用
- `state.rs:68` `Running::join` 字段从未读取
- `state.rs:131` 局部变量 `already_running` 未使用
- `state.rs:369` `resolve_bind_with_default` 从未使用
- `state.rs:377` `SharedState` 类型别名从未使用
- `commands/repos.rs:110` 参数 `state` 未使用

`[FACT]` 这些**不阻塞打包**，且本次未加 `#[allow]` 掩盖。`[TBD]` 是否清理属独立决策；其中 `state.rs:369` 的 `resolve_bind_with_default` 与 `:377` 的 `SharedState` 看起来像「写了但没接线」的功能代码，删之前应先确认是否属于未完成的功能而非残留。

## 未验证项

- `[UNVERIFIED-FACT]` **安装包未实际安装运行**。产物已生成且哈希已记录，但未在干净 Windows 环境执行安装、启动、卸载全流程。
- `[UNVERIFIED-FACT]` **未做代码签名**。产出的 MSI 未签名，Windows SmartScreen / 安装时会告警。签名证书不在本次范围。
- `[TBD]` macOS `.dmg` / Linux `.appimage` / `.deb` 未验证 —— 本机为 Windows，Tauri 交叉打包这三类不受支持。
- `[TBD]` 打包未纳入 CI。`tauri build` 需要 WiX 联网下载与完整 release 编译（约 5 分钟以上），是否设为必需门禁需单独决策；当前 CI 仍只跑 `cargo check`。
