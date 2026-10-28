# AGPL-3.0 §6(d) — why the per-user MSI cannot carry a `bundle.resources` entry

> `[FACT]` 本文是一次**本地实测**的记录。2026-10-28。
> 结论与设计取舍见 `docs/plan/agpl-corresponding-source-msi-dmg.md`。

## 记的是什么

`bundle.resources` 是让指引文件进入 **`.msi`（per-machine）与 `.dmg`** 的机制。
它也是**唯一**的机制：Tauri 的 bundler 把这一个键交给所有读它的 bundler。

问题在于：**per-user 的 `.msi` 用不了它。**

## 实测

`wixprobe/` 里是一个**最小复现**：一个 per-user 包的 WiX 源，两个组件。

- `Path` — 可执行文件那个组件，KeyPath 是 **HKCU 注册表值**
  （这正是本仓 `render-wix-template.py` 里那条 *"ICE38: Path component keypath
  moves from the file to HKCU"* edit 做的事）。
- `Res` — Tauri bundler 为每个 `bundle.resources` 条目渲染出来的形状：
  `<Component … KeyPath="yes"><File …/></Component>`，即 **KeyPath 是一个文件**。

复现（Windows，WiX 工具链 `candle.exe` / `light.exe`）：

```
> candle -arch x64 probe.wxs
> light -o probe.msi probe.wixobj
error LGHT0204 : ICE38: Component Res installs to user profile.
It must use a registry key under HKCU as its KeyPath, not a file.
```

`light` 退出码 **204**。

## 怎么自己跑一遍

```powershell
# 工具链：cargo install tauri-cli 之后在 %LOCALAPPDATA%\tauri\WixTools314\ 下
$candle = "$env:LOCALAPPDATA\tauri\WixTools314\candle.exe"
$light  = "$env:LOCALAPPDATA\tauri\WixTools314\light.exe"
& $candle -arch x64 probe.wxs
& $light -o probe.msi probe.wixobj      # <- LGHT0204, exit 204
```

`probe.exe` 与 `notice.md` 是这份源里引用的两个文件（各是一小段占位内容），
它们只需要存在即可。

## CI 里的那一次

历史上第一次踩到这件事的是 **run 37644440611**（2026-10-07）：

```
gh run view 37644440611 --repo UlyssesLeoLee/GitGit --log
```

那份日志显示 **per-machine 那一遍成功**了（产出 7.54 MiB 的 MSI），失败的只有
per-user 那一遍。所以「非空的 `bundle.resources` 会破坏 MSI 构建」这句话当年
只对**两个变体中的一个**成立。

`[FACT]` **CI 日志里没有 ICE 的原文**——Tauri 把 `light.exe` 的输出吞了，只
留下 `failed to run …\light.exe`。上面那份 `LGHT0204` 是本地复现出来的，
这正是当年缺的那一环。
