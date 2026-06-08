# gdx

gdx is a Rust CLI that helps AI agents operate real Godot 4.x projects through official Godot command line workflows.

gdx is a Godot automation layer, not a game migration framework. Codex decides architecture, writes scripts, generates scene/resource specs, and interprets failures. gdx only wraps Godot capabilities: project settings, autoloads, input maps, assets, scripts, scenes, resources, runtime daemon control, tests, screenshots, and exports. gdx does not modify Godot engine source, call LLM/VLM APIs, translate source projects, or expose a network API beyond its local per-project daemon.

## Build

```powershell
cargo build --workspace
```

## Environment

```powershell
gdx doctor --json
```

If Godot is not on `PATH`, pass `--godot` or set `GDX_GODOT`:

```powershell
$env:GDX_GODOT = "C:\Path\To\Godot_v4.x.exe"
gdx doctor --json
```

## New Project Workflow

```powershell
gdx project init --project .\demo --name Demo --json
gdx project setting set --project .\demo --section application --key run/main_scene --value res://scenes/main.tscn --json
gdx scene create --project .\demo --out res://scenes/main.tscn --root-type Node2D --name Main --set-main --json
gdx asset import --project .\demo --json
gdx daemon start --project .\demo --json
gdx scene node add --project .\demo --parent / --type Label --name Title --json
gdx scene node set-property --project .\demo --node /Title --property text --value "Hello from gdx" --json
gdx scene node set-property --project .\demo --node /Title --property position --vec2 40 40 --json
gdx scene save --project .\demo --json
gdx daemon capture --project .\demo --out .\demo\shot.png --json
gdx daemon stop --project .\demo --json
```

## Godot Automation Workflow

Codex can generate Godot scripts, specs, and resource definitions, then ask gdx to apply them:

```powershell
gdx script create --project .\demo --path res://scripts/game_state.gd --class-name GameState --extends Node --json
gdx project autoload add --project .\demo --name GameState --path res://scripts/game_state.gd --global --json
gdx project input add --project .\demo --action ui_accept --keycode 32 --json
gdx asset copy --project .\demo --from C:\Assets\player.png --to res://assets/player.png --json
gdx asset import --project .\demo --json
gdx asset inspect --project .\demo --path res://assets/player.png --json
gdx scene build --project .\demo --spec .\main_scene_spec.json --json
gdx script attach --project .\demo --scene res://scenes/main.tscn --node / --script res://scripts/main.gd --json
gdx script check-all --project .\demo --json
gdx resource create --project .\demo --type StandardMaterial3D --out res://materials/basic.tres --json
gdx test run --project .\demo --path res://tests/smoke_test.gd --json
```

At runtime:

```powershell
gdx daemon start --project .\demo --json
gdx daemon input --project .\demo --mouse-button 1 --position 120 240 --json
gdx daemon call --project .\demo --target / --method start_game --args-json "[]" --json
gdx daemon state --project .\demo --target / --method gdx_state --json
gdx daemon capture --project .\demo --out .\demo\shot.png --json
gdx daemon stop --project .\demo --json
```

## Existing Project Workflow

```powershell
gdx project install --project C:\Path\To\GodotProject --json
gdx project inspect --project C:\Path\To\GodotProject --json
gdx daemon start --project C:\Path\To\GodotProject --json
gdx scene tree --project C:\Path\To\GodotProject --json
gdx daemon capture --project C:\Path\To\GodotProject --out C:\Path\To\GodotProject\.gdx\shot.png --json
```

`daemon start` and `run capture` use the project's configured main scene when `--scene` is omitted. If the project has no main scene, create one with `scene create --set-main` or pass `--scene res://...`.

## Tests

The Windows E2E scripts require a Godot 4.x executable:

```powershell
.\tests\e2e\hello_world.ps1 -Godot "C:\Path\To\Godot_v4.x.exe"
```

The scripts create temporary projects, exercise the public Agent workflow, and verify that screenshots are created.
