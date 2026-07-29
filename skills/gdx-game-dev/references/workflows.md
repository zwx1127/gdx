# gdx Workflows

For project-facing workflow documentation, see `../../../docs/en/quickstart.md`, `../../../docs/en/agent-usage.md`, or the matching files under `../../../docs/zh-CN/`.

## New Game

```powershell
gdx doctor
gdx project create --path .\demo --name Demo
gdx --project .\demo scene create --out res://scenes/main.tscn --root-type Node2D --name Main --set-main
```

Then write scripts/assets/specs and verify:

```powershell
gdx --project .\demo asset import
gdx --project .\demo script check-all
gdx --project .\demo verify --spec .\demo\verify.json
```

Run `asset import` here because this workflow just created or changed importable assets. It is an explicit project decision, not a daemon-start preflight.

## Existing Game

```powershell
gdx --project C:\Path\To\Game project update
gdx --project C:\Path\To\Game project inspect
```

Use `project inspect` to confirm the main scene, language, and managed addon status. Inspect business files with normal repository tools.

After rebuilding or upgrading `gdx`, refresh an already attached project:

```powershell
gdx --project C:\Path\To\Game project update --check
gdx --project C:\Path\To\Game project update
gdx --project C:\Path\To\Game daemon start --restart
```

## Scene Build Loop

1. Write or update scripts.
2. Copy assets and explicitly run `asset import` when those changes require Godot import.
3. Generate a scene spec JSON with top-level `out` and `root`.
4. Run `scene build --spec <json>`.
5. Run `script check-all`.
6. Run `verify --spec <json>` for runtime checks, use `capture run` for narrow screenshot checks, or use `capture record` for short animation review clips.

```powershell
gdx --project .\demo scene build --spec .\main_scene.json
gdx --project .\demo script check-all
gdx --project .\demo capture run --scene res://scenes/main.tscn --out .\artifacts\capture.png
gdx --project .\demo capture record --scene res://scenes/main.tscn --out .\artifacts\recording.avi --duration 3 --fps 60
gdx --project .\demo capture record --scene res://scenes/main.tscn --out .\artifacts\gesture.avi --duration 3 --fps 60 --input-sequence .\touch-sequence.json
```

`script check-all` is strict and runs Godot's parser over `.gd` files. Use `script load-check` only for the older fast resource-load check.

Keep screenshot baselines, comparison metrics, thresholds, masks, and visual acceptance logic in the game repository. gdx captures evidence and returns paths; it does not decide whether the visuals meet the product goal.

## Long-running automation

Use the shell's native pipelines and redirection:

```powershell
gdx --project .\demo --progress ndjson verify --spec .\demo\verify.json `
    1>verify-result.json 2>verify-progress.ndjson
```

The caller owns retained output, background-process state, retries, and retention. gdx does not create a run database.

## Daemon Edit Loop

Use this when the scene exists and needs iterative node edits or runtime inspection.

```powershell
gdx --project .\demo daemon start --restart --width 1280 --height 720
gdx --project .\demo scene tree
gdx --project .\demo node create --parent / --type Label --name Score
gdx --project .\demo node set --node /Score --property text --value "Score: 0"
gdx --project .\demo node set --node /Score --property position --vec2 24 24
gdx --project .\demo scene save
gdx --project .\demo capture daemon --out .\artifacts\after-edit.png
gdx --project .\demo daemon stop
```

## Runtime Verification

Expose structured state from gameplay scripts:

```gdscript
func gdx_state() -> Dictionary:
    return {
        "score": score,
        "player_position": [player.position.x, player.position.y]
    }
```

Then call:

```powershell
gdx --project .\demo state get --target /
gdx --project .\demo state get --target / --method gdx_state
gdx --project .\demo input send --keycode 32
gdx --project .\demo input send --mouse-button 1 --position 320 180
gdx --project .\demo input send --mouse-button 1 --position 320 180 --pressed false
gdx --project .\demo input click-node --target /StartButton
gdx --project .\demo input activate --target /StartButton
gdx --project .\demo input tap --position 320 180
gdx --project .\demo input drag --from 320 180 --to 320 80
gdx --project .\demo input pinch --center 320 180 --start-distance 120 --end-distance 40
gdx --project .\demo call invoke --target / --method start_game --args-json "[]"
```

For multi-step UI/gameplay regression, put the flow in a verify spec:

```json
{
  "checks": { "script": { "root": "res://", "strict": true } },
  "tests": [{ "path": "res://tests/smoke_test.gd", "method": "run_tests" }],
  "daemon": { "width": 390, "height": 844, "restart": true, "stop": true },
  "steps": [
    { "call": { "target": "/", "method": "gdx_start_run", "args": [] } },
    { "state": { "target": "/", "method": "gdx_state" } },
    { "capture": { "out": "artifacts/main.png", "frames": 10 } }
  ]
}
```

Run it with:

```powershell
gdx --project .\demo verify --spec .\demo\verify.json
```

## Godot Tests

Create a test script with a method returning JSON-compatible values:

```gdscript
extends RefCounted

func run_tests() -> Dictionary:
    return {"ok": true}
```

Run:

```powershell
gdx --project .\demo test run --path res://tests/smoke_test.gd --method run_tests
```

## Export

Export only after the game runs and an export preset exists:

```powershell
gdx --project .\demo export build --preset "Windows Desktop" --out .\demo\export\demo.exe
```

If export fails because templates or presets are missing, report that clearly and leave runtime/test verification complete.
