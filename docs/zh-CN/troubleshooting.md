# 故障排查

不要在公开 issue 中分享私有路径、专有资产、prompt、agent 输出、截图、daemon token 或凭据。

## 找不到 gdx 或 Godot

```powershell
gdx doctor
```

如果 Godot 不在 `PATH`，设置 `GDX_GODOT` 或传入 `--godot`。混合环境可分别配置 `GDX_GODOT_STANDARD` 与 `GDX_GODOT_DOTNET`；显式 `--godot` 优先。

`doctor` 会识别 GDScript/C#、选中的标准/.NET Godot 以及已安装 SDK。C# 项目缺少 .NET Godot 或 .NET 8+ SDK 时会在执行项目命令前失败。

## 长命令看起来卡住

使用 `--progress ndjson` 逐行读取 stderr。命令会立即发送 `command_started`，长操作至少每五秒发送 heartbeat。需要保留输出时由 shell 重定向：

```powershell
gdx --project .\demo --progress ndjson script check-all 1>result.json 2>progress.ndjson
```

不要用 stderr 是否为空判断成功，应检查进程退出码。超时或 Ctrl+C 时，gdx 会终止一次性进程树，从临时日志提取有界诊断后删除日志；取消退出码为 130。

`script check-all` 资源竞争时可使用 `--jobs 1` 或 `--jobs 2`，用重复 `--exclude res://...` 缩小范围，并分别调整脚本与总超时。

## Godot 原生崩溃

`error: "godot_native_crash"` 表示 Godot 在返回 runtime JSON 前退出。检查 stderr 中的结构化 stdout/stderr 尾部和本地运行环境。gdx 会区分原生崩溃与 GDScript 解析错误，但不会自动切换 Godot。

## 缺少主场景

```powershell
gdx --project .\demo scene create --out res://scenes/main.tscn --root-type Node2D --name Main --set-main
gdx --project .\demo setting set --section application --key run/main_scene --value res://scenes/main.tscn
gdx --project .\demo daemon start --scene res://scenes/main.tscn
```

## 脚本检查失败

先读取 stderr JSON 的 `error`、`message`、`suggestion` 与 `details.diagnostics.primary_error`。修复第一个解析/加载错误后重新执行：

```powershell
gdx --project <project> script check-all
```

如果 Godot 把类型推断 warning 当成错误，避免对来自 `Dictionary` 或 `Variant` 的值使用 `:=`，改用普通赋值或显式类型。

## Daemon 已运行、陈旧或不健康

```powershell
gdx --project .\demo daemon status
gdx --project .\demo daemon start --restart
gdx --project .\demo daemon stop --force
```

活动 daemon 的 session 与当前日志位于按项目规范路径散列的用户 OS 临时目录中。`daemon stop` 和陈旧 session 检测会自动清理。旧版本留下的 `.gdx/daemon` 不再是当前 daemon 状态。

`project_unhealthy` 时检查 `health.first_error`。新启动的异常 daemon 会自动停止；已经运行的异常 daemon 保留给开发者检查或显式停止。`--no-health-check` 只用于诊断。

## 旧 `.gdx/runs` 占用磁盘

当前 gdx 不创建或管理运行历史。已有 `.gdx/runs` 来自旧版本，gdx 为避免误删用户数据会保持不动。请用普通文件工具审阅后自行归档或删除。

## Daemon runtime 比 CLI 旧

```powershell
gdx --project .\demo project update --check
gdx --project .\demo project update
gdx --project .\demo daemon start --restart
```

`daemon status` 在 `runtime` 中返回 `status`、`version` 与 `protocol_version`。稳定状态为 `current`、`outdated`、`unknown`、`not_running`。Touch 命令不会降级为鼠标事件。

## 截图缺失或空白

检查：

- 命令的结构化诊断与第一个 Godot 错误。
- 主场景是否存在并有可见内容。
- 2D/3D 场景是否配置了需要的相机。
- 资产是否已显式导入。
- 捕获分辨率是否符合目标 viewport。

UI 回归优先使用 `verify --spec`、项目级方法、`input click-node` 或 `input activate`。触控玩法使用 `input tap`、`input drag`、`input pinch` 或 `input sequence`。

gdx 只生成 artifact。视觉 baseline、比较指标、阈值、mask 与验收规则属于游戏业务层。

## 录制缺失或为空

`capture record` 使用 Godot Movie Writer 启动一个新场景，不录制当前 daemon。使用 `.avi` 输出；调试时缩短 `--duration` 并降低 `--fps`。失败时检查 stderr 结构化诊断。

## Scene build 失败

确认 spec 包含顶层 `out` 与 `root`，所有 `type` 都是 Godot 类，脚本/资源位于项目内并使用 `res://` 路径，向量和颜色使用 scene-spec 文档中的 wrapper。

## Export 失败

需要项目中的 `export_presets.cfg`、匹配的 preset 名称、已安装的 export templates，以及存在的输出目录。
