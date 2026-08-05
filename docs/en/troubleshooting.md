# Troubleshooting

Do not share private paths, proprietary assets, prompts, agent output, screenshots, daemon session tokens, or credentials in public issues.

## gdx or Godot is not found

Run:

```powershell
gdx doctor
```

If `gdx` is not on `PATH`, use the repo binary after `cargo build --workspace`. If Godot is not found, set `GDX_GODOT` or pass `--godot`. For mixed projects, configure `GDX_GODOT_STANDARD` and `GDX_GODOT_DOTNET`; explicit `--godot` always wins.

`gdx doctor` identifies GDScript/C# projects, the selected standard/.NET Godot build, and installed .NET SDKs. A GDScript-only project using .NET Godot without an SDK is reported by `doctor` as a non-blocking environment suggestion, but it is omitted from ordinary runtime and daemon failure diagnostics. A C# project without a .NET-enabled Godot build or a .NET 8+ SDK fails before project execution.

## A long command appears stuck

Use `--progress ndjson` and parse stderr one line at a time. A command emits `command_started` immediately and a heartbeat at least every five seconds. Godot child events include PID, subject, and timeout. Redirect stderr with the shell when progress must be retained:

```powershell
gdx --project .\demo --progress ndjson script check-all 1>result.json 2>progress.ndjson
```

Do not treat non-empty stderr as failure; use the process exit code. On timeout or Ctrl+C, gdx terminates the one-shot process tree and deletes its temporary logs after extracting bounded diagnostics. Cancellation exits with code 130.

For `script check-all`, reduce concurrency with `--jobs 1` or `--jobs 2`, narrow the tree with repeated `--exclude res://...`, and adjust `--script-timeout-secs` separately from the global `--timeout-secs`.

## Godot crashes in native code

If stderr JSON reports `error: "godot_native_crash"` or diagnostics `primary_error: "godot_native_crash"`, Godot exited before gdx received runtime JSON. Check the structured stdout/stderr tails and the local Godot/runtime environment. gdx reports this distinctly from GDScript parse errors, but it does not switch Godot binaries automatically.

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

- `error`
- `message`
- `suggestion`
- `details.diagnostics.primary_error`

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

Daemon session data and current logs live under the current user's OS temporary directory, keyed by canonical project path. `daemon stop` and stale-session detection clean them automatically. Existing legacy `.gdx/daemon` directories are not daemon state for current gdx releases.

If `daemon start` reports `project_unhealthy`, inspect `health.first_error` and the attached daemon logs. Parsing, resource, and scene-load errors block startup and the newly started daemon is stopped automatically. Missing .NET SDK messages from a GDScript-only project are treated as environment warnings. Use `--no-health-check` only for deliberate diagnostics; it returns `project_healthy: null`.

The daemon records a content snapshot of active `.gd` files when it starts. If a script is added, modified, or removed, ordinary RPC commands and `daemon start` without `--restart` fail with `daemon_scripts_stale`; `daemon status` reports `script_sync.status: "stale"` and bounded `res://` paths. The daemon stays available for inspection and `daemon stop`. Restart it explicitly before repeating state setup, verification, or capture:

```powershell
gdx --project .\demo daemon start --restart
```

If a project method returns INF, -INF, or NaN, the daemon rejects that response with `daemon_non_finite_response` instead of emitting invalid JSON. Read `details.non_finite_values`; each entry contains an RFC 6901 path relative to the RPC result and a `kind` of `positive_infinity`, `negative_infinity`, or `nan`. Fix the project method to return finite JSON-compatible data.

## A legacy `.gdx/runs` directory consumes disk space

Current gdx releases do not create or manage run history. An existing `.gdx/runs` directory came from an older version and is left untouched to avoid deleting user data. Review and remove or archive it with normal filesystem tools.

## Daemon runtime is older than the CLI

If an input or verify step reports `daemon_runtime_outdated`, the running project daemon does not support an RPC method used by this CLI. Update the bundled runtime and restart the daemon:

```powershell
gdx --project .\demo project update --check
gdx --project .\demo project update
gdx --project .\demo daemon start --restart
```

`daemon start` checks managed runtime hashes and protocol compatibility before spawning Godot. `daemon status` reports a concise nested `runtime` object with `status`, `version`, and `protocol_version`. Stable status values are `current`, `outdated`, `unknown`, and `not_running`.

Touch commands and verify touch steps do not downgrade to mouse events. If the project runtime lacks `touch_sequence`, update the managed runtime files and restart the daemon.

## Screenshot is missing or blank

Check:

- The command JSON diagnostics.
- The first Godot stderr error if the daemon exited early.
- The main scene exists and has visible content.
- 2D/3D scenes have a suitable camera when needed.
- Assets were imported before capture.
- Capture resolution is high enough for the target view.

For UI regressions, prefer `verify --spec`, project-level methods, `input click-node`, or `input activate`. For mobile gameplay that handles `InputEventScreenTouch` or `InputEventScreenDrag`, use touch commands such as `input tap`, `input drag`, `input pinch`, or `input sequence`. Touch commands require `touch_sequence` support in the daemon runtime and intentionally do not fall back to mouse events.

gdx only captures the artifact. The game project owns visual baselines, pixel/SSIM thresholds, masks, and the decision that a result satisfies product requirements. If assets changed, the project workflow must explicitly run `asset import` before capture or daemon restart; gdx does not infer that business-level change.

## Recording is missing or empty

`capture record` writes AVI files through Godot Movie Writer and launches a fresh scene. It does not record an already running daemon session.

Use `capture record --input-sequence <json>` to replay touch events in that fresh scene while recording a gesture. Use a `.avi` output path and keep `--duration` and `--fps` small while debugging. On failure, inspect the structured diagnostics in stderr.

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
