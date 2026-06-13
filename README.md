# gdx

[中文](README.zh-CN.md) | English

`gdx` is a Rust CLI for automating real Godot 4.x projects from scripts and AI agents. It wraps official Godot command-line workflows and exposes project setup, scene editing, runtime control, screenshots, tests, and exports through JSON-emitting commands.

`gdx` is not a game migration framework or a game engine abstraction. The caller, often Codex or another coding agent, still owns game design, architecture, GDScript, scene specs, assets, and failure analysis. `gdx` provides a reliable automation layer around Godot.

When combined with a remote agent operator such as [`agent-relay`](https://github.com/zwx1127/agent-relay), `gdx` can support remote Godot game development from chat. Godot, Codex, and project files stay on a trusted local machine, while you send prompts, approve actions, run checks, inspect screenshots, and steer the development loop from Telegram or Lark/Feishu.

## What you can do

- Create a new Godot project and configure its main scene.
- Install the `gdx` runtime addons into an existing Godot project.
- Update installed project addons to the runtime bundled with the current `gdx` CLI.
- Set project settings, autoloads, and input map entries.
- Copy, import, and inspect assets.
- Create, attach, parse-check, and load-check GDScript files.
- Create scenes directly or build scenes from JSON specs.
- Start a local Godot daemon for live scene edits, input, method calls, state reads, and screenshots.
- Run Godot test scripts and multi-step verification specs.
- Build exports through Godot export presets.
- Pair with [`agent-relay`](https://github.com/zwx1127/agent-relay) to operate a local Codex plus `gdx` workflow remotely from chat.

All successful command output is JSON on stdout. Failures are JSON on stderr and may include Godot logs, diagnostics, and actionable suggestions.

## Quick start

Build the CLI:

```powershell
cargo build --workspace
```

Check that `gdx` can find Godot:

```powershell
target\debug\gdx.exe doctor
```

If Godot is not on `PATH`, pass it explicitly or set `GDX_GODOT`:

```powershell
$env:GDX_GODOT = "C:\Path\To\Godot_v4.x.exe"
target\debug\gdx.exe --godot $env:GDX_GODOT doctor
```

Create and inspect a minimal project:

```powershell
target\debug\gdx.exe project create --path .\demo --name Demo
target\debug\gdx.exe --project .\demo scene create --out res://scenes/main.tscn --root-type Node2D --name Main --set-main
target\debug\gdx.exe --project .\demo project inspect
```

Use `--project <dir>` on every command that operates on an existing Godot project.

## Common workflows

Attach to an existing project:

```powershell
gdx --project C:\Path\To\GodotProject project install
gdx --project C:\Path\To\GodotProject project inspect
```

Update an already attached project after rebuilding or upgrading `gdx`:

```powershell
gdx --project C:\Path\To\GodotProject project update
gdx --project C:\Path\To\GodotProject daemon start --restart
```

Build and verify project files:

```powershell
gdx --project .\demo asset import
gdx --project .\demo script check-all
gdx --project .\demo test run --path res://tests/smoke_test.gd --method run_tests
```

Use the daemon for runtime edits and screenshots:

```powershell
gdx --project .\demo daemon start --restart --width 1280 --height 720
gdx --project .\demo scene tree
gdx --project .\demo node create --parent / --type Label --name Title
gdx --project .\demo node set --node /Title --property text --value "Hello from gdx"
gdx --project .\demo node set --node /Title --property position --vec2 40 40
gdx --project .\demo scene save
gdx --project .\demo capture daemon --out .\demo\.gdx\capture.png
gdx --project .\demo daemon stop
```

Run a multi-step verification spec:

```powershell
gdx --project .\demo verify --spec .\demo\.gdx\verify.json
```

```json
{
  "checks": { "script": { "root": "res://", "strict": true } },
  "tests": [{ "path": "res://tests/smoke_test.gd", "method": "run_tests" }],
  "daemon": { "width": 390, "height": 844, "restart": true, "stop": true },
  "steps": [
    { "call": { "target": "/", "method": "gdx_start_run", "args": [] } },
    { "state": { "target": "/", "method": "gdx_state" } },
    { "capture": { "out": ".gdx/capture.png", "frames": 10 } }
  ]
}
```

## Documentation

- [Quickstart](docs/en/quickstart.md)
- [Agent usage](docs/en/agent-usage.md)
- [CLI reference](docs/en/cli-reference.md)
- [Troubleshooting](docs/en/troubleshooting.md)
- [Developing gdx](docs/en/developing.md)

The bundled Codex skill lives in [`skills/gdx-game-dev`](skills/gdx-game-dev/SKILL.md). It is intended for agents that need to build, modify, run, test, screenshot, and export Godot games through `gdx`.

For remote development, run Codex and `gdx` on the machine that has Godot installed, then use [`agent-relay`](https://github.com/zwx1127/agent-relay) as the chat control surface. This keeps file access, Godot execution, daemon sessions, and exports local while still letting you develop and review progress away from the workstation.

agent-relay project: <https://github.com/zwx1127/agent-relay>

## Requirements

- Rust stable, using the repository `rust-toolchain.toml`.
- Godot 4.x executable available on `PATH`, through `GDX_GODOT`, or with `--godot`.
- PowerShell for the bundled Windows E2E scripts.
- Godot export templates and `export_presets.cfg` only when running `gdx export build`.

## Project status

`gdx` is pre-1.0. The current focus is a reliable local automation loop for Godot 4.x projects:

- CLI: Rust binary named `gdx`.
- Runtime integration: Godot addons installed under `addons/gdx_*`.
- Verification: script checks, Godot tests, daemon state calls, input, and screenshots.
- Supported caller model: local scripts and AI agents.

Known limitations:

- The CLI does not design or migrate games by itself.
- `scene build` consumes a Godot-specific JSON spec; callers are responsible for generating the spec.
- The daemon binds to `127.0.0.1` and is intended for trusted local automation.
- Export requires Godot export presets and installed export templates.

## Contributing and support

- Read [CONTRIBUTING.md](CONTRIBUTING.md) before opening a pull request.
- Read [SECURITY.md](SECURITY.md) before reporting sensitive issues.
- Check [Troubleshooting](docs/en/troubleshooting.md) before opening a setup issue.
- See [CHANGELOG.md](CHANGELOG.md) for release notes.

## License

`gdx` is licensed under either of:

- [MIT License](LICENSE-MIT)
- [Apache License, Version 2.0](LICENSE-APACHE)
