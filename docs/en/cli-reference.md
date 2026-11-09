# gdx CLI Reference

`gdx` is a stateless, composable Godot CLI. A successful command writes one compact JSON object to stdout. Progress and failures go to stderr; with `--progress ndjson`, the final stderr line on failure is the structured error object.

Redirect output with the shell when it must be retained:

```powershell
gdx --project .\demo --progress ndjson script check-all 1>result.json 2>progress.ndjson
```

By default, gdx does not persist command results, progress, or one-shot Godot logs in the project. Recording logs can be explicitly retained with `capture record --logs-dir`. Build provenance is reported by `doctor` only.

## Global options

```powershell
gdx --project <project-dir> --progress auto <command>
```

- `--godot <path>` overrides Godot binary discovery.
- `--godot-standard <path>` / `GDX_GODOT_STANDARD` configure a standard build.
- `--godot-dotnet <path>` / `GDX_GODOT_DOTNET` configure a .NET-enabled build.
- `--project <dir>` targets an existing Godot project.
- `GDX_GODOT` is the environment variable alternative to `--godot`.
- `--progress auto|stderr|ndjson|off` defaults to human progress on a TTY and NDJSON otherwise. Long commands emit heartbeats.

Godot selection is explicit `--godot`, then the project-matching standard/.NET setting, then `GDX_GODOT`, then `PATH`.

## Environment and projects

```powershell
gdx doctor
gdx project create --path .\demo --name Demo
gdx --project .\demo project update --check
gdx --project .\demo project update
gdx --project .\demo project update --force
gdx --project .\demo project inspect
```

`project create` accepts only a missing or empty directory. It creates the minimal project, installs managed `addons/gdx_*` files, and adds only `.godot/` to a new or existing `.gitignore`.

`project update` is the single install/update entry point for managed addon files. `--check` reports pending changes without writing; `--force` rewrites all managed files. Updating an existing project does not modify its `.gitignore`.

`project inspect` returns concise project identity, main-scene, language, and managed-addon status. `doctor` returns detailed executable, Godot, .NET, project-language, and managed-runtime diagnostics.

## Settings, autoloads, and input map

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

## Assets, scripts, scenes, and resources

```powershell
gdx --project .\demo asset import
gdx --project .\demo asset inspect --path res://assets/player.png

gdx --project .\demo script create --path res://scripts/main.gd --extends Node2D
gdx --project .\demo script attach --scene res://scenes/main.tscn --node / --script res://scripts/main.gd
gdx --project .\demo script check res://scripts/main.gd
gdx --project .\demo script check-all --jobs 4 --script-timeout-secs 60 --timeout-secs 300 --exclude res://vendor
gdx --project .\demo script load-check

gdx --project .\demo scene create --out res://scenes/main.tscn --root-type Node2D --name Main --set-main
gdx --project .\demo scene build --spec .\scene-spec.json
gdx --project .\demo scene tree
gdx --project .\demo scene save

gdx --project .\demo resource create --type StandardMaterial3D --out res://materials/basic.tres
gdx --project .\demo resource inspect --path res://materials/basic.tres
```

Copy source assets with the shell or build system, then run `asset import` when the project workflow determines that imports are needed. `script check-all` uses bounded parallelism and returns aggregate counts rather than an expanded per-file success list.

## Daemon and runtime commands

```powershell
gdx --project .\demo daemon start --restart --width 1280 --height 720
gdx --project .\demo daemon status
gdx --project .\demo scene tree --include-script --include-groups --include-methods
gdx --project .\demo node create --parent / --type Label --name Status
gdx --project .\demo node set --node /Status --property text --value "Ready"
gdx --project .\demo scene save
gdx --project .\demo input send --mouse-button 1 --position 120 240 --pressed false
gdx --project .\demo input click --position 120 240
gdx --project .\demo input click-node --target /StartButton
gdx --project .\demo input touch --position 120 240 --pressed true
gdx --project .\demo input tap --position 120 240
gdx --project .\demo input drag --from 120 240 --to 220 260
gdx --project .\demo input pinch --center 180 240 --start-distance 120 --end-distance 40
gdx --project .\demo input sequence --spec .\touch-sequence.json
gdx --project .\demo input activate --target /StartButton
gdx --project .\demo call invoke --target / --method start_game --args-json "[]"
Get-Content -Raw .\args.json | gdx --project .\demo call invoke --target / --method start_game --args-json -
gdx --project .\demo call invoke --target / --method show_area --arg "river bridge"
gdx --project .\demo state get --target /
gdx --project .\demo capture daemon --out .\artifacts\capture.png
gdx --project .\demo daemon stop
```

