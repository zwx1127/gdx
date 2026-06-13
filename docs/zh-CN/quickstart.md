# gdx 快速开始

本指南会让本地 `gdx` 二进制连接到 Godot 4.x，并创建一个最小 Godot 项目。

## 环境要求

- Rust stable。
- Godot 4.x。
- Windows 上运行仓库 E2E 脚本需要 PowerShell。

## 构建

```powershell
cargo build --workspace
```

Windows 上的本地二进制是 `target\debug\gdx.exe`。

## 定位 Godot

运行：

```powershell
target\debug\gdx.exe doctor
```

如果 Godot 不在 `PATH`，设置 `GDX_GODOT` 或传入 `--godot`：

```powershell
$env:GDX_GODOT = "C:\Path\To\Godot_v4.x.exe"
target\debug\gdx.exe --godot $env:GDX_GODOT doctor
```

## 创建项目

```powershell
target\debug\gdx.exe project create --path .\demo --name Demo
target\debug\gdx.exe --project .\demo scene create --out res://scenes/main.tscn --root-type Node2D --name Main --set-main
target\debug\gdx.exe --project .\demo project inspect
```

`--root-type` 和节点类型使用 Godot 类名，例如 `Node2D`、`Control`、`Label`、`Node3D`、`MeshInstance3D` 和 `Camera3D`。

## 接入已有项目

```powershell
gdx --project C:\Path\To\Game project install
gdx --project C:\Path\To\Game project inspect
```

`project install` 会复制 scene automation 和 daemon 工作流需要的 `addons/gdx_*` runtime 文件。它不会重写你的游戏架构。

升级或重新构建 `gdx` 后，用下面的命令刷新已接入项目：

```powershell
gdx --project C:\Path\To\Game project update
```

## 验证闭环

```powershell
gdx --project .\demo asset import
gdx --project .\demo script check-all
gdx --project .\demo capture run --scene res://scenes/main.tscn --out .\demo\.gdx\capture.png
```

如果项目已经配置 main scene，`capture run` 可以省略 `--scene`。

## 下一步

- 阅读 [CLI 参考](cli-reference.md) 查看命令语法。
- 用 Codex 或其他自动化 agent 驱动 `gdx` 时阅读 [Agent 使用指南](agent-usage.md)。
- Godot、脚本检查、daemon 或截图失败时阅读 [故障排查](troubleshooting.md)。
