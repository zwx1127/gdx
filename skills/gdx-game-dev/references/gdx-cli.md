# gdx CLI Quick Reference

gdx is stateless. Success is one compact JSON line on stdout; progress and failures use stderr. Retain them with shell redirection:

```powershell
gdx --project .\demo --progress ndjson script check-all 1>result.json 2>progress.ndjson
```

Global options are `--godot`, `--godot-standard`, `--godot-dotnet`, `--project`, and `--progress auto|stderr|ndjson|off`. Build provenance is available from `doctor`.

## Project and files

```powershell
gdx doctor
gdx project create --path .\demo --name Demo
gdx --project .\demo project update --check
gdx --project .\demo project update
gdx --project .\demo project inspect

gdx --project .\demo asset import
gdx --project .\demo asset inspect --path res://assets/icon.png
gdx --project .\demo script check res://scripts/main.gd
gdx --project .\demo script check-all --exclude res://vendor
gdx --project .\demo scene build --spec .\scene-spec.json
gdx --project .\demo resource inspect --path res://data/item.tres
```

`project update` installs missing managed addons and updates outdated ones. Copy business assets with normal filesystem tools. `script check-all` returns aggregate counts.

## Runtime

```powershell
gdx --project .\demo daemon start --restart --width 390 --height 844
gdx --project .\demo daemon status
gdx --project .\demo scene tree
gdx --project .\demo node set --node /Title --property text --value "Ready"
gdx --project .\demo scene save
gdx --project .\demo input click-node --target /StartButton
gdx --project .\demo input tap --position 120 240
gdx --project .\demo input drag --from 120 240 --to 220 240
gdx --project .\demo input pinch --center 180 240 --start-distance 120 --end-distance 40
gdx --project .\demo call invoke --target / --method start_game --args-json "[]"
Get-Content -Raw .\args.json | gdx --project .\demo call invoke --target / --method start_game --args-json -
gdx --project .\demo state get --target /
gdx --project .\demo capture daemon --out .\artifacts\capture.png
gdx --project .\demo daemon stop
```

Use `input drag` for swipe motion and `input send --pressed false` for release. Typed call arguments use a JSON array passed to `--args-json`; `-` reads stdin.

Daemon session metadata and current logs live in a per-user OS temporary directory, not the project. Active daemon responses expose concise `runtime`, `health`, and current log artifacts, but not token or full capabilities. `daemon status` also reports `script_sync`; stale `.gd` contents block ordinary RPC with `daemon_scripts_stale` until an explicit restart.

Project RPC methods must return finite JSON-compatible numbers. INF, -INF, or NaN produces `daemon_non_finite_response` with RFC 6901 paths in `details.non_finite_values`.

## Verification and artifacts

```powershell
gdx --project .\demo verify --spec .\verify.json
gdx --project .\demo test run --path res://tests/smoke_test.gd
gdx --project .\demo capture run --out .\artifacts\capture.png
gdx --project .\demo capture record --out .\artifacts\recording.avi --duration 3 --fps 60
gdx --project .\demo export build --preset "Windows Desktop" --out .\export\game.exe
```

Verify steps are `call`, `state`, `capture`, `input_click_node`, `input_activate`, `input_tap`, `input_drag`, `input_pinch`, and `input_touch_sequence`. Product assertions, screenshot comparisons, thresholds, masks, and acceptance rules belong in project tests or scripts.

On failure, read the final stderr JSON first. Parse errors report origin and position without copying input content. One-shot logs are temporary; their bounded diagnostic tails are included only when needed.
