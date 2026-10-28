# AGPL-3.0 §6(d) — the Corresponding Source notice in the `.msi` and the `.dmg`

> `[FACT]` 本文事实由父代理在 2026-10-28 实测得到。标注约定：
> `[FACT]` 实测或官方文档原文；`[INFERENCE]` 由事实推出；`[PROPOSAL]` 建议。

## 1. 义务是什么

AGPL-3.0 §6(d)（`LICENSE:263-275`）要求：当目标代码通过「指定地点」交付时，
目标代码旁须有**书面指引**，说明到何处可获得 Corresponding Source。

`docs/plan/v0-tasks.md` 的风险登记里有一行记着这项义务的现状。本文档记的是
**机制**、**实测的失败**、以及**每个产物现在带什么**。

`[FACT]` **合法性判断仍然开放。** 本文只闭合**机械**缺口（文件确实在产物里、
检查确实会因它缺失而红）。该义务是否被满足，是需要人来裁定的事，仓里已经
这样写着，本文不改变这一点。

## 2. 机制（从上游源码读出来的，不是猜的）

`[FACT]` `bundle.resources`（`tauri-utils` 的 `BundleConfig.resources:
Option<BundleResources>`）是一个 **源 → 目标** 的映射。方向是从
`tauri-utils` 的 `ResourcePaths::from_map` 读出来的：map 的**键是源**，
**值是目标**（与 `bundle.linux.deb.files` 的方向相反，后者是目标 → 源）。

`[FACT]` bundler 的 `Settings::resource_files()`（`tauri-bundler-2.10.4`
`src/bundle/settings.rs:1180`）把这**同一个键**交给所有读它的 bundler。

`[FACT]` **MSI**：`tauri-bundler-2.10.4` `src/bundle/windows/msi/mod.rs` 的
`generate_resource_data` 遍历 `settings.resource_files()`，把目标的目录部分
做成 `<Directory>`、最后一段做成文件的 `Name`，并把结果以 `resources` 键交给
WiX 模板（文件 id 以 `resource_file_ids` 交给模板的 `<ComponentRef>`）。
本仓自绘的模板本来就有这两个槽位：`wix/main.wxs:249`（`{{resources}}`）与
`main.wxs:307`（`{{#each resource_file_ids …}}`）。

`[FACT]` **macOS / `.dmg`**：`tauri-bundler-2.10.4` `src/bundle/macos/app.rs:97`
调用 `settings.copy_resources(&resources_dir)?`，目标是
`<app>.app/Contents/Resources`。`.dmg` 里装的就是那个 `.app`。

`[FACT]` **配置里的路径基准是 Tauri 工程目录**：`tauri-cli` 的 `build.rs:166`
在打包前 `set_current_dir(dirs.tauri)`，所以 `"../../../CORRESPONDING-SOURCE.md"`
解析到仓库根那一份。

## 3. 被证伪的两条旧说法

`[FACT]` **`docs/plan/v0-tasks.md` 原来写着「dmg bundler 也不读
`bundle.resources`，配置里没有能放进去的键」——这句是错的。** macOS 的 bundler
读它（上一节）。`CORRESPONDING-SOURCE.md` 原来也重复了同一句错话。

`[FACT]` **「非空的 `bundle.resources` 会破坏 MSI 构建」这句话也太笼统。**
run `37644440611`（2026-10-07）的 CI 日志显示：**per-machine 那一遍成功**了
（产出了 7.54 MiB 的 MSI），失败的是 **per-user** 那一遍。
`gh run view 37644440611 --repo UlyssesLeoLee/GitGit --log` 可复核。

## 4. 根因（实测，不是推断）

`[FACT]` Tauri 的 bundler 把每个 `bundle.resources` 条目渲染成：

```xml
<Component Id="…" Guid="…" Win64="$(var.Win64)" KeyPath="yes">
  <File Id="PathFile_…" Source="…" Name="…"/>
</Component>
```

