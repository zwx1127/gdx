# gdx CLI Reference

All `gdx` command output is JSON. Success is written to stdout. Failures are written to stderr as JSON and may include Godot log artifacts.

## Global Options

```powershell
gdx --godot <path-to-godot> --project <project-dir> <command>
```

- `--godot <path>`: override Godot binary discovery.
- `--project <dir>`: target an existing Godot project.
- `GDX_GODOT`: environment variable alternative to `--godot`.

Use `--project` for every command that operates on a project, including scene, script, asset, daemon, input, state, capture, resource, test, and export commands.

## Environment and Projects

```powershell
gdx doctor
gdx project create --path .\demo --name Demo
gdx --project .\demo project install
gdx --project .\demo project inspect
```

`project install` installs the `addons/gdx_*` runtime files required by scene automation and daemon workflows.

## Settings, Autoloads, and Inputs

```powershell
gdx --project .\demo setting set --section application --key run/main_scene --value res://scenes/main.tscn
gdx --project .\demo setting list --section application
gdx --project .\demo autoload add --name GameState --path res://scripts/game_state.gd --global
gdx --project .\demo autoload list
gdx --project .\demo input-map add --action jump --keycode 32
gdx --project .\demo input-map list
```

Use keycode integers accepted by Godot.

## Assets, Scripts, Scenes, and Resources

```powershell
gdx --project .\demo asset copy --from C:\Assets\player.png --to res://assets/player.png --force
gdx --project .\demo asset import
gdx --project .\demo asset inspect --path res://assets/player.png

gdx --project .\demo script create --path res://scripts/main.gd --extends Node2D
gdx --project .\demo script attach --scene res://scenes/main.tscn --node / --script res://scripts/main.gd
gdx --project .\demo script check res://scripts/main.gd
gdx --project .\demo script check-all

gdx --project .\demo scene create --out res://scenes/main.tscn --root-type Node2D --name Main --set-main
gdx --project .\demo scene build --spec .\scene_spec.json
gdx --project .\demo resource create --type StandardMaterial3D --out res://materials/basic.tres
gdx --project .\demo resource inspect --path res://materials/basic.tres
```

`script create` writes only a minimal script header. For real gameplay, edit the `.gd` file directly after creating or use normal file edits first and then run `script check-all`.

## Daemon and Runtime Commands

```powershell
gdx --project .\demo daemon start --restart --width 1280 --height 720
gdx --project .\demo daemon status
gdx --project .\demo scene tree
gdx --project .\demo node create --parent / --type Label --name Status
gdx --project .\demo node set --node /Status --property text --value "Ready"
gdx --project .\demo node set --node /Status --property position --vec2 40 40
gdx --project .\demo scene save
gdx --project .\demo input send --mouse-button 1 --position 120 240
gdx --project .\demo call invoke --target / --method start_game --args-json "[]"
gdx --project .\demo state get --target / --method gdx_state
gdx --project .\demo capture daemon --out .\demo\.gdx\capture.png
gdx --project .\demo daemon stop
```

`daemon start` uses the configured main scene unless `--scene res://...` is supplied.

## Capture, Tests, and Export

```powershell
gdx --project .\demo capture run --scene res://scenes/main.tscn --out .\demo\.gdx\capture.png
gdx --project .\demo test run --path res://tests/smoke_test.gd --method run_tests
gdx --project .\demo export build --preset "Windows Desktop" --out .\demo\export\game.exe
```

`capture run` starts a one-shot capture runner. `capture daemon` captures the current daemon session. Export requires `export_presets.cfg` and installed Godot export templates.

## Failure Handling

When a command fails:

1. Parse the stderr JSON.
2. Read `suggestion` if present.
3. Open `artifacts.stderr_log` and find the first Godot error.
4. Fix project files or command arguments.
5. Re-run the narrowest failing command before continuing.

Do not infer success from process output text. Use the JSON `ok` field and expected artifacts such as created scenes or non-empty screenshots.
