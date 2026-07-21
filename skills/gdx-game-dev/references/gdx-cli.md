# gdx CLI Reference

All final `gdx` results are JSON. Stdout contains one success object. Progress is written to stderr; in non-TTY use it is line-delimited NDJSON, and a failure object is the final stderr line.

For project-facing documentation, see `../../../docs/en/cli-reference.md` or `../../../docs/zh-CN/cli-reference.md`. This reference is the compact agent-facing version.

## Global Options

```powershell
gdx --project <project-dir> --progress auto --warnings summary <command>
```

- `--godot <path>`: override Godot binary discovery.
- `--godot-standard` / `GDX_GODOT_STANDARD`: configure standard Godot.
- `--godot-dotnet` / `GDX_GODOT_DOTNET`: configure .NET Godot.
- `--project <dir>`: target an existing Godot project.
- `GDX_GODOT`: environment variable alternative to `--godot`.
- `--progress auto|stderr|ndjson|off`: progress format; auto uses NDJSON when stderr is not a TTY.
- `--warnings all|summary|error`: returned diagnostic detail; complete logs remain on disk.

Selection order is explicit `--godot`, project-matching typed configuration, `GDX_GODOT`, then `PATH`.

Use `--project` for every command that operates on a project, including scene, script, asset, daemon, input, state, capture, resource, test, and export commands.

## Environment and Projects

```powershell
gdx doctor
gdx project create --path .\demo --name Demo
gdx --project .\demo project install
gdx --project .\demo project update
gdx --project .\demo project update --check
gdx --project .\demo project update --force
gdx --project .\demo project inspect
```

`project install` installs the `addons/gdx_*` runtime files required by scene automation and daemon workflows.
`project update` refreshes those managed addon files from the current CLI bundle. Use `--check` to report status without writing, and `--force` to rewrite all managed addon files.

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
gdx --project .\demo script check-all --jobs 4 --script-timeout-secs 60 --timeout-secs 300 --exclude res://vendor
gdx --project .\demo script load-check

gdx --project .\demo scene create --out res://scenes/main.tscn --root-type Node2D --name Main --set-main
gdx --project .\demo scene build --spec .\scene_spec.json
gdx --project .\demo resource create --type StandardMaterial3D --out res://materials/basic.tres
gdx --project .\demo resource inspect --path res://materials/basic.tres
```

`script create` writes only a minimal script header. For real gameplay, edit the `.gd` file directly after creating or use normal file edits first and then run `script check-all`.

`script check-all` is strict and runs Godot parser checks over `.gd` files with stable result ordering. Jobs default to at most four. Repeated `--exclude` values prune explicit subtrees; managed `addons/gdx_*` scripts stay in scope. `script load-check` is the older fast resource-load check.

## Run History

```powershell
gdx --project .\demo runs list
gdx --project .\demo runs prune
gdx --project .\demo runs prune --apply
```

Every one-shot Godot child has `.gdx/runs/<run-id>/run.json` and complete logs. Prune is preview-only without `--apply`; legacy directories are enrolled only after the first applied prune.

## Daemon and Runtime Commands

```powershell
gdx --project .\demo daemon start --restart --width 1280 --height 720 --health-timeout-secs 5
gdx --project .\demo daemon status
gdx --project .\demo scene tree
gdx --project .\demo scene tree --include-script --include-groups --include-methods
gdx --project .\demo node create --parent / --type Label --name Status
gdx --project .\demo node set --node /Status --property text --value "Ready"
gdx --project .\demo node set --node /Status --property metadata --value-file .\value.json
gdx --project .\demo node set --node /Status --property position --vec2 40 40
gdx --project .\demo scene save
gdx --project .\demo input send --mouse-button 1 --position 120 240
gdx --project .\demo input send --mouse-button 1 --position 120 240 --pressed false
gdx --project .\demo input send --mouse-button 1 --position 120 240 --release
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
gdx --project .\demo call invoke --target / --method start_game --args-file .\args.json
gdx --project .\demo state get --target /
gdx --project .\demo state get --target / --method gdx_state
gdx --project .\demo capture daemon --out .\demo\.gdx\capture.png
gdx --project .\demo capture record --out .\demo\.gdx\recording.avi --duration 3 --fps 60
gdx --project .\demo capture record --out .\demo\.gdx\gesture.avi --duration 3 --fps 60 --input-sequence .\demo\.gdx\touch-sequence.json
gdx --project .\demo daemon stop
```

`daemon start` uses the configured main scene unless `--scene res://...` is supplied. Its `health` validates capabilities, the scene root, and hard startup errors. `project_unhealthy` stops a newly started daemon automatically. `--no-health-check` returns `project_healthy: null`. Runtime capability fields remain available; update the managed runtime when `runtime_status` is `unknown` or `outdated`.