即 **KeyPath 是一个文件**。per-user 的包装进 `%LOCALAPPDATA%\Programs`，而
**ICE38** 要求装进用户目录的组件必须用 **HKCU 注册表值**做 KeyPath。

`[FACT]` 本机实测（用最小 WiX 探针，`candle.exe` / `light.exe` 在本机可用）：

```
error LGHT0204 : ICE38: Component Res installs to user profile.
It must use a registry key under HKCU as its KeyPath, not a file.
```

`light` 退出码 **204**。

`[FACT]` 本仓自己的 per-user 模板里早就有这条适配：`render-wix-template.py`
里有一条 edit 就叫 *"ICE38: Path component keypath moves from the file to
HKCU"*。

## 5. 本轮怎么绕开它

`[FACT]` **per-user 模板不再渲染资源。** `render-wix-template.py` 新增两条
edit，把 `{{resources}}` 槽位与 `{{#each resource_file_ids …}}` 循环从
**per-user** 变体里去掉（`main.wxs` 保留）。于是：

- per-user 的 MSI 里没有资源组件 → ICE38 无从发作 → 构建通过；
- per-machine 的 MSI 保留两个槽位 → 指引文件落在安装目录里、与可执行文件并排；
- `.dmg` 由 macOS bundler 复制到 `Contents/Resources/`。

`[FACT]` **代价要说清楚：per-user 的 `.msi` 不带指引文件。** 那是一个真实的
§6(d) 缺口，不是遗漏；它原来也没有。`CORRESPONDING-SOURCE.md` 的
"What this notice does not cover" 一节现在**只**记这一条。

## 6. 门禁

`[FACT]` **`.msi`**：`apps/gm-desktop/scripts/verify-msi.ps1` 做的行政安装
（`msiexec /a`）后按**内容**找指引文件（三个标记：`Corresponding Source for
this build`、`AGPL-3.0`、以及从 `bundle.homepage` 推出的指定地点），per-machine
变体缺失即红。per-user 变体**不**断言（断言一个永远不成立的东西，等于一个
会被关掉的门禁），而是打印一条黄字说明它缺什么、为什么。

`[FACT]` **`.dmg`**：`apps/gm-desktop/scripts/verify-bundle.sh` 里原来那条
「NO CORRESPONDING SOURCE NOTICE inside the mounted image」只打印不失败；
现在它**失败**，与 `.deb` / `.AppImage` 两个分支同一套判据。

`[FACT]` **钉住不变量的测试**：`apps/gm-desktop/tests/unit/agpl-license-gate.test.ts`
里原来那条「`bundle.resources` 必须为空」的断言被**重写**（不是删除、不是放宽）：
现在它断言 `bundle.resources` 带的是仓库那一份 `CORRESPONDING-SOURCE.md`
（逐字节）、目标没有目录部分、**per-machine 模板有那两个槽位而 per-user 模板
没有**，并把 ICE38 的实测原因写进注释。

## 7. 没能测到的部分

`[FACT]` **`.msi` 与 `.dmg` 的产物级证据来自 CI**：`.msi` 在
`windows-latest` 上构建（本机整包构建在 `light.exe` 阶段以 `LGHT0094` 失败，
而 CI 每轮成功），`.dmg` 在 `macos-latest` 上构建（Windows 上做不出、也挂不了
一个 `.dmg`）。本机能测的是 **WiX 层的 ICE38**：最小探针在本机跑通了
`candle` 与 `light`，红臂退出 204。

`[FACT]` **per-user 变体的缺口是否可闭合，本轮没有结论。** 上游 bundler 的
渲染是固定的字符串，模板改不了它；能想到的另一条路是在 per-user 模板里**手写**
一个带 HKCU KeyPath 的组件，而它的 `Source` 必须能被 `candle` 解析——`candle`
把**相对** `Source` 解析到 `<target>/release/wix/x64/`（bundler 把
`update.xml`、`install-task.ps1` 写在那里），所以需要一份相对路径；这条路
依赖 target 目录的层级，很脆，本轮**没有**实现，也**没有**声称它可行。
