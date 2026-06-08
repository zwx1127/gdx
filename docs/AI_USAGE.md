# AI Usage

gdx is intended to be driven by agents through subprocess calls. Use `--json` for every command. Failures are emitted to stderr as JSON and include log artifacts when a Godot process was started.

Codex is the game developer: it analyzes requirements, writes Godot scripts, creates scene/resource specs, chooses architecture, and interprets failures. gdx is the Godot wrapper: it applies project settings, installs runtime files, copies/imports assets, creates/checks scripts, builds scenes/resources, controls runtime sessions, captures screenshots, and runs Godot tests. Do not add source-project-specific migration logic to gdx.

## Attach to a Project

For an existing Godot project:

```powershell
gdx project install --project <project> --json
gdx project inspect --project <project> --json
```

`project install` installs gdx runtime files under `addons/gdx_*`. `project inspect` returns the project name, configured main scene, gdx installation status, and categorized project files.

## Create a Scene

If the project has no main scene:

```powershell
gdx scene create --project <project> --out res://scenes/main.tscn --root-type Node2D --name Main --set-main --json
```

Use Godot class names for `--root-type` and `scene node add --type`. gdx validates them inside Godot.

## Project Settings, Autoloads, and Input

```powershell
gdx project setting set --project <project> --section application --key run/main_scene --value res://scenes/main.tscn --json
gdx project setting list --project <project> --section application --json
gdx project autoload add --project <project> --name GameState --path res://scripts/game_state.gd --global --json
gdx project autoload list --project <project> --json
gdx project input add --project <project> --action ui_accept --keycode 32 --json
```

## Assets, Scripts, Scenes, and Resources

```powershell
gdx asset copy --project <project> --from C:\Assets\icon.png --to res://assets/icon.png --json
gdx asset import --project <project> --json
gdx asset inspect --project <project> --path res://assets/icon.png --json
gdx script create --project <project> --path res://scripts/main.gd --extends Node2D --json
gdx script attach --project <project> --scene res://scenes/main.tscn --node / --script res://scripts/main.gd --json
gdx script check-all --project <project> --json
gdx scene build --project <project> --spec .\scene_spec.json --json
gdx resource create --project <project> --type StandardMaterial3D --out res://materials/basic.tres --json
gdx resource inspect --project <project> --path res://materials/basic.tres --json
```

`scene build` consumes a Godot-only JSON spec. The spec should describe nodes, properties, scripts, resources, groups, and children. Codex is responsible for generating the spec from its own design decisions.

## Edit and Verify

```powershell
gdx daemon start --project <project> --json
gdx scene tree --project <project> --json
gdx scene node add --project <project> --parent / --type Label --name Status --json
gdx scene node set-property --project <project> --node /Status --property text --value "Ready" --json
gdx scene node set-property --project <project> --node /Status --property position --vec2 40 40 --json
gdx scene save --project <project> --json
gdx daemon capture --project <project> --out <project>\.gdx\capture.png --json
gdx daemon input --project <project> --mouse-button 1 --position 120 240 --json
gdx daemon call --project <project> --target / --method start_game --args-json "[]" --json
gdx daemon state --project <project> --target / --method gdx_state --json
gdx daemon stop --project <project> --json
```

`daemon start` uses the project's main scene unless `--scene res://...` is provided. The daemon listens only on `127.0.0.1` and uses a per-session token stored in `.gdx/daemon/session.json`.

## Godot Tests

```powershell
gdx test run --project <project> --path res://tests/smoke_test.gd --method run_tests --json
```

The test script should expose the requested method and return JSON-compatible data.
