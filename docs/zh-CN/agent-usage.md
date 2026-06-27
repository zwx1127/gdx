# Agent 使用指南

`gdx` 面向 subprocess 自动化。Agent 应把它当作 Godot 自动化层，而不是游戏开发判断的替代品。

`gdx` 也很适合配合 [`agent-relay`](https://github.com/zwx1127/agent-relay) 这类远程 agent 控制项目使用。在这种模式下，Codex 和 `gdx` 运行在安装了 Godot 的可信本地机器上；开发者通过 Telegram 或 Lark/飞书发送需求、回答问题、批准操作、接收截图，并可以在任何地方持续推进 Godot 游戏开发。

Agent 负责：

- 游戏设计和实现决策。
- GDScript 文件。
- 场景和资源 JSON specs。
- 资源放置。
- 测试方法和运行时状态方法。
- 解释失败并修复。

`gdx` 负责：

- Godot 二进制定位。
- 项目设置、autoload 和 input map。
- Runtime addon 安装。
- 资源导入。
- 场景和资源创建。
- Daemon 输入、方法调用、状态读取、截图和一次性场景录制。
- Godot 脚本检查、测试、验证和导出。

## 命令契约

所有成功输出都是 stdout 上的 JSON。所有失败输出都是 stderr 上的 JSON。

失败时：

1. 解析 stderr JSON。
2. 阅读 `error.code`、`message` 和 `suggestion`。
3. 如果存在，检查 `details.diagnostics.primary_error`。
4. 只有 JSON 摘要不够时，再打开 `artifacts.stdout_log` 或 `artifacts.stderr_log`。
5. 修复项目文件、spec、资源或命令参数。
6. 重新运行最窄的失败命令。

不要从自由格式 Godot 输出推断成功。使用 JSON `ok` 字段，以及创建的场景或非空截图等预期产物判断。

## 标准工作流

新项目：

```powershell
gdx doctor
gdx project create --path .\demo --name Demo
gdx --project .\demo scene create --out res://scenes/main.tscn --root-type Node2D --name Main --set-main
```

已有项目：

```powershell
gdx --project C:\Path\To\Game project install
gdx --project C:\Path\To\Game project inspect
gdx --project C:\Path\To\Game project update --check
```

构建并验证：

```powershell
gdx --project .\demo asset import
gdx --project .\demo script check-all
gdx --project .\demo test run --path res://tests/smoke_test.gd --method run_tests
gdx --project .\demo verify --spec .\demo\.gdx\verify.json
```

## 配合 agent-relay 的远程工作流

当工作站安装了 Godot，但开发者希望用手机或另一台机器远程操作时，使用这个模式：

1. 在 Godot 项目所在的可信机器上运行 [`agent-relay`](https://github.com/zwx1127/agent-relay)。
2. 在聊天工具里选择 Godot 项目 workspace。
3. 要求 Codex 使用 `gdx-game-dev` skill，并在本地驱动 `gdx` 命令。
4. 改动后让 Codex 运行 `script check-all`、`test run`、`verify`，或有针对性的 daemon/capture 命令。
5. 需要视觉审阅时，通过 relay 把截图、录制文件或生成产物发回聊天窗口。

这种模式会把 Godot 执行、daemon 访问、项目文件和导出产物留在本地。聊天消息、截图、日志和 agent 输出都应视为可能包含敏感项目数据。

agent-relay 项目链接：<https://github.com/zwx1127/agent-relay>

## 运行时状态

当测试需要检查行为时，在游戏脚本中暴露结构化状态：

```gdscript
func gdx_state() -> Dictionary:
    return {
        "score": score,
        "player_position": [player.position.x, player.position.y]
    }
```

然后查询：

```powershell
gdx --project .\demo state get --target /
gdx --project .\demo state get --target / --method gdx_state
```

UI 流程优先使用 `input click-node`、`input activate` 和通过 `call invoke` 调用的项目级方法。只有坐标本身是测试对象时才使用坐标点击。移动端玩法如果监听触摸事件，使用 `input tap`、`input drag`、`input swipe`、`input pinch` 或 `input sequence`，不要用鼠标点击替代。

`state get --target /` 在未传 `method` 或 `property` 时默认调用 `gdx_state()`，返回结果会标明 state 来自 method 还是 property。

Touch 命令要求 daemon runtime 提供 `touch_sequence`；如果报告 `daemon_runtime_outdated`，先运行 `project update --check`，再更新 managed runtime 并重启 daemon。不要把 touch 手势改写成鼠标事件来绕过旧 runtime。

需要审阅动画节奏时，使用 `capture record --out .gdx/recording.avi --duration 3 --fps 60`。增加 `--input-sequence <json>` 可以在录制时回放 touch 事件。它通过 Godot Movie Writer 录制重新启动的场景，不会录制当前 daemon session。

## Verify specs

多步骤回归检查使用 `verify`：

```json
{
  "checks": { "script": { "root": "res://", "strict": true } },
  "tests": [{ "path": "res://tests/smoke_test.gd", "method": "run_tests" }],
  "daemon": { "width": 390, "height": 844, "restart": true, "stop": true },
  "steps": [
    { "call": { "target": "/", "method": "gdx_start_run", "args": [] } },
    { "state": { "target": "/", "method": "gdx_state" } },
    { "capture": { "out": ".gdx/main.png", "frames": 10 } }
  ]
}
```

每个 step 必须只包含一个 action。支持的 step key 是 `call`、`state`、`capture`、`input_click_node`、`input_activate`、`input_tap`、`input_drag`、`input_swipe`、`input_pinch` 和 `input_touch_sequence`。

## GDScript 注意事项

从 `Dictionary` 或 `Variant` 读取值时，除非类型显式，否则避免使用 `:=`。Godot 可能在运行时加载场景时把类型推断 warning 当作 parse error：

```gdscript
var content_bottom = layout.height - layout.safe_bottom
```
