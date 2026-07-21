# 故障排查

不要在公开 issue 中分享私有路径、专有资源、prompt、agent 输出、截图、daemon session token 或凭据。

## 找不到 gdx 或 Godot

运行：

```powershell
gdx doctor
```

如果 `gdx` 不在 `PATH`，在 `cargo build --workspace` 后使用仓库二进制。如果找不到 Godot，设置 `GDX_GODOT` 或传入 `--godot`。混合项目环境可分别配置 `GDX_GODOT_STANDARD` 和 `GDX_GODOT_DOTNET`；显式 `--godot` 始终优先。

`gdx doctor` 会识别 GDScript/C# 项目、选中的标准/.NET Godot 及已安装 SDK。纯 GDScript 项目使用 .NET Godot 且缺少 SDK 时只返回 warning；C# 项目缺少 .NET Godot 或 .NET 8+ SDK 时会在运行项目前失败。

## 长命令看起来卡住

使用 `--progress ndjson` 并逐行解析 stderr。命令会立即发出 `command_started`，之后至少每 5 秒发出一次 `heartbeat`。Godot 子进程事件包含 PID、subject、timeout 和日志路径。超时或 Ctrl+C 后，gdx 会保留日志并终止一次性进程树；取消的退出码是 130。

对于 `script check-all`，可用 `--jobs 1` 或 `--jobs 2` 降低并行度，用可重复的 `--exclude res://...` 缩小范围，并分别调整单脚本 `--script-timeout-secs` 与全局 `--timeout-secs`。

## Godot 原生崩溃

如果 stderr JSON 报告 `error: "godot_native_crash"`，或 diagnostics 中的 `primary_error` 是 `"godot_native_crash"`，说明 Godot 在 gdx 收到 runtime JSON 之前退出。先查看输出中附带的 Godot stdout/stderr 日志和本机 Godot/runtime 环境。gdx 会把它和 GDScript parse error 区分开，但不会自动切换 Godot 二进制。

## 缺少 main scene

省略 `--scene` 时，`daemon start`、`capture run` 和 `capture record` 会使用项目配置的 main scene。

可用以下任一方式修复：

```powershell
gdx --project .\demo scene create --out res://scenes/main.tscn --root-type Node2D --name Main --set-main
gdx --project .\demo setting set --section application --key run/main_scene --value res://scenes/main.tscn
gdx --project .\demo daemon start --scene res://scenes/main.tscn
```

## 脚本检查失败

先阅读 stderr JSON：

- `error.code`
- `message`
- `suggestion`
- `details.diagnostics.primary_error`
- `artifacts.stderr_log`

修复第一个 parser 或 load error，然后重新运行：

```powershell
gdx --project <project> script check-all
```

如果诊断显示 warning-as-error 行为，检查是否有从 `Dictionary` 或 `Variant` 派生的 `:=`。优先使用普通赋值或显式类型。

## Asset import 报 .NET 或 hostfxr warning

当项目没有 C# 文件时，`asset import` 可能返回包含 `skipped: true` 和 `reason: mono_dotnet_unavailable` 的 JSON。对非 C# 项目可把它视为 warning，然后继续脚本和运行时检查。

如果项目使用 C#，先安装所需的 .NET 和 Godot Mono 依赖。

## Daemon 已运行或状态过期

使用：

```powershell
gdx --project .\demo daemon status
gdx --project .\demo daemon start --restart
gdx --project .\demo daemon stop --force
```

Daemon session 数据位于 `.gdx/daemon/session.json`。优先使用命令清理，不要手动删除文件。

如果 `daemon start` 报告 `project_unhealthy`，先查看 `health.first_error` 和附带的 daemon 日志。解析、资源和场景加载错误会阻断启动，新启动的 daemon 会被自动停止；纯 GDScript 项目中的缺 SDK 信息只视为环境 warning。`--no-health-check` 仅用于明确的诊断场景，并会返回 `project_healthy: null`。

## 运行历史占用磁盘

删除前先预览：

```powershell
gdx --project .\demo runs list
gdx --project .\demo runs prune
gdx --project .\demo runs prune --apply
```

第一次实际 prune 会把旧 `.gdx/runs` 目录纳入后续自动保留策略。运行中的记录和 daemon 状态受保护。应依据返回的 `candidate_count`、`candidate_bytes`、`deleted` 和 `freed_bytes` 字段判断结果，不要手动删除运行目录。

## Daemon runtime 旧于 CLI

如果 input 或 verify step 报告 `daemon_runtime_outdated`，说明当前项目里运行的 daemon runtime 不支持这个 CLI 使用的 RPC 方法。更新内置 runtime 并重启 daemon：

```powershell
gdx --project .\demo project update --check
gdx --project .\demo project update
gdx --project .\demo daemon start --restart
```

`daemon status` 会在顶层返回 `runtime_status`、`runtime_version`、`protocol_version`、`methods` 和 `warnings`。`runtime_status: "unknown"` 表示项目内 runtime 早于 capabilities RPC；`runtime_status: "outdated"` 表示 runtime 已知但缺少 `touch_sequence` 等能力。

Touch 命令和 verify touch step 不会降级成鼠标事件。项目 runtime 缺少 `touch_sequence` 时，更新 managed runtime 文件并重启 daemon。

## 截图缺失或空白

检查：

- 命令 JSON 诊断。
- 如果 daemon 提前退出，先看第一个 Godot stderr 错误。
- main scene 是否存在且有可见内容。
- 2D/3D 场景在需要时是否有合适的 camera。
- 截图前是否导入资源。
- 截图分辨率是否足够。

UI 回归优先使用 `verify --spec`、项目级方法、`input click-node` 或 `input activate`。移动端玩法如果处理 `InputEventScreenTouch` 或 `InputEventScreenDrag`，使用 `input tap`、`input swipe`、`input pinch` 或 `input sequence` 等触控命令。Touch 命令要求 daemon runtime 支持 `touch_sequence`，并且不会降级成鼠标事件。

## 录制缺失或空白

`capture record` 通过 Godot Movie Writer 写出 AVI，并会重新启动一个场景实例；它不会录制已经运行中的 daemon session。

使用 `capture record --input-sequence <json>` 可以在 fresh scene 中回放 touch 事件并录制手势过程。调试时使用 `.avi` 输出路径，并先保持较短的 `--duration` 和较低的 `--fps`。如果 Godot 退出但没有录制文件，优先查看 `artifacts.stderr_log`。

## Scene build 失败

检查：

- spec 是否有顶层 `out` 和 `root`。
- 每个 `type` 是否是真实 Godot 类。
- 每个脚本和 resource 路径是否存在于项目内。
- Godot 资源是否使用 `res://` 路径。
- 向量和颜色属性是否使用 scene-spec reference 中记录的 wrapper。

## 导出失败

导出需要：

- 项目中存在 `export_presets.cfg`。
- preset 名称与命令匹配。
- 已安装 Godot export templates。
- 输出目录已经存在。

如果导出不是任务必需项，报告导出阻塞原因，并保留已完成的运行时和测试验证。
