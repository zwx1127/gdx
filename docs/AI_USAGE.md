# AI Usage

gdx is intended to be driven by agents through subprocess calls. Use `--json` for every command. Failures are emitted to stderr as JSON and include log artifacts when a Godot process was started.

## Attach to a Project

For an existing Godot project:

```powershell
gdx project setup --project <project> --json
gdx project inspect --project <project> --json
```

`project setup` installs gdx runtime files under `addons/gdx_*`. `project inspect` returns the project name, configured main scene, gdx installation status, and categorized project files.

## Create a Scene

If the project has no main scene:

```powershell
gdx scene new --project <project> --out res://scenes/main.tscn --root-type Node2D --name Main --set-main --json
```

Use Godot class names for `--root-type` and `scene add-node --type`. gdx validates them inside Godot.

## Edit and Verify

```powershell
gdx serve --project <project> --json
gdx scene tree --project <project> --json
gdx scene add-node --project <project> --parent / --type Label --name Status --json
gdx scene set --project <project> --node /Status --property text --value "Ready" --json
gdx scene set --project <project> --node /Status --property position --vec2 40 40 --json
gdx scene save --project <project> --json
gdx capture --project <project> --out <project>\.gdx\capture.png --json
gdx kill --project <project> --json
```

`serve` uses the project's main scene unless `--scene res://...` is provided. The daemon listens only on `127.0.0.1` and uses a per-session token stored in `.gdx/daemon/session.json`.
