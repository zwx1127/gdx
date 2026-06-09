# 开发 gdx

`gdx` 是一个 Rust workspace，包含一个 CLI crate 和随包分发的 Godot runtime 资源。

## 仓库结构

- `crates/gdx-cli/src/`：CLI 命令、上下文、Godot 进程处理、daemon RPC、诊断和 JSON 输出。
- `crates/gdx-cli/resources/addons/`：`project install` 安装到目标项目的 Godot 脚本。
- `docs/`：用户、agent 和开发者文档。
- `skills/gdx-game-dev/`：Codex skill 和紧凑命令参考。
- `tests/e2e/`：PowerShell 脚本，用临时 Godot 项目验证真实工作流。

## 构建和测试

```powershell
cargo fmt --check
cargo test --workspace
cargo build --workspace
```

当 Godot 可用时：

```powershell
$env:GDX_GODOT = "C:\Path\To\Godot_v4.x.exe"
target\debug\gdx.exe doctor
.\tests\e2e\hello_world.ps1 -Godot $env:GDX_GODOT
```

运行最贴近改动的 E2E 脚本。触及 daemon、scene edit、UI click、3D 或 export 共享行为时扩大验证范围。

## 新增或修改命令

保持 CLI 契约稳定：

- 成功输出是 stdout 上的 JSON。
- 失败输出是 stderr 上的 JSON。
- 错误码应稳定、精确，并对自动化有用。
- 当用户或 agent 有明确下一步时，提供 `suggestion`。
- 启动过 Godot 进程的失败应包含 Godot stdout/stderr 日志产物路径。

同步更新：

- 用户可见工作流：`README.md` 和 `README.zh-CN.md`。
- 命令语法：`docs/en/cli-reference.md` 和 `docs/zh-CN/cli-reference.md`。
- 自动化行为：`docs/en/agent-usage.md` 和 `docs/zh-CN/agent-usage.md`。
- Agent-facing 行为：`skills/gdx-game-dev/`。

## Godot runtime 资源

`crates/gdx-cli/resources/addons/` 下的 runtime 文件会被复制到目标项目。把它们视作公开自动化契约的一部分。改动这些文件时，应尽量用真实 Godot 运行验证。

## 文档风格

- README 保持为项目入口，不承载过长命令细节。
- 长命令说明放在 `docs/`。
- 中英文文档结构保持对齐。
- Windows E2E 路径使用 PowerShell 示例，并保持命令可复制。
