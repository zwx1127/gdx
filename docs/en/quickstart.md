# gdx Quickstart

This guide gets a local `gdx` binary talking to Godot 4.x, then creates a minimal Godot project.

## Requirements

- Rust stable.
- Godot 4.x.
- PowerShell on Windows for the bundled E2E scripts.

## Build

```powershell
cargo build --workspace
```

The local binary is `target\debug\gdx.exe` on Windows.

## Locate Godot

Run:

```powershell
target\debug\gdx.exe doctor
```

If Godot is not on `PATH`, set `GDX_GODOT` or pass `--godot`:

```powershell
$env:GDX_GODOT = "C:\Path\To\Godot_v4.x.exe"
target\debug\gdx.exe --godot $env:GDX_GODOT doctor
```

## Create a project

```powershell
target\debug\gdx.exe project create --path .\demo --name Demo
target\debug\gdx.exe --project .\demo scene create --out res://scenes/main.tscn --root-type Node2D --name Main --set-main
target\debug\gdx.exe --project .\demo project inspect
```

Use Godot class names for `--root-type` and node types, for example `Node2D`, `Control`, `Label`, `Node3D`, `MeshInstance3D`, and `Camera3D`.

## Attach to an existing project

```powershell
gdx --project C:\Path\To\Game project install
gdx --project C:\Path\To\Game project inspect
```

`project install` copies the `addons/gdx_*` runtime files needed by scene automation and daemon workflows. It does not rewrite your game architecture.

After rebuilding or upgrading `gdx`, refresh an already attached project with:

```powershell
gdx --project C:\Path\To\Game project update
```

## Verify the loop

```powershell
gdx --project .\demo asset import
gdx --project .\demo script check-all
gdx --project .\demo capture run --scene res://scenes/main.tscn --out .\demo\.gdx\capture.png
```

If the project has a configured main scene, `capture run` can omit `--scene`.

## Next steps

- Read [CLI reference](cli-reference.md) for command syntax.
- Read [Agent usage](agent-usage.md) when driving `gdx` from Codex or another automation agent.
- Read [Troubleshooting](troubleshooting.md) when Godot, script checks, daemon sessions, or screenshots fail.
