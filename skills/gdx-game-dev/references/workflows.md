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
gdx --project .\demo verify --spec .\demo\.gdx\verify.json
```

## Existing Game

```powershell
gdx --project C:\Path\To\Game project install
gdx --project C:\Path\To\Game project inspect
```

Use `project inspect` to find the main scene and existing project files. Avoid replacing project structure unless the user requested a rewrite.

After rebuilding or upgrading `gdx`, refresh an already attached project:

```powershell
gdx --project C:\Path\To\Game project update
gdx --project C:\Path\To\Game daemon start --restart
```

## Scene Build Loop

1. Write or update scripts.
2. Copy assets and run `asset import`.
3. Generate a scene spec JSON with top-level `out` and `root`.
4. Run `scene build --spec <json>`.
5. Run `script check-all`.
6. Run `verify --spec <json>` for runtime checks, or start daemon/capture one-shot for narrow visual checks.

```powershell
gdx --project .\demo scene build --spec .\main_scene.json
gdx --project .\demo script check-all
gdx --project .\demo capture run --scene res://scenes/main.tscn --out .\demo\.gdx\capture.png
```

`script check-all` is strict and runs Godot's parser over `.gd` files. Use `script load-check` only for the older fast resource-load check.

## Daemon Edit Loop

Use this when the scene exists and needs iterative node edits or runtime inspection.

```powershell
gdx --project .\demo daemon start --restart --width 1280 --height 720
gdx --project .\demo scene tree
gdx --project .\demo node create --parent / --type Label --name Score
gdx --project .\demo node set --node /Score --property text --value "Score: 0"
gdx --project .\demo node set --node /Score --property position --vec2 24 24
gdx --project .\demo scene save
gdx --project .\demo capture daemon --out .\demo\.gdx\after-edit.png
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
gdx --project .\demo state get --target / --method gdx_state
gdx --project .\demo input send --keycode 32
gdx --project .\demo input send --mouse-button 1 --position 320 180
gdx --project .\demo input click-node --target /StartButton
gdx --project .\demo input activate --target /StartButton
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
    { "capture": { "out": ".gdx/main.png", "frames": 10 } }
  ]
}
```

Run it with:

```powershell
gdx --project .\demo verify --spec .\demo\.gdx\verify.json
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
