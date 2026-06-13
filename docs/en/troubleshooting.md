# Troubleshooting

Do not share private paths, proprietary assets, prompts, agent output, screenshots, daemon session tokens, or credentials in public issues.

## gdx or Godot is not found

Run:

```powershell
gdx doctor
```

If `gdx` is not on `PATH`, use the repo binary after `cargo build --workspace`. If Godot is not found, set `GDX_GODOT` or pass `--godot`.

## Godot crashes in native code

If stderr JSON reports `error: "godot_native_crash"` or diagnostics `primary_error: "godot_native_crash"`, Godot exited before gdx received runtime JSON. Check the included Godot stdout/stderr logs and the local Godot/runtime environment. gdx reports this distinctly from GDScript parse errors, but it does not switch Godot binaries automatically.

## Main scene is missing

`daemon start` and `capture run` use the configured main scene when `--scene` is omitted.

Fix with one of:

```powershell
gdx --project .\demo scene create --out res://scenes/main.tscn --root-type Node2D --name Main --set-main
gdx --project .\demo setting set --section application --key run/main_scene --value res://scenes/main.tscn
gdx --project .\demo daemon start --scene res://scenes/main.tscn
```

## Script checks fail

Read stderr JSON first:

- `error.code`
- `message`
- `suggestion`
- `details.diagnostics.primary_error`
- `artifacts.stderr_log`

Fix the first parser or load error, then re-run:

```powershell
gdx --project <project> script check-all
```

If diagnostics reports warning-as-error behavior, check for `:=` values derived from `Dictionary` or `Variant`. Prefer plain assignment or explicit types.

## Asset import reports .NET or hostfxr warnings

When the project has no C# files, `asset import` may return JSON with `skipped: true` and `reason: mono_dotnet_unavailable`. Treat this as a warning for non-C# projects and continue with script/runtime checks.

If the project uses C#, install the required .NET and Godot Mono dependencies.

## Daemon is already running or stale

Use:

```powershell
gdx --project .\demo daemon status
gdx --project .\demo daemon start --restart
gdx --project .\demo daemon stop --force
```

Daemon session data lives under `.gdx/daemon/session.json`. Prefer command cleanup over deleting files manually.

## Daemon runtime is older than the CLI

If an input or verify step reports `daemon_runtime_outdated`, the running project daemon does not support an RPC method used by this CLI. Reinstall the bundled runtime and restart the daemon:

```powershell
gdx --project .\demo project install
gdx --project .\demo daemon start --restart
```

`daemon status` reports runtime capabilities when supported. A capabilities `status` of `unknown` usually means the project runtime predates the capabilities RPC.

## Screenshot is missing or blank

Check:

- The command JSON diagnostics.
- The first Godot stderr error if the daemon exited early.
- The main scene exists and has visible content.
- 2D/3D scenes have a suitable camera when needed.
- Assets were imported before capture.
- Capture resolution is high enough for the target view.

For UI regressions, prefer `verify --spec`, project-level methods, `input click-node`, or `input activate`.

## Scene build fails

Check:

- The spec has top-level `out` and `root`.
- Every `type` is a real Godot class.
- Every script and resource path exists under the project.
- Godot resources use `res://` paths.
- Vector and color properties use the wrappers documented by the scene-spec reference.

## Export fails

Export requires:

- `export_presets.cfg` in the project.
- A preset name matching the command.
- Installed Godot export templates.
- An existing output directory.

If export is optional, report the export blocker and keep runtime/test verification complete.
