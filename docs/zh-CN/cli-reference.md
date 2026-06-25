# gdx CLI 参考

所有 `gdx` 命令输出都是 JSON。成功写到 stdout，失败以 JSON 写到 stderr，并可能包含 Godot 日志产物和诊断信息。

## 全局选项

```powershell
gdx --godot <path-to-godot> --project <project-dir> <command>
```

- `--godot <path>` 覆盖 Godot 二进制发现结果。
- `--project <dir>` 指定已有 Godot 项目。
- `GDX_GODOT` 是 `--godot` 的环境变量替代方式。

凡是操作项目的命令都使用 `--project`。

## 环境和项目

```powershell
gdx doctor
gdx project create --path .\demo --name Demo
gdx --project .\demo project install
gdx --project .\demo project update
gdx --project .\demo project update --check
gdx --project .\demo project update --force
gdx --project .\demo project inspect
```

`project install` 会把 runtime 文件安装到 `addons/gdx_*`。
`project update` 会用当前 CLI 内置内容刷新这些 managed addon 文件。使用 `--check` 只报告状态不写入，使用 `--force` 重写全部 managed addon 文件。

## 设置、autoload 和输入

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

`--keycode` 使用 Godot 接受的 keycode 整数。

## 资源、脚本、场景和 resource

```powershell
gdx --project .\demo asset copy --from C:\Assets\player.png --to res://assets/player.png --force
gdx --project .\demo asset import
gdx --project .\demo asset inspect --path res://assets/player.png

gdx --project .\demo script create --path res://scripts/main.gd --extends Node2D
gdx --project .\demo script attach --scene res://scenes/main.tscn --node / --script res://scripts/main.gd
gdx --project .\demo script check res://scripts/main.gd
gdx --project .\demo script check-all
gdx --project .\demo script load-check

gdx --project .\demo scene create --out res://scenes/main.tscn --root-type Node2D --name Main --set-main
gdx --project .\demo scene build --spec .\scene_spec.json
gdx --project .\demo scene tree
gdx --project .\demo scene save

gdx --project .\demo resource create --type StandardMaterial3D --out res://materials/basic.tres
gdx --project .\demo resource inspect --path res://materials/basic.tres
```

`script check-all` 会对 `.gd` 文件运行严格 Godot parser 检查。只有明确需要旧的快速 resource-load 检查时才使用 `script load-check`。

## Daemon 和运行时命令

```powershell
gdx --project .\demo daemon start --restart --width 1280 --height 720
gdx --project .\demo daemon status
gdx --project .\demo scene tree
gdx --project .\demo scene tree --include-script --include-groups --include-methods
gdx --project .\demo node create --parent / --type Label --name Status
gdx --project .\demo node set --node /Status --property text --value "Ready"
gdx --project .\demo node set --node /Status --property position --vec2 40 40
gdx --project .\demo scene save
gdx --project .\demo input send --mouse-button 1 --position 120 240
gdx --project .\demo input click --position 120 240
gdx --project .\demo input click-node --target /StartButton
gdx --project .\demo input touch --position 120 240 --pressed true
gdx --project .\demo input tap --position 120 240
gdx --project .\demo input drag --from 120 240 --to 220 260
gdx --project .\demo input swipe --from 120 240 --to 220 240
gdx --project .\demo input pinch --center 180 240 --start-distance 120 --end-distance 40
gdx --project .\demo input sequence --spec .\demo\.gdx\touch-sequence.json
gdx --project .\demo input activate --target /StartButton
gdx --project .\demo call invoke --target / --method start_game --args-json "[]"
gdx --project .\demo state get --target / --method gdx_state
gdx --project .\demo capture daemon --out .\demo\.gdx\capture.png
gdx --project .\demo daemon stop
```

`input click` 使用鼠标事件。移动端玩法如果监听 `InputEventScreenTouch` 或 `InputEventScreenDrag`，使用 `input tap`、`input drag`、`input swipe`、`input pinch` 或 `input sequence`。

`input sequence` 读取 `{ "events": [...] }` JSON。事件可以是 `{ "kind": "touch", "index": 0, "position": [120, 240], "pressed": true }`、`{ "kind": "drag", "index": 0, "position": [160, 260], "relative": [40, 20] }` 或 `{ "kind": "wait", "frames": 2 }`。

未提供 `--scene res://...` 时，`daemon start` 使用项目配置的 main scene。`daemon start` 和 `daemon status` 会在已安装 runtime 支持时返回 daemon runtime capabilities；`status: "unknown"` 表示项目内 runtime 早于 capabilities RPC。

升级或重新构建 `gdx` 后，运行 `gdx --project .\demo project update` 并重启 daemon，让运行中的项目使用新的内置 runtime。

`scene tree --include-methods` 会列出匹配 `--method-prefix` 的可调用方法，默认前缀是 `gdx_`。这些字段只用于诊断；gdx 不会自动选择替代 target。

## Verify、截图、测试和导出

```powershell
gdx --project .\demo verify --spec .\demo\.gdx\verify.json
gdx --project .\demo capture run --scene res://scenes/main.tscn --out .\demo\.gdx\capture.png
gdx --project .\demo capture record --scene res://scenes/main.tscn --out .\demo\.gdx\recording.avi --duration 3 --fps 60
gdx --project .\demo test run --path res://tests/smoke_test.gd --method run_tests
gdx --project .\demo export build --preset "Windows Desktop" --out .\demo\export\game.exe
```

`capture run` 启动一次性截图 runner。`capture daemon` 截取当前 daemon session。`capture record` 使用 Godot Movie Writer 启动一个新的场景实例并写出 AVI，不会录制当前 daemon session。导出需要 `export_presets.cfg` 和已安装的 Godot export templates。