`input send --pressed` accepts `true` or `false`; `--release` is a clear alias for `--pressed false`. `input click` uses mouse events. For mobile gameplay that listens to `InputEventScreenTouch` or `InputEventScreenDrag`, use `input tap`, `input drag`, `input swipe`, `input pinch`, or `input sequence`. Touch commands require `touch_sequence` and do not fall back to mouse events.

`input sequence` reads `{ "events": [...] }` JSON. Events are `{ "kind": "touch", "index": 0, "position": [120, 240], "pressed": true }`, `{ "kind": "drag", "index": 0, "position": [160, 260], "relative": [40, 20] }`, or `{ "kind": "wait", "frames": 2 }`.

`state get --target /` defaults to `gdx_state()` when no method or property is supplied, and the result reports whether it read a method or property.

Use `scene tree --include-methods` to find `gdx_*` automation entrypoints. Treat method candidates as diagnostics only; choose the target explicitly.

Use `capture record` when animation timing needs review. It launches a fresh scene through Godot Movie Writer and writes an AVI file; it does not record the current daemon session. Add `--input-sequence <json>` to replay a touch sequence in that fresh scene while recording.

## Verify Specs

```powershell
gdx --project .\demo verify --spec .\demo\.gdx\verify.json
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

## Capture, Tests, and Export

```powershell
gdx --project .\demo capture run --scene res://scenes/main.tscn --out .\demo\.gdx\capture.png
gdx --project .\demo capture record --scene res://scenes/main.tscn --out .\demo\.gdx\recording.avi --duration 3 --fps 60
gdx --project .\demo capture record --scene res://scenes/main.tscn --out .\demo\.gdx\gesture.avi --duration 3 --fps 60 --input-sequence .\demo\.gdx\touch-sequence.json
gdx --project .\demo test run --path res://tests/smoke_test.gd --method run_tests
gdx --project .\demo export build --preset "Windows Desktop" --out .\demo\export\game.exe
```

`capture run` starts a one-shot screenshot runner. `capture daemon` captures the current daemon session. `capture record` records a freshly launched scene to AVI through Godot Movie Writer. `capture record --input-sequence <json>` replays touch events in that fresh scene. Export requires `export_presets.cfg` and installed Godot export templates.

## Failure Handling

When a command fails:

1. Parse stderr line by line; progress records have `type: "progress"` and the final failure has `ok: false`.
2. Read `suggestion` if present.
3. Read `details.diagnostics.primary_error` and the included log tails.
4. Open `artifacts.stderr_log` if more context is needed.
5. Fix project files or command arguments.
6. Re-run the narrowest failing command before continuing.

Do not infer success from process output text. Use the JSON `ok` field and expected artifacts such as created scenes or non-empty screenshots.

If `error` is `godot_native_crash`, inspect the Godot logs and local Godot/runtime environment. gdx does not automatically switch Godot binaries. If `error` is `daemon_runtime_outdated`, run `project update --check`, then `project update`, then restart the daemon.
