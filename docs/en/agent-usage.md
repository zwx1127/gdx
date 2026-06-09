# Agent Usage

`gdx` is designed for subprocess automation. Agents should treat it as a Godot automation layer, not as a substitute for game-development judgment.

The agent owns:

- Game design and implementation decisions.
- GDScript files.
- Scene and resource JSON specs.
- Asset placement.
- Test methods and runtime state methods.
- Failure interpretation and repair.

`gdx` owns:

- Godot binary discovery.
- Project settings, autoloads, and input maps.
- Runtime addon installation.
- Asset import.
- Scene/resource creation.
- Daemon input, calls, state reads, and captures.
- Godot script checks, tests, verification, and exports.

## Command contract

All success output is JSON on stdout. All failures are JSON on stderr.

On failure:

1. Parse stderr JSON.
2. Read `error.code`, `message`, and `suggestion`.
3. Inspect `details.diagnostics.primary_error` when present.
4. Open `artifacts.stdout_log` or `artifacts.stderr_log` only when the JSON summary is not enough.
5. Fix the project file, spec, asset, or command argument.
6. Re-run the narrowest failing command.

Do not infer success from free-form Godot output. Use the JSON `ok` field and expected artifacts such as created scenes or non-empty screenshots.

## Standard workflow

New project:

```powershell
gdx doctor
gdx project create --path .\demo --name Demo
gdx --project .\demo scene create --out res://scenes/main.tscn --root-type Node2D --name Main --set-main
```

Existing project:

```powershell
gdx --project C:\Path\To\Game project install
gdx --project C:\Path\To\Game project inspect
```

Build and verify:

```powershell
gdx --project .\demo asset import
gdx --project .\demo script check-all
gdx --project .\demo test run --path res://tests/smoke_test.gd --method run_tests
gdx --project .\demo verify --spec .\demo\.gdx\verify.json
```

## Runtime state

Expose structured state from gameplay scripts when tests need to inspect behavior:

```gdscript
func gdx_state() -> Dictionary:
    return {
        "score": score,
        "player_position": [player.position.x, player.position.y]
    }
```

Then query it:

```powershell
gdx --project .\demo state get --target / --method gdx_state
```

For UI flows, prefer `input click-node`, `input activate`, and project-level methods invoked with `call invoke`. Use coordinate clicks only when coordinates are part of the test.

## Verify specs

Use `verify` for multi-step regression checks:

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

Each step must contain exactly one action. Supported step keys are `call`, `state`, `capture`, `input_click_node`, and `input_activate`.

## GDScript caution

When reading from `Dictionary` or `Variant`, avoid `:=` unless the type is explicit. Godot can treat type inference warnings as parse errors during runtime scene loading:

```gdscript
var content_bottom = layout.height - layout.safe_bottom
```
