# AGPL-3.0 §6(d) — the per-user MSI CAN carry the Corresponding Source notice

> `[FACT]` 本文是一次**本地实测**的记录。2026-10-31。
> 前一篇（失败的形状）在 `docs/reports/2026-10-28-agpl-corresponding-source/`。

## 记的是什么

上一轮记的是一个**死路**：Tauri bundler 把每个 `bundle.resources` 条目渲染成
**KeyPath 是文件**的组件，而 ICE38 不许装进用户目录的组件用文件作 KeyPath →
`light.exe` 报 `LGHT0204` 并退出 204。所以 per-user 的 `.msi` 不带指引文件。

这一轮记的是**另一条路**，以及它为什么成立。

## 形状

组件的 KeyPath 是 **HKCU 注册表值**时，它可以**额外携带 File 子节点**——正是仓库
per-user 模板里 `Path` 组件（可执行文件那个）已经在用的形状。指引文件就挂在它旁边。

## 实测

```powershell
$candle = "$env:LOCALAPPDATA\tauri\WixTools314\candle.exe"
$light  = "$env:LOCALAPPDATA\tauri\WixTools314\light.exe"
& $candle -arch x64 probe.wxs
& $light  -o probe2.msi probe.wixobj
```

```
candle exit=0
light  exit=0
```

（旧的形状是 `light` 退出 204、`LGHT0204 : ICE38`。）

而且文件**真的在 MSI 里**——读 MSI 的 File 表（Windows Installer COM）：

```
Path                      |  probe.exe                        |  Path
CorrespondingSourceNotice |  ukioxpnv.md|CORRESPONDING-SOURCE.md |  Path
```

`probe.exe` 与 `notice.md` 是这份源里引用的两个占位文件，只需要存在即可。

## 这条路在真实构建里怎么落地

| 环节 | 事实 |
|---|---|
| 模板 | per-user 模板给 `Path` 组件加一个 File 子节点：`Source="../../resources/CORRESPONDING-SOURCE.md"`（**相对**路径） |
| 相对路径为什么成立 | `candle` 的工作目录是 `<out>/wix/<arch>`，而文件放在 `<out>/resources`——那正是 bundler 复制图标的地方，且**不会被清**（bundler 只删 `<out>/wix/<arch>`） |
| 文件怎么到那儿 | `build-msi.ps1` 在构建前把仓里的 `CORRESPONDING-SOURCE.md` 复制进 `<target>/release/resources/` |
| 为什么不用绝对路径 | 绝对路径会把已提交的模板绑死在某台机器上；相对路径在任何机器、任何 `CARGO_TARGET_DIR` 下都成立 |
| per-machine 与 `.dmg` | 仍走 `bundle.resources`，**不动**——两边都加会让同一文件被装两遍 |

## 实测的边界

- `[FACT]` 本探针证明的是**机制**（形状能链接、文件在包里）。
- `[UNVERIFIED-FACT]` 真实 MSI 的内容断言只能由 CI 给出：本地整包 MSI 构建在
  `light.exe` 处 LGHT0094 失败。CI 的 `msi` 作业构建两个 scope，并由
  `apps/gm-desktop/scripts/verify-msi.ps1` 对 **两个 scope 都**按内容断言指引存在。
