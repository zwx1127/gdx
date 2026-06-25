# gdx

中文 | [English](README.md)

`gdx` 是一个面向 Godot 4.x 项目的 Rust CLI，用于让脚本和 AI agent 自动化操作真实的 Godot 工程。它封装 Godot 官方命令行工作流，并把项目创建、场景编辑、运行时控制、截图、测试和导出暴露为 JSON 输出的命令。

`gdx` 不是游戏迁移框架，也不是游戏引擎抽象层。调用方，通常是 Codex 或其他编码 agent，仍然负责游戏设计、架构、GDScript、场景 spec、资源和失败分析。`gdx` 只提供围绕 Godot 的可靠自动化层。

当 `gdx` 配合 [`agent-relay`](https://github.com/zwx1127/agent-relay) 这类远程 agent 操作项目使用时，可以实现通过聊天工具随时随地开发 Godot 游戏。Godot、Codex 和项目文件仍然运行在可信本地机器上，你可以从 Telegram 或 Lark/飞书发送需求、批准操作、运行检查、查看截图，并持续调整开发方向。

## 能做什么

- 创建新的 Godot 项目并配置 main scene。
- 把 `gdx` runtime addons 安装到已有 Godot 项目。
- 把已安装项目内的 gdx addons 更新到当前 `gdx` CLI 内置 runtime。
- 设置项目配置、autoload 和 input map。
- 复制、导入和检查资源。
- 创建、挂载、解析检查和加载检查 GDScript。
- 直接创建场景，或从 JSON spec 构建场景。
- 启动本地 Godot daemon，用于实时场景编辑、输入、方法调用、状态读取和截图。
- 运行 Godot 测试脚本和多步骤 verify spec。
- 通过 Godot export preset 构建导出产物。
- 配合 [`agent-relay`](https://github.com/zwx1127/agent-relay)，从聊天工具远程操作本地 Codex 加 `gdx` 的开发闭环。

所有成功命令都会把 JSON 写到 stdout。失败会把 JSON 写到 stderr，并可能包含 Godot 日志、诊断信息和修复建议。

## 快速开始

构建 CLI：

```powershell
cargo build --workspace
```

检查 `gdx` 是否能找到 Godot：

```powershell
target\debug\gdx.exe doctor
```

如果 Godot 不在 `PATH`，可以显式传入路径，或设置 `GDX_GODOT`：

```powershell
$env:GDX_GODOT = "C:\Path\To\Godot_v4.x.exe"
target\debug\gdx.exe --godot $env:GDX_GODOT doctor
```

创建并检查一个最小项目：

```powershell
target\debug\gdx.exe project create --path .\demo --name Demo
target\debug\gdx.exe --project .\demo scene create --out res://scenes/main.tscn --root-type Node2D --name Main --set-main
target\debug\gdx.exe --project .\demo project inspect
```

凡是操作已有 Godot 项目的命令，都使用 `--project <dir>` 指定项目目录。

## 常用工作流

接入已有项目：

```powershell
gdx --project C:\Path\To\GodotProject project install
gdx --project C:\Path\To\GodotProject project inspect
```

升级或重新构建 `gdx` 后，更新已接入项目：

```powershell
gdx --project C:\Path\To\GodotProject project update
gdx --project C:\Path\To\GodotProject daemon start --restart
```

构建并验证项目文件：

```powershell
gdx --project .\demo asset import
gdx --project .\demo script check-all
gdx --project .\demo test run --path res://tests/smoke_test.gd --method run_tests
```

使用 daemon 做运行时编辑和截图：

```powershell
gdx --project .\demo daemon start --restart --width 1280 --height 720
gdx --project .\demo scene tree
gdx --project .\demo node create --parent / --type Label --name Title
gdx --project .\demo node set --node /Title --property text --value "Hello from gdx"
gdx --project .\demo node set --node /Title --property position --vec2 40 40
gdx --project .\demo scene save
gdx --project .\demo capture daemon --out .\demo\.gdx\capture.png
gdx --project .\demo daemon stop
```

录制一个从场景重新启动开始的短 AVI，用于检查动画节奏：

```powershell
gdx --project .\demo capture record --out .\demo\.gdx\recording.avi --duration 3 --fps 60
```

运行多步骤 verify spec：

```powershell
gdx --project .\demo verify --spec .\demo\.gdx\verify.json
```

```json
{
  "checks": { "script": { "root": "res://", "strict": true } },
  "tests": [{ "path": "res://tests/smoke_test.gd", "method": "run_tests" }],
  "daemon": { "width": 390, "height": 844, "restart": true, "stop": true },
  "steps": [
    { "call": { "target": "/", "method": "gdx_start_run", "args": [] } },
    { "state": { "target": "/", "method": "gdx_state" } },
    { "capture": { "out": ".gdx/capture.png", "frames": 10 } }
  ]
}
```

## 文档

- [快速开始](docs/zh-CN/quickstart.md)
- [Agent 使用指南](docs/zh-CN/agent-usage.md)
- [CLI 参考](docs/zh-CN/cli-reference.md)
- [故障排查](docs/zh-CN/troubleshooting.md)
- [开发 gdx](docs/zh-CN/developing.md)

内置 Codex skill 位于 [`skills/gdx-game-dev`](skills/gdx-game-dev/SKILL.md)。它面向需要通过 `gdx` 构建、修改、运行、测试、截图和导出 Godot 游戏的 agent。

远程开发时，把 Codex 和 `gdx` 运行在安装了 Godot 的本地机器上，再用 [`agent-relay`](https://github.com/zwx1127/agent-relay) 作为聊天控制面。这样文件访问、Godot 执行、daemon session 和导出仍留在本机，同时可以离开工作站继续开发和审阅进展。

agent-relay 项目链接：<https://github.com/zwx1127/agent-relay>

## 环境要求

- Rust stable，使用仓库中的 `rust-toolchain.toml`。
- Godot 4.x 可执行文件，可放在 `PATH`，通过 `GDX_GODOT` 指定，或用 `--godot` 传入。
- PowerShell，用于运行仓库内的 Windows E2E 脚本。
- 只有运行 `gdx export build` 时才需要 Godot export templates 和 `export_presets.cfg`。

## 项目状态

`gdx` 目前处于 pre-1.0 阶段。当前重点是为 Godot 4.x 项目提供可靠的本地自动化闭环：

- CLI：Rust 二进制 `gdx`。
- Runtime 集成：安装到 `addons/gdx_*` 的 Godot addons。
- 验证：脚本检查、Godot 测试、daemon 状态调用、输入和截图。
- 调用模型：本地脚本和 AI agent。

已知限制：

- CLI 不会自行设计或迁移游戏。
- `scene build` 接收 Godot 专用 JSON spec；调用方负责生成 spec。
- daemon 绑定到 `127.0.0.1`，用于可信本地自动化。
- 导出依赖 Godot export preset 和已安装的 export templates。

## 贡献与支持

- 提交 PR 前请阅读 [CONTRIBUTING.md](CONTRIBUTING.md)。
- 报告敏感问题前请阅读 [SECURITY.md](SECURITY.md)。
- 提交安装或运行问题前，请先查看 [故障排查](docs/zh-CN/troubleshooting.md)。
- 版本记录见 [CHANGELOG.md](CHANGELOG.md)。

## 许可证

`gdx` 使用双许可证授权，可任选其一：

- [MIT License](LICENSE-MIT)
- [Apache License, Version 2.0](LICENSE-APACHE)
