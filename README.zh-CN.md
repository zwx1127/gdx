# gdx

中文 | [English](README.md)

`gdx` 是用于真实 Godot 4.x 项目的 Rust 自动化 CLI。它封装 Godot 官方命令行工作流，并通过 JSON 命令提供项目脚手架、场景编辑、运行时控制、截图、录制、测试与导出。

`gdx` 不是游戏迁移框架或引擎抽象层。调用方仍负责游戏设计、业务架构、GDScript、场景 spec、资产、测试逻辑和失败分析；gdx 只提供可靠、无状态、可组合的 Godot 开发工具。

## 能做什么

- 创建最小 Godot 项目，或为已有项目安装/更新 `addons/gdx_*` 管理文件。
- 设置项目配置、autoload 与 Input Map。
- 导入和检查资产。
- 创建、挂载并检查 GDScript。
- 直接创建场景，或从 JSON spec 构建场景。
- 启动本地 daemon，执行实时场景编辑、输入、方法调用、状态读取和截图。
- 使用 Godot Movie Writer 录制短 AVI。
- 运行 Godot 测试和声明式 verify spec。
- 通过 Godot export preset 构建导出。

成功命令只向 stdout 输出一行紧凑 JSON。进度与失败写入 stderr；失败对象是 stderr 的最后一行。`doctor` 返回实际 gdx 可执行文件、版本和构建 commit。

gdx 不在游戏项目中持久化命令结果、进度或一次性日志。需要保留时直接使用 shell：

```powershell
gdx --project .\demo --progress ndjson script check-all 1>result.json 2>progress.ndjson
```

## 快速开始

```powershell
cargo build --workspace
target\debug\gdx.exe doctor

target\debug\gdx.exe project create --path .\demo --name Demo
target\debug\gdx.exe --project .\demo scene create `
    --out res://scenes/main.tscn --root-type Node2D --name Main --set-main
target\debug\gdx.exe --project .\demo project inspect
```

Godot 不在 `PATH` 时，使用 `--godot <path>` 或设置 `GDX_GODOT`。操作已有项目的命令始终传入 `--project <dir>`。

## 常用工作流

接入或更新已有项目：

```powershell
gdx --project C:\Path\To\GodotProject project update --check
gdx --project C:\Path\To\GodotProject project update
gdx --project C:\Path\To\GodotProject project inspect
```

构建与验证：

```powershell
gdx --project .\demo asset import
gdx --project .\demo script check-all
gdx --project .\demo test run --path res://tests/smoke_test.gd --method run_tests
```

只有业务工作流确认导入资产已变化时才运行 `asset import`；daemon 不猜测业务层资产状态。

运行时编辑与截图：

```powershell
gdx --project .\demo daemon start --restart --width 1280 --height 720
gdx --project .\demo scene tree
gdx --project .\demo node create --parent / --type Label --name Title
gdx --project .\demo node set --node /Title --property text --value "Hello from gdx"
gdx --project .\demo scene save
gdx --project .\demo capture daemon --out .\artifacts\capture.png
gdx --project .\demo daemon stop
```

录制与 verify：

```powershell
gdx --project .\demo capture record --out .\artifacts\recording.avi --duration 3 --fps 60
gdx --project .\demo verify --spec .\verify.json
```

```json
{
  "checks": { "script": { "root": "res://", "strict": true } },
  "tests": [{ "path": "res://tests/smoke_test.gd", "method": "run_tests" }],
  "daemon": { "width": 390, "height": 844, "restart": true, "stop": true },
  "steps": [
    { "call": { "target": "/", "method": "gdx_start_run", "args": [] } },
    { "state": { "target": "/", "method": "gdx_state" } },
    { "capture": { "out": "artifacts/capture.png", "frames": 10 } }
  ]
}
```

## 定位与边界

- gdx 负责通用 Godot 工具能力和脚手架，不实现具体游戏业务。
- `scene build` 接受 Godot 专用 JSON spec，spec 由调用方生成。
- daemon 绑定 `127.0.0.1`，用于可信本地自动化；临时 session 和日志不写入项目。
- 视觉 baseline、比较指标、阈值、mask、玩法断言和产品验收规则属于游戏项目。
- `capture record` 启动新场景，不录制当前 daemon session。
- Touch 手势要求 daemon runtime 支持 `touch_sequence`，不会降级为鼠标事件。
- Export 需要 Godot export preset 和 export templates。

## 文档

- [快速开始](docs/zh-CN/quickstart.md)
- [Agent 使用指南](docs/zh-CN/agent-usage.md)
- [CLI 参考](docs/zh-CN/cli-reference.md)
- [故障排查](docs/zh-CN/troubleshooting.md)
- [开发 gdx](docs/zh-CN/developing.md)

内置 Codex skill 位于 [`skills/gdx-game-dev`](skills/gdx-game-dev/SKILL.md)。

## License

- [MIT License](LICENSE-MIT)
- [Apache License, Version 2.0](LICENSE-APACHE)
