# gdx CLI Reference

All `gdx` command results are JSON. Stdout contains exactly one final success object. Progress and failures are written to stderr; in non-interactive use progress is line-delimited NDJSON and the final stderr line is the existing `{ "ok": false, ... }` error object.

## Global options

```powershell
gdx --project <project-dir> --progress auto --warnings summary <command>
```

- `--godot <path>` overrides Godot binary discovery.
- `--godot-standard <path>` / `GDX_GODOT_STANDARD` configure a standard Godot build.
- `--godot-dotnet <path>` / `GDX_GODOT_DOTNET` configure a .NET-enabled Godot build.
- `--project <dir>` targets an existing Godot project.
- `GDX_GODOT` is the environment variable alternative to `--godot`.
- `--progress auto|stderr|ndjson|off` defaults to human progress on a TTY and NDJSON otherwise. Long commands emit a heartbeat at least every five seconds.
- `--warnings all|summary|error` controls the diagnostic groups returned in the result; complete logs are always retained.

Godot selection is explicit `--godot`, then the project-matching standard/.NET setting, then `GDX_GODOT`, then `PATH`.

Use `--project` for commands that operate on a project.

## Environment and projects

```powershell
gdx doctor
gdx project create --path .\demo --name Demo
gdx --project .\demo project install
gdx --project .\demo project update
gdx --project .\demo project update --check
gdx --project .\demo project update --force
gdx --project .\demo project inspect
```

`project install` installs runtime files under `addons/gdx_*`.
`project update` refreshes those managed addon files from the current CLI bundle. Use `--check` to report status without writing, and `--force` to rewrite all managed addon files.

## Settings, autoloads, and inputs

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

Use keycode integers accepted by Godot.

## Assets, scripts, scenes, and resources

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
gdx --project .\demo scene tree
gdx --project .\demo scene save

gdx --project .\demo resource create --type StandardMaterial3D --out res://materials/basic.tres
gdx --project .\demo resource inspect --path res://materials/basic.tres
```

`script check-all` runs strict Godot parser checks over `.gd` files with a bounded worker pool. The default job count is `min(available_parallelism, 4)`. Repeated `--exclude res://...` values prune explicit subtrees, while managed `addons/gdx_*` scripts remain checked.

## Run records and retention

```powershell
gdx --project .\demo runs list --status all --limit 50
gdx --project .\demo runs prune
gdx --project .\demo runs prune --apply
```

Every one-shot Godot process writes `.gdx/runs/<run-id>/run.json` plus complete stdout/stderr logs. `runs prune` is preview-only unless `--apply` is passed. Defaults retain successful/unknown runs for 200 entries or 7 days and failed runs for 50 entries or 30 days; exceeding either limit makes a completed run eligible. Legacy run directories are only enrolled after the first explicit `runs prune --apply`. Running records and `.gdx/daemon` are never pruned.

## Daemon and runtime commands

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
gdx --project .\demo daemon stop
```

`daemon start` uses the configured main scene unless `--scene res://...` is supplied. After RPC readiness it checks `capabilities`, `scene_tree`, and startup logs. The returned `health` contains `rpc_ready`, `scene_loaded`, `project_healthy`, `scene_root`, the first hard error, and elapsed time. An unhealthy new daemon fails with `project_unhealthy`, is stopped, and has its session removed. `--no-health-check` skips project validation and returns `project_healthy: null`.

`daemon start` and `daemon status` preserve `runtime_status`, `runtime_version`, `protocol_version`, `methods`, and `warnings`. `runtime_status: "unknown"` means the project runtime predates the capabilities RPC; `runtime_status: "outdated"` means capabilities are available but a required method is missing. `daemon status` removes an unreachable stale session safely.

After rebuilding or upgrading `gdx`, run `gdx --project .\demo project update --check`, review the managed addon changes, then run `gdx --project .\demo project update` and restart the daemon so the running project uses the bundled runtime.

`input send --pressed` accepts `true` or `false`; `--release` is a clear alias for `--pressed false`. `input click` uses mouse events. For mobile gameplay that listens to `InputEventScreenTouch` or `InputEventScreenDrag`, use `input tap`, `input drag`, `input swipe`, `input pinch`, or `input sequence`. Touch commands require a daemon runtime with `touch_sequence`; they report `daemon_runtime_outdated` instead of falling back to mouse events.

`input sequence` reads `{ "events": [...] }` JSON. Events are `{ "kind": "touch", "index": 0, "position": [120, 240], "pressed": true }`, `{ "kind": "drag", "index": 0, "position": [160, 260], "relative": [40, 20] }`, or `{ "kind": "wait", "frames": 2 }`.

`state get --target /` omits `method` and `property`, so the runtime defaults to `gdx_state()` when the target implements it. The result includes `target`, `source`, `method` or `property`, and `state` so null state values still show what was queried.

`scene tree --include-methods` lists callable methods matching `--method-prefix`, which defaults to `gdx_`. These fields are diagnostic only; gdx does not auto-select alternate targets.

## Verify, capture, tests, and export

```powershell
gdx --project .\demo verify --spec .\demo\.gdx\verify.json
gdx --project .\demo capture run --scene res://scenes/main.tscn --out .\demo\.gdx\capture.png
gdx --project .\demo capture record --scene res://scenes/main.tscn --out .\demo\.gdx\recording.avi --duration 3 --fps 60
gdx --project .\demo capture record --scene res://scenes/main.tscn --out .\demo\.gdx\gesture.avi --duration 3 --fps 60 --input-sequence .\demo\.gdx\touch-sequence.json
gdx --project .\demo test run --path res://tests/smoke_test.gd --method run_tests
gdx --project .\demo export build --preset "Windows Desktop" --out .\demo\export\game.exe
```

`capture run` starts a one-shot screenshot runner. `capture daemon` captures the current daemon session. `capture record` starts a fresh scene with Godot Movie Writer and writes an AVI file; it does not record the current daemon session. `capture record --input-sequence <json>` replays touch events in that fresh scene before Movie Writer frames are emitted, which is useful for gesture animation review. Export requires `export_presets.cfg` and installed Godot export templates.
