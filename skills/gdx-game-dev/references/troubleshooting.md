# Troubleshooting

## gdx or Godot Not Found

- Run `gdx doctor`.
- If `gdx` is not on `PATH`, use the repo binary after `cargo build --workspace`.
- If Godot is not found, set `$env:GDX_GODOT = "C:\Path\To\Godot_v4.x.exe"` or pass `--godot`.

## Missing Main Scene

`daemon start` and `capture run` use the configured main scene when `--scene` is omitted.

Fix with one of:

```powershell
gdx --project .\demo scene create --out res://scenes/main.tscn --root-type Node2D --name Main --set-main
gdx --project .\demo setting set --section application --key run/main_scene --value res://scenes/main.tscn
gdx --project .\demo daemon start --scene res://scenes/main.tscn
```

## Script Check Fails

- Read the stderr JSON and Godot stderr log.
- Fix the first parser or load error.
- Re-run `gdx --project <project> script check-all`.

Use `script check <path>` for a narrow parse check and `script check-all` before runtime verification.

## Asset Import Warning About .NET or hostfxr

When the project has no C# files, `asset import` may return JSON with `skipped: true` and `reason: mono_dotnet_unavailable`. Treat this as a warning for non-C# projects, then continue with script and runtime checks.

If the project uses C#, install the required .NET/Godot Mono dependencies before continuing.

## Daemon Already Running or Stale

Use:

```powershell
gdx --project .\demo daemon status
gdx --project .\demo daemon start --restart
gdx --project .\demo daemon stop --force
```

Daemon session data lives under `.gdx/daemon/session.json`. Prefer command cleanup over deleting files manually.

## Screenshot Missing or Blank

- Ensure the main scene exists and has visible content.
- Add or enable a camera for 2D/3D scenes when needed.
- Increase daemon or capture resolution with `--width` and `--height`.
- For one-shot captures, pass `--scene res://...` if no main scene is set.
- Re-run `asset import` if textures or resources are missing.

## Scene Build Fails

- Validate the spec has top-level `out` and `root`.
- Check every `type` is a real Godot class.
- Check every `script` and `resource` path exists under the project and uses `res://`.
- For vector/color properties, use wrappers from `scene-spec.md`.

## Export Fails

Export requires:

- `export_presets.cfg` in the project.
- A preset name matching the command.
- Installed Godot export templates.
- An existing output directory.

If export is optional for the task, report the export blocker and keep the verified project artifacts.
