# gdx

gdx MVP-0 is a small Rust CLI that wraps official Godot command line workflows for agent-first automation.

The MVP-0 loop is:

1. Create a Godot project from a template.
2. Build a scene from a JSON SceneSpec through a headless Godot editor script.
3. Trigger asset import.
4. Run a parse-only script check.
5. Start a normal Godot runtime, capture a PNG, and exit.

This repository does not modify Godot engine source, implement `DisplayServerHeadlessGPU`, parse full `.tscn` files, run a daemon, expose RPC/MCP, or call LLM/VLM APIs.

## Build

```powershell
cargo build --workspace
```

## Check the Environment

```powershell
gdx env --json
```

If Godot is not on `PATH`, pass `--godot` or set `GDX_GODOT`:

```powershell
$env:GDX_GODOT = "C:\Path\To\Godot_v4.4-stable_win64.exe"
gdx env --json
```

## End-to-End Smoke Test

The Windows E2E script requires a Godot 4.x executable:

```powershell
.\tests\e2e\hello_world.ps1 -Godot "C:\Path\To\Godot_v4.4-stable_win64.exe"
```

The script creates a temporary project, builds `examples\hello_scene.json`, imports assets, checks `res://scripts/main.gd`, runs the capture scene, and verifies the screenshot is non-empty.
