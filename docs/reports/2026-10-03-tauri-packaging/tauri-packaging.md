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

- `[UNVERIFIED-FACT]` **安装包未实际安装运行**。产物已生成且哈希已记录，但未在干净 Windows 环境执行安装、启动、卸载全流程。**已于 2026-10-04 关闭，见下节。**
- `[UNVERIFIED-FACT]` **未做代码签名**。产出的 MSI 未签名，Windows SmartScreen / 安装时会告警。签名证书不在本次范围。
- `[TBD]` macOS `.dmg` / Linux `.appimage` / `.deb` 未验证 —— 本机为 Windows，Tauri 交叉打包这三类不受支持。
- `[TBD]` 打包未纳入 CI。`tauri build` 需要 WiX 联网下载与完整 release 编译（约 5 分钟以上），是否设为必需门禁需单独决策；当前 CI 仍只跑 `cargo check`。**部分关闭：CI 已加入 `cargo fmt` / `cargo clippy -D warnings` / `cargo test --lib`，但完整 `tauri build` 仍未纳入。**

---

# 追加：安装与启动实测（2026-10-04）

上一份报告证明了「`.msi` 能被构建出来」，但从未证明「包能装上、装上后程序能跑」。本节记录这两件事的结果，以及过程中暴露的三个真实缺陷。相关提交：`344bc87`。

## 结论

`[FACT]` **应用此前根本无法启动。** 运行打包出的 `gm-desktop.exe` 立即以退出码 1 结束：

```
gitgit desktop failed to start: failed to initialize plugin `log`:
Error deserializing 'plugins.log' within your Tauri configuration:
invalid type: map, expected unit
```

`[FACT]` **安装包此前无法在非提权会话安装。** `msiexec /i ... /qn` 返回 1603，错误为 `Error 1925`，失败点在 `InstallFinalize`。

`[FACT]` 两项均已修复并实测通过：安装 → 启动 → 卸载全流程走通。

## 三个致命配置项

`[FACT]` `tauri.conf.json` 中三段 `plugins.*` 均为死配置，且各自独立致命：

| 段 | 插件 | 实际注册方式 | 后果 |
|---|---|---|---|
| `plugins.log` | tauri-plugin-log 2.9.1 | `plugin::Builder::new("log")`，**无配置类型** | 启动即崩；等级与 target 改由 Rust builder 设置 |
| `plugins.notification` | tauri-plugin-notification 2.4.0 | 同上，**无配置类型** | 启动即崩；`title` 从未被任何代码读取 |
| `plugins.updater` | tauri-plugin-updater 2.11.0 | 有真实 `Config`，但 `pubkey` **必填且无默认值** | 启动即崩；本项目无签名公钥，endpoint 指向 `.example` 域 |

`[FACT]` 第四个问题：`init_tracing()` 先 `try_init()` 了全局订阅器，日志插件随后安装自己的 logger 时失败（`attempted to set a logger after the logging system was already initialized`）。现以 `tauri-plugin-log` 为唯一日志出口。

`[FACT]` 修复后实测：窗口标题 `gitgit Desktop`，40 线程，1 个 WebView2 子进程；构建产物与安装后路径下的二进制均验证过。

## 安装范围：perUser 与 perMachine 双产出

`[FACT]` Tauri 2 的 `WixConfig` **没有** install-scope 选项，只能换模板。已把模板入库：

- `wix/main.wxs` —— perMachine 变体，与 `tauri-v2.12.1` 上游模板逐字节一致（仅多一段出处注释）。入库的意义在于：Tauri 升级时 perMachine 的行为不会在没有 diff 的情况下被悄悄改掉。
- `scripts/render-wix-template.py` —— 由上面的模板推导 `wix/main-peruser.wxs`，共 6 处改写，每处都断言恰好命中一次。上游模板漂移时会直接报错，而不是产出一份「看着对但其实错了」的文件。生成结果入库，因此构建不依赖先跑脚本。

`[FACT]` perUser 变体的 6 处改写中，有三处是踩过坑之后才知道必须存在的：

1. `InstallScope` perMachine → perUser。
   `[FACT]` **不要再额外加 `<Property Id="ALLUSERS" Value="2"/>`。** 这是整件事里最常被推荐、也最容易误导人的一步：`ALLUSERS=2` 的语义是「双用途，交给安装器按用户权限决定」，不是「强制 per-user」。加在 `InstallScope="perUser"` 之上会覆盖 WiX 已写入的 scope，非提权会话下系统随即选择 per-machine，报出一模一样的 1925。纯 per-user 包要的是 `ALLUSERS` 为空串，而 WiX 只允许通过 `InstallScope` 表达，写成 `Property` 会报 `CNDL0006`。
