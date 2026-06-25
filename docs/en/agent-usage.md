# Agent Usage

`gdx` is designed for subprocess automation. Agents should treat it as a Godot automation layer, not as a substitute for game-development judgment.

`gdx` also works well with remote agent control projects such as [`agent-relay`](https://github.com/zwx1127/agent-relay). In that setup, Codex and `gdx` run on the trusted machine that has Godot installed, while the human operator uses Telegram or Lark/Feishu to send prompts, answer questions, approve actions, receive screenshots, and keep the game-development loop moving from anywhere.

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
- Daemon input, calls, state reads, captures, and one-shot scene recordings.
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

## Remote workflow with agent-relay

Use this pattern when the workstation has Godot installed but the developer wants to operate the loop from a phone or another machine:

1. Run [`agent-relay`](https://github.com/zwx1127/agent-relay) on the same trusted machine as the Godot project.
2. Select the Godot project workspace from chat.
3. Ask Codex to use the `gdx-game-dev` skill and drive `gdx` commands locally.
4. Have Codex run `script check-all`, `test run`, `verify`, or targeted daemon/capture commands after changes.
5. Send screenshots, recordings, or generated artifacts back through the relay when visual review is needed.

This keeps Godot execution, daemon access, project files, and exports local. Treat chat messages, screenshots, logs, and agent output as potentially sensitive project data.

agent-relay project: <https://github.com/zwx1127/agent-relay>

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

For UI flows, prefer `input click-node`, `input activate`, and project-level methods invoked with `call invoke`. Use coordinate clicks only when coordinates are part of the test. For mobile gameplay that listens to touch events, use `input tap`, `input drag`, `input swipe`, `input pinch`, or `input sequence` instead of mouse clicks.

For animation review, use `capture record --out .gdx/recording.avi --duration 3 --fps 60` after selecting the scene state you want to launch. This uses Godot Movie Writer and records a freshly started scene, not the current daemon session.

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

Each step must contain exactly one action. Supported step keys are `call`, `state`, `capture`, `input_click_node`, `input_activate`, `input_tap`, `input_drag`, `input_swipe`, `input_pinch`, and `input_touch_sequence`.

## GDScript caution

When reading from `Dictionary` or `Variant`, avoid `:=` unless the type is explicit. Godot can treat type inference warnings as parse errors during runtime scene loading:

```gdscript
var content_bottom = layout.height - layout.safe_bottom
```
