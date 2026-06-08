# gdx

gdx is a Rust CLI that helps AI agents operate real Godot 4.x projects through official Godot command line workflows.

It can create or attach to a project, inspect project structure, create scenes, run a local edit daemon, modify scene nodes, save scenes, import assets, check scripts, capture screenshots, and export builds. gdx does not modify Godot engine source, call LLM/VLM APIs, or expose a network API beyond its local per-project daemon.

## Build

```powershell
cargo build --workspace
```

## Environment

```powershell
gdx env --json
```

If Godot is not on `PATH`, pass `--godot` or set `GDX_GODOT`:

```powershell
$env:GDX_GODOT = "C:\Path\To\Godot_v4.x.exe"
gdx env --json
```

## New Project Workflow

```powershell
gdx init --path .\demo --name Demo --json
gdx scene new --project .\demo --out res://scenes/main.tscn --root-type Node2D --name Main --set-main --json
gdx asset import --project .\demo --json
gdx serve --project .\demo --json
gdx scene add-node --project .\demo --parent / --type Label --name Title --json
gdx scene set --project .\demo --node /Title --property text --value "Hello from gdx" --json
gdx scene set --project .\demo --node /Title --property position --vec2 40 40 --json
gdx scene save --project .\demo --json
gdx capture --project .\demo --out .\demo\shot.png --json
gdx kill --project .\demo --json
```

## Existing Project Workflow

```powershell
gdx project setup --project C:\Path\To\GodotProject --json
gdx project inspect --project C:\Path\To\GodotProject --json
gdx serve --project C:\Path\To\GodotProject --json
gdx scene tree --project C:\Path\To\GodotProject --json
gdx capture --project C:\Path\To\GodotProject --out C:\Path\To\GodotProject\.gdx\shot.png --json
```

`serve` and `play run` use the project's configured main scene when `--scene` is omitted. If the project has no main scene, create one with `scene new --set-main` or pass `--scene res://...`.

## Tests

The Windows E2E scripts require a Godot 4.x executable:

```powershell
.\tests\e2e\hello_world.ps1 -Godot "C:\Path\To\Godot_v4.x.exe"
```

The scripts create temporary projects, exercise the public Agent workflow, and verify that screenshots are created.
