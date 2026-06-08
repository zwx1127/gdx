# AI Usage

gdx is intended to be driven by agents through subprocess calls. Every command emits JSON. Failures are emitted to stderr as JSON and include log artifacts when a Godot process was started.

Codex is the game developer: it analyzes requirements, writes Godot scripts, creates scene/resource specs, chooses architecture, and interprets failures. gdx is the Godot wrapper: it applies project settings, installs runtime files, copies/imports assets, creates/checks scripts, builds scenes/resources, controls runtime sessions, captures screenshots, and runs Godot tests. Do not add source-project-specific migration logic to gdx.

Use `--project <dir>` as a global option for every command that operates on an existing Godot project.

## Attach to a Project

```powershell
gdx --project <project> project install
gdx --project <project> project inspect
```

`project install` installs gdx runtime files under `addons/gdx_*`. `project inspect` returns the project name, configured main scene, gdx installation status, and categorized project files.

## Create a Project and Scene

```powershell
gdx project create --path <project> --name Demo
gdx --project <project> scene create --out res://scenes/main.tscn --root-type Node2D --name Main --set-main
```

Use Godot class names for `--root-type` and `node create --type`. gdx validates them inside Godot.

## Project Settings, Autoloads, and Input Maps

```powershell
gdx --project <project> setting set --section application --key run/main_scene --value res://scenes/main.tscn
gdx --project <project> setting list --section application
gdx --project <project> autoload add --name GameState --path res://scripts/game_state.gd --global
gdx --project <project> autoload list
gdx --project <project> input-map add --action ui_accept --keycode 32
```

## Assets, Scripts, Scenes, and Resources

```powershell
gdx --project <project> asset copy --from C:\Assets\icon.png --to res://assets/icon.png
gdx --project <project> asset import
gdx --project <project> asset inspect --path res://assets/icon.png
gdx --project <project> script create --path res://scripts/main.gd --extends Node2D
gdx --project <project> script attach --scene res://scenes/main.tscn --node / --script res://scripts/main.gd
gdx --project <project> script check-all
gdx --project <project> scene build --spec .\scene_spec.json
gdx --project <project> resource create --type StandardMaterial3D --out res://materials/basic.tres
gdx --project <project> resource inspect --path res://materials/basic.tres
```

`scene build` consumes a Godot-only JSON spec. The spec should describe nodes, properties, scripts, resources, groups, and children. Codex is responsible for generating the spec from its own design decisions.

## Edit and Verify

```powershell
gdx --project <project> daemon start
gdx --project <project> scene tree
gdx --project <project> node create --parent / --type Label --name Status
gdx --project <project> node set --node /Status --property text --value "Ready"
gdx --project <project> node set --node /Status --property position --vec2 40 40
gdx --project <project> scene save
gdx --project <project> capture daemon --out <project>\.gdx\capture.png
gdx --project <project> input send --mouse-button 1 --position 120 240
gdx --project <project> call invoke --target / --method start_game --args-json "[]"
gdx --project <project> state get --target / --method gdx_state
gdx --project <project> daemon stop
```

`daemon start` uses the project's main scene unless `--scene res://...` is provided. The daemon listens only on `127.0.0.1` and uses a per-session token stored in `.gdx/daemon/session.json`.

## Capture Without a Daemon

```powershell
gdx --project <project> capture run --scene res://scenes/main.tscn --out <project>\.gdx\capture.png
```

When `--scene` is omitted, `capture run` uses the configured main scene.

## Godot Tests

```powershell
gdx --project <project> test run --path res://tests/smoke_test.gd --method run_tests
```

The test script should expose the requested method and return JSON-compatible data.
