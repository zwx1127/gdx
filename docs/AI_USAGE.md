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
