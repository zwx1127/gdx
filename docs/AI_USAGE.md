# AI Usage

gdx MVP-0 is intended to be driven by agents through subprocess calls.

Recommended agent flow:

```powershell
gdx init basic --path $env:TEMP\gdx_hello --name hello --json
gdx scene build --project $env:TEMP\gdx_hello --spec examples\hello_scene.json --out res://scenes/main.tscn --json
gdx asset import --project $env:TEMP\gdx_hello --json
gdx code check --project $env:TEMP\gdx_hello res://scripts/main.gd --json
gdx play run --project $env:TEMP\gdx_hello --scene res://scenes/main.tscn --capture $env:TEMP\gdx_hello\shot.png --json
```

Use `--json` for every command. Failures are emitted to stderr as JSON and include log artifacts when a Godot process was started.

MVP-0 deliberately does not modify Godot engine source, implement a headless GPU display server, run a daemon, expose RPC, or call LLM/VLM APIs.

MVP-1 adds a local daemon for repeated scene edits:

```powershell
gdx serve --project $env:TEMP\gdx_hello --scene res://scenes/main.tscn --json
gdx scene tree --project $env:TEMP\gdx_hello --json
gdx scene add-node --project $env:TEMP\gdx_hello --parent / --type Label --name Subtitle --json
gdx scene set --project $env:TEMP\gdx_hello --node /Subtitle --property text --value-json "Edited by daemon" --json
gdx scene save --project $env:TEMP\gdx_hello --json
gdx capture --project $env:TEMP\gdx_hello --out $env:TEMP\gdx_hello\daemon-shot.png --json
gdx kill --project $env:TEMP\gdx_hello --json
```

The daemon listens only on `127.0.0.1` and uses a per-session token in `.gdx/daemon/session.json`.
