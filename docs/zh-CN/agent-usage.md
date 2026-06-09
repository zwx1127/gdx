# Agent 使用指南

`gdx` 面向 subprocess 自动化。Agent 应把它当作 Godot 自动化层，而不是游戏开发判断的替代品。

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
- Daemon 输入、方法调用、状态读取和截图。
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
```

构建并验证：

```powershell
gdx --project .\demo asset import
gdx --project .\demo script check-all
gdx --project .\demo test run --path res://tests/smoke_test.gd --method run_tests
gdx --project .\demo verify --spec .\demo\.gdx\verify.json
```

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
gdx --project .\demo state get --target / --method gdx_state
```

UI 流程优先使用 `input click-node`、`input activate` 和通过 `call invoke` 调用的项目级方法。只有坐标本身是测试对象时才使用坐标点击。

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

每个 step 必须只包含一个 action。支持的 step key 是 `call`、`state`、`capture`、`input_click_node` 和 `input_activate`。

## GDScript 注意事项

从 `Dictionary` 或 `Variant` 读取值时，除非类型显式，否则避免使用 `:=`。Godot 可能在运行时加载场景时把类型推断 warning 当作 parse error：

```gdscript
var content_bottom = layout.height - layout.safe_bottom
```
