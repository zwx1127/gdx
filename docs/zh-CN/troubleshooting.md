# 故障排查

不要在公开 issue 中分享私有路径、专有资源、prompt、agent 输出、截图、daemon session token 或凭据。

## 找不到 gdx 或 Godot

运行：

```powershell
gdx doctor
```

如果 `gdx` 不在 `PATH`，在 `cargo build --workspace` 后使用仓库二进制。如果找不到 Godot，设置 `GDX_GODOT` 或传入 `--godot`。

## Godot 原生崩溃

如果 stderr JSON 报告 `error: "godot_native_crash"`，或 diagnostics 中的 `primary_error` 是 `"godot_native_crash"`，说明 Godot 在 gdx 收到 runtime JSON 之前退出。先查看输出中附带的 Godot stdout/stderr 日志和本机 Godot/runtime 环境。gdx 会把它和 GDScript parse error 区分开，但不会自动切换 Godot 二进制。

## 缺少 main scene

省略 `--scene` 时，`daemon start` 和 `capture run` 会使用项目配置的 main scene。

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

## Daemon runtime 旧于 CLI

如果 input 或 verify step 报告 `daemon_runtime_outdated`，说明当前项目里运行的 daemon runtime 不支持这个 CLI 使用的 RPC 方法。重新安装内置 runtime 并重启 daemon：

```powershell
gdx --project .\demo project install
gdx --project .\demo daemon start --restart
```

`daemon status` 会在 runtime 支持时报告 capabilities。capabilities 的 `status` 为 `unknown` 通常表示项目内 runtime 早于 capabilities RPC。

## 截图缺失或空白

检查：

- 命令 JSON 诊断。
- 如果 daemon 提前退出，先看第一个 Godot stderr 错误。
- main scene 是否存在且有可见内容。
- 2D/3D 场景在需要时是否有合适的 camera。
- 截图前是否导入资源。
- 截图分辨率是否足够。

UI 回归优先使用 `verify --spec`、项目级方法、`input click-node` 或 `input activate`。

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
