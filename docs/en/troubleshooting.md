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

`daemon start`, `capture run`, and `capture record` use the configured main scene when `--scene` is omitted.

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

If an input or verify step reports `daemon_runtime_outdated`, the running project daemon does not support an RPC method used by this CLI. Update the bundled runtime and restart the daemon:

```powershell
gdx --project .\demo project update --check
gdx --project .\demo project update
gdx --project .\demo daemon start --restart
```

`daemon status` promotes `runtime_status`, `runtime_version`, `protocol_version`, `methods`, and `warnings` at the top level. `runtime_status: "unknown"` means the project runtime predates the capabilities RPC; `runtime_status: "outdated"` means the runtime is known but missing a capability such as `touch_sequence`.

Touch commands and verify touch steps do not downgrade to mouse events. If the project runtime lacks `touch_sequence`, update the managed runtime files and restart the daemon.

## Screenshot is missing or blank

Check:

- The command JSON diagnostics.
- The first Godot stderr error if the daemon exited early.
- The main scene exists and has visible content.
- 2D/3D scenes have a suitable camera when needed.
- Assets were imported before capture.
- Capture resolution is high enough for the target view.

For UI regressions, prefer `verify --spec`, project-level methods, `input click-node`, or `input activate`. For mobile gameplay that handles `InputEventScreenTouch` or `InputEventScreenDrag`, use touch commands such as `input tap`, `input swipe`, `input pinch`, or `input sequence`. Touch commands require `touch_sequence` support in the daemon runtime and intentionally do not fall back to mouse events.

## Recording is missing or empty

`capture record` writes AVI files through Godot Movie Writer and launches a fresh scene. It does not record an already running daemon session.

Use `capture record --input-sequence <json>` to replay touch events in that fresh scene while recording a gesture. Use a `.avi` output path, keep `--duration` and `--fps` small while debugging, and inspect `artifacts.stderr_log` when Godot exits without a recording.

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