2. `INSTALLDIR` 从 `ProgramFiles[64]Folder` 改为 `LocalAppDataFolder\Programs`。
3. 深链注册表根 HKLM → HKCU（当前未配置深链，属预防性改动；上游模板自带这条注释）。
4. **ICE38**（硬错误）：`Path` 组件的 KeyPath 从文件改为 HKCU 注册表值。装到用户目录的组件必须用 per-user keypath。
5. **ICE64**（硬错误）：`ProgramsFolder` 的 `RemoveFolder` 必须声明在一个**位于该目录内部**、且被 MainProgram 特性引用的组件里。挂在 INSTALLDIR 下的卸载组件上时，该目录确实进了 MSI 的 RemoveFile 表（已用读取产物 MSI 表的方式核实），ICE64 依然失败。

`[FACT]` 另注：XML 注释内部不允许出现连续两个连字符。`candle` 遇到这种写法时报的是 `CNDL0104: Not a valid source file`，但真实原因是注释被提前截断、剩余内容成了根元素之前的游离文本。排查时被这句错误信息带偏过一轮。

## 端到端实测（非提权会话）

`[FACT]` perUser，`/qn`：

| 步骤 | 结果 |
|---|---|
| 安装 | `msiexec` 退出码 **0** |
| 文件落地 | `%LOCALAPPDATA%\Programs\gitgit Desktop\gm-desktop.exe`（19,580,928 B，合法 PE） |
| 开始菜单 | 快捷方式已创建 |
| 注册表 | `HKCU\Software\Ulysses\gitgit Desktop` 下 6 个值：`InstallDir`、`Path`、`Desktop Shortcut`、`Start Menu Shortcut`、`Uninstaller Shortcut`、`ProgramsFolderCleanup` |
| 启动 | 安装后的二进制可启动并打开窗口 |
| 卸载 | 退出码 **0**；安装目录、HKCU 键、开始菜单项全部移除 |
| `%LOCALAPPDATA%\Programs` | 保留 —— 该目录是共享的，`RemoveFolder` 只删空目录，行为正确 |

`[FACT]` 两个变体均出 en-US / zh-CN 四件套，且字节不同：

| 变体 | 语言 | 字节数 | SHA-256 |
|---|---|---|---|
| per-user | en-US | 6,742,016 | `1B3F5EC6A6DBE35773246FC84566AFE8680FF3939F7D8CB92F9D4EA65C7DF192` |
| per-user | zh-CN | 6,737,920 | `ADBEFD7F5510197C3F95DA96F5FF6C8D01E6D61362BECB4DD4C5A604EF2AFBEB` |
| per-machine | en-US | 6,742,016 | `FD2C294FF612762CC94FAF98F799CC64CC4280F36F77F36DBBB786DB6E7DBF39` |
| per-machine | zh-CN | 6,737,920 | `B5A3482F4D7C83112CD857D68FD3E0CDB4218E1C090C9BB24E0B66748F9BC5C7` |

`[FACT]` 产物内二进制的哈希与 `target/release/gm-desktop.exe` 不同，这是预期的：Tauri 在打包后会向二进制写入 bundle 类型标记，构建日志中有对应一行。

## 图标（原缺口 3：已关闭）

`[FACT]` `apps/gm-desktop/scripts/generate-icons.py` 产出 1024×1024 主图（提交图：主干三点、右上分出两点，笔画占画布 48/1024，保证 32×32 下仍不消失）与 64×64 透明托盘图（`tray.png` 由 `lib.rs` 以 `include_bytes!` 直接引用，不在 Tauri 生成集内）。其余尺寸由 `pnpm tauri icon` 生成，`.ico` 内含 16/24/32/48/64/256 六档。Tauri 顺带生成的 `ios/` 与 `android/` 资源树已删除 —— 本项目是纯桌面产品，无移动端目标。

`[INFERENCE]` 该标记是中性几何占位，不是品牌美术。真实素材到位后替换 `icon-source.png` 并重跑 `pnpm tauri icon` 即可，构建没有任何代码按内容引用这些文件，届时脚本可删。

## 仍未关闭

- `[UNVERIFIED-FACT]` **未做代码签名**。四个 MSI 均未签名，Windows SmartScreen 会告警。需购买代码签名证书。
- `[INFERENCE]` **两个变体写入同一输出目录** `target/release/bundle/msi/`，后构建的会覆盖先构建的 —— 上述 SHA-256 只能在两次构建之间分别抓取。若日后纳入发布流水线，需要在构建后重命名区分。
- `[TBD]` 完整 `tauri build` 仍未纳入 CI（WiX 需联网下载 + 5 分钟以上 release 编译）。CI 现阶段覆盖到 `cargo fmt --check` / `cargo clippy -D warnings` / `cargo test --lib`。
- `[TBD]` macOS `.dmg` / Linux `.appimage` / `.deb` 未验证，本机为 Windows。
- `[TBD]` T7 真打通 OpenAI 仍未验证，仓内无 `GITGIT_AI_API_KEY` 凭据。