`daemon start` validates managed runtime compatibility before spawning Godot. Its health check verifies RPC readiness, scene loading, and hard startup errors. A newly created unhealthy daemon is stopped automatically; an already-running unhealthy daemon is reported but left running for the developer to inspect or stop.

Active daemon session metadata and current logs live in a per-user OS temporary directory keyed by canonical project path. `daemon stop`, stale-session detection, and failed startup remove that state. `daemon start` exposes concise `runtime` and `health` summaries; `daemon status` also exposes `script_sync`. The token and full capability list are not returned.

The daemon hashes active `.gd` files at startup. A later add, modification, or removal makes ordinary RPC commands fail with `daemon_scripts_stale`; status remains available and reports `fresh`, `stale`, or `unknown`. gdx never auto-restarts because that would silently discard scene state. Use `daemon start --restart`, then recreate the state needed for verification or capture. Direct RPC results containing INF, -INF, or NaN fail with `daemon_non_finite_response`; `details.non_finite_values` identifies each value by RFC 6901 path and kind.

Touch commands require `touch_sequence` support and do not fall back to mouse input. Use `input drag` for drag or swipe gestures. `input send --pressed false` sends a release. Repeated `--arg` values are strings; use `--args-json <array>` for typed values, or `--args-json -` to read the array from stdin. JSON errors report origin and position without copying input content into the error.

## Verify, capture, tests, and export

```powershell
gdx --project .\demo verify --spec .\verify.json
gdx --project .\demo capture run --scene res://scenes/main.tscn --out .\artifacts\capture.png
gdx --project .\demo capture record --scene res://scenes/main.tscn --out .\artifacts\recording.avi --duration 3 --fps 60
gdx --project .\demo test run --path res://tests/smoke_test.gd --method run_tests
gdx --project .\demo export build --preset "Windows Desktop" --out .\export\game.exe
```

`verify` is a generic declarative command runner for script checks, Godot tests, daemon calls, state reads, input, and capture. Its result contains compact `steps` summaries rather than copies of each nested command envelope. Product assertions, visual baselines, comparison metrics, thresholds, masks, and acceptance rules belong in the game project.

`capture run` and one-shot Godot commands use temporary logs that are deleted when the command completes; failures retain bounded structured diagnostic tails, not log paths. `capture daemon` and exports return their explicitly requested artifacts. `capture record` can additionally retain logs with `--logs-dir` (see below).

### Recording evidence logs

`capture record --logs-dir ./artifacts/logs` optionally retains unique, raw Godot stdout/stderr files when the child process exits. On success their absolute paths appear in `artifacts.stdout_log` and `artifacts.stderr_log`; `elapsed_ms` reports wall-clock child runtime, not movie duration. These files preserve renderer/device startup messages, fallback warnings and project output without parsing them into a guessed renderer or quality score. Repeated runs do not overwrite logs.

Nonzero exits and missing/empty recordings include retained paths in `details.artifacts`. Spawn failures, timeouts and cancellation retain the existing bounded diagnostics only; full log retention for these cases is not implemented. Without `--logs-dir`, temporary logs are deleted as before. If retention fails, the command reports `io_failed`; an already-created movie may still exist. Logs may contain private project output; review before sharing. Movie success is not a gameplay, visual-quality or real-time performance pass.

`resolution` reports requested dimensions, not a media probe: project viewport settings can produce a different encoded size. Verify actual dimensions, frame count and frame rate with a media inspector such as ffprobe before paired visual comparisons.
