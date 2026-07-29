# gdx CLI 参考

`gdx` 是无状态、可组合的 Godot CLI。成功命令只向 stdout 写一行紧凑 JSON；进度和失败写入 stderr。使用 `--progress ndjson` 时，失败对象是 stderr 的最后一行。

需要保留输出时直接使用 shell 重定向：

```powershell
gdx --project .\demo --progress ndjson script check-all 1>result.json 2>progress.ndjson
```

gdx 不在项目中持久化命令结果、进度或一次性 Godot 日志。构建来源信息只由 `doctor` 返回。

## 全局选项

```powershell
gdx --project <project-dir> --progress auto <command>
```

- `--godot <path>`：显式指定 Godot。
- `--godot-standard <path>` / `GDX_GODOT_STANDARD`：配置标准版 Godot。
- `--godot-dotnet <path>` / `GDX_GODOT_DOTNET`：配置 .NET 版 Godot。
- `--project <dir>`：指定已有 Godot 项目。
- `GDX_GODOT`：`--godot` 的环境变量形式。
- `--progress auto|stderr|ndjson|off`：TTY 默认显示人类可读进度，非 TTY 默认 NDJSON；长命令会发送 heartbeat。

Godot 选择顺序是：显式 `--godot`、与项目语言匹配的标准/.NET 配置、`GDX_GODOT`、`PATH`。

## 环境与项目

```powershell
gdx doctor
gdx project create --path .\demo --name Demo
gdx --project .\demo project update --check
gdx --project .\demo project update
gdx --project .\demo project update --force
gdx --project .\demo project inspect
```

`project create` 只接受不存在或空目录。它创建最小项目、安装 `addons/gdx_*` 管理文件，并且只向 `.gitignore` 增加 `.godot/`。

`project update` 是管理 addon 的唯一安装/更新入口。`--check` 只报告待变更文件，`--force` 重写全部管理文件。更新已有项目不会修改 `.gitignore`。

`project inspect` 只返回项目标识、主场景、语言和 addon 状态。`doctor` 才返回可执行文件、Godot、.NET、项目语言和 runtime 的详细诊断。

## 设置、autoload 与 Input Map

```powershell
gdx --project .\demo setting get --section application --key run/main_scene
gdx --project .\demo setting set --section application --key run/main_scene --value res://scenes/main.tscn
gdx --project .\demo setting list --section application
gdx --project .\demo autoload add --name GameState --path res://scripts/game_state.gd --global
gdx --project .\demo autoload remove --name GameState
gdx --project .\demo autoload list
gdx --project .\demo input-map add --action jump --keycode 32
gdx --project .\demo input-map remove --action jump
gdx --project .\demo input-map list
```

## 资产、脚本、场景与资源

```powershell
gdx --project .\demo asset import
gdx --project .\demo asset inspect --path res://assets/player.png

gdx --project .\demo script create --path res://scripts/main.gd --extends Node2D
gdx --project .\demo script attach --scene res://scenes/main.tscn --node / --script res://scripts/main.gd
gdx --project .\demo script check res://scripts/main.gd
gdx --project .\demo script check-all --jobs 4 --script-timeout-secs 60 --timeout-secs 300 --exclude res://vendor
gdx --project .\demo script load-check

gdx --project .\demo scene create --out res://scenes/main.tscn --root-type Node2D --name Main --set-main
gdx --project .\demo scene build --spec .\scene-spec.json
gdx --project .\demo scene tree
gdx --project .\demo scene save

gdx --project .\demo resource create --type StandardMaterial3D --out res://materials/basic.tres
gdx --project .\demo resource inspect --path res://materials/basic.tres
```

用 shell 或构建系统复制业务资产；项目工作流判断需要导入时再显式执行 `asset import`。`script check-all` 使用有界并行，并只返回汇总计数，不展开每个成功脚本。

## Daemon 与运行时命令

```powershell
gdx --project .\demo daemon start --restart --width 1280 --height 720
gdx --project .\demo daemon status
gdx --project .\demo scene tree --include-script --include-groups --include-methods
gdx --project .\demo node create --parent / --type Label --name Status
gdx --project .\demo node set --node /Status --property text --value "Ready"
gdx --project .\demo scene save
gdx --project .\demo input send --mouse-button 1 --position 120 240 --pressed false
gdx --project .\demo input click --position 120 240
gdx --project .\demo input click-node --target /StartButton
gdx --project .\demo input touch --position 120 240 --pressed true
gdx --project .\demo input tap --position 120 240
gdx --project .\demo input drag --from 120 240 --to 220 260
gdx --project .\demo input pinch --center 180 240 --start-distance 120 --end-distance 40
gdx --project .\demo input sequence --spec .\touch-sequence.json
gdx --project .\demo input activate --target /StartButton
gdx --project .\demo call invoke --target / --method start_game --args-json "[]"
Get-Content -Raw .\args.json | gdx --project .\demo call invoke --target / --method start_game --args-json -
gdx --project .\demo call invoke --target / --method show_area --arg "河流 石桥"
gdx --project .\demo state get --target /
gdx --project .\demo capture daemon --out .\artifacts\capture.png
gdx --project .\demo daemon stop
```

`daemon start` 在启动 Godot 前验证管理 runtime 的兼容性，并检查 RPC、场景加载与启动硬错误。新建 daemon 不健康时会自动停止；已经运行但不健康的 daemon 只报告错误，保留给开发者检查或停止。

活动 daemon 的 session 和当前日志存放在按项目规范路径散列的用户 OS 临时目录中。`daemon stop`、陈旧 session 检测和启动失败都会清理这些状态。`daemon start/status` 只返回紧凑的 `runtime` 与 `health`，不会返回 token 或完整 capability 列表。

触控命令要求 runtime 支持 `touch_sequence`，不会降级成鼠标输入。拖动和滑动都使用 `input drag`；释放事件使用 `input send --pressed false`。重复 `--arg` 只生成字符串；类型化参数使用 `--args-json <数组>`，或用 `--args-json -` 从 stdin 读取。JSON 错误只报告来源和位置，不复制输入内容。

## Verify、捕获、测试与导出

```powershell
gdx --project .\demo verify --spec .\verify.json
gdx --project .\demo capture run --scene res://scenes/main.tscn --out .\artifacts\capture.png
gdx --project .\demo capture record --scene res://scenes/main.tscn --out .\artifacts\recording.avi --duration 3 --fps 60
gdx --project .\demo test run --path res://tests/smoke_test.gd --method run_tests
gdx --project .\demo export build --preset "Windows Desktop" --out .\export\game.exe
```

`verify` 是用于脚本检查、Godot 测试、daemon 调用、状态读取、输入与截图的通用声明式编排器。结果只返回紧凑的 `steps` 摘要，不复制每个子命令的完整 envelope。产品断言、视觉基线、比较指标、阈值、mask 和验收规则属于游戏业务层。

`capture run` 等一次性 Godot 命令使用命令结束即删除的临时日志；失败只保留有界结构化诊断尾部，不返回日志路径。`capture daemon`、`capture record` 和 export 只返回调用方明确要求生成的 artifact。
