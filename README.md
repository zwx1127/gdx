# gdx

gdx is a Rust CLI that helps AI agents operate real Godot 4.x projects through official Godot command line workflows.

gdx is a Godot automation layer, not a game migration framework. Codex decides architecture, writes scripts, generates scene/resource specs, and interprets failures. gdx only wraps Godot capabilities: project settings, autoloads, input maps, assets, scripts, scenes, resources, runtime daemon control, tests, screenshots, and exports.

All command output is JSON. Failures are emitted to stderr as JSON and include log artifacts and diagnostics when a Godot process was started.

## Build

```powershell
cargo build --workspace
```

## Environment

```powershell
gdx doctor
```

If Godot is not on `PATH`, pass `--godot` or set `GDX_GODOT`:

```powershell
$env:GDX_GODOT = "C:\Path\To\Godot_v4.x.exe"
gdx --godot $env:GDX_GODOT doctor
```

## New Project Workflow

```powershell
gdx project create --path .\demo --name Demo
gdx --project .\demo setting set --section application --key run/main_scene --value res://scenes/main.tscn
gdx --project .\demo scene create --out res://scenes/main.tscn --root-type Node2D --name Main --set-main
gdx --project .\demo asset import
gdx --project .\demo daemon start
gdx --project .\demo node create --parent / --type Label --name Title
gdx --project .\demo node set --node /Title --property text --value "Hello from gdx"
gdx --project .\demo node set --node /Title --property position --vec2 40 40
gdx --project .\demo scene save
gdx --project .\demo capture daemon --out .\demo\shot.png
gdx --project .\demo daemon stop
```

## Godot Automation Workflow

```powershell
gdx --project .\demo script create --path res://scripts/game_state.gd --class-name GameState --extends Node
gdx --project .\demo autoload add --name GameState --path res://scripts/game_state.gd --global
gdx --project .\demo input-map add --action ui_accept --keycode 32
gdx --project .\demo asset copy --from C:\Assets\player.png --to res://assets/player.png
gdx --project .\demo asset import
gdx --project .\demo asset inspect --path res://assets/player.png
gdx --project .\demo scene build --spec .\main_scene_spec.json
gdx --project .\demo script attach --scene res://scenes/main.tscn --node / --script res://scripts/main.gd
gdx --project .\demo script check-all
gdx --project .\demo script load-check
gdx --project .\demo resource create --type StandardMaterial3D --out res://materials/basic.tres
gdx --project .\demo test run --path res://tests/smoke_test.gd
```

`script check-all` is strict: it runs Godot's script parser for every `.gd` file and fails on parser errors and warnings that Godot treats as errors. `script load-check` keeps the older fast resource-load check.

At runtime:

```powershell
gdx --project .\demo daemon start
gdx --project .\demo input send --mouse-button 1 --position 120 240
gdx --project .\demo input click --position 120 240
gdx --project .\demo input click-node --target /StartButton
gdx --project .\demo input activate --target /StartButton
gdx --project .\demo call invoke --target / --method start_game --args-json "[]"
gdx --project .\demo state get --target / --method gdx_state
gdx --project .\demo capture daemon --out .\demo\shot.png
gdx --project .\demo daemon stop
```

For multi-step runtime verification, prefer a spec:

```powershell
gdx --project .\demo verify --spec .\verify.json
```

```json
{
  "checks": { "script": { "root": "res://", "strict": true } },
  "tests": [{ "path": "res://tests/smoke_test.gd", "method": "run_tests" }],
  "daemon": { "width": 390, "height": 844, "restart": true, "stop": true },
  "steps": [
    { "call": { "target": "/", "method": "start_game", "args": [] } },
    { "state": { "target": "/", "method": "gdx_state" } },
    { "capture": { "out": ".\\demo\\.gdx\\capture.png", "frames": 10 } }
  ]
}
```

## Existing Project Workflow

```powershell
gdx --project C:\Path\To\GodotProject project install
gdx --project C:\Path\To\GodotProject project inspect
gdx --project C:\Path\To\GodotProject daemon start
gdx --project C:\Path\To\GodotProject scene tree
gdx --project C:\Path\To\GodotProject capture daemon --out C:\Path\To\GodotProject\.gdx\shot.png
```

`daemon start` and `capture run` use the project's configured main scene when `--scene` is omitted. If the project has no main scene, create one with `scene create --set-main` or pass `--scene res://...`.

## Tests

The Windows E2E scripts require a Godot 4.x executable:

```powershell
.\tests\e2e\hello_world.ps1 -Godot "C:\Path\To\Godot_v4.x.exe"
```

The scripts create temporary projects, exercise the public agent workflow, and verify that screenshots are created.
