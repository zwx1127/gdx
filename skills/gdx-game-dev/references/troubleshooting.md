# Troubleshooting

## Discovery and environment

- Run `gdx doctor`.
- Set `GDX_GODOT` or use `--godot` when Godot is not on `PATH`.
- Configure `GDX_GODOT_STANDARD` and `GDX_GODOT_DOTNET` for mixed environments.
- C# projects require a .NET-enabled Godot build and a current .NET SDK.

## Long commands

- Use `--progress ndjson` and consume stderr incrementally.
- Redirect output with the shell when it must be retained: `gdx ... 1>result.json 2>progress.ndjson`.
- Judge success by exit code, not by whether stderr contains progress.
- Lower `script check-all --jobs` when Godot instances contend for resources.

gdx does not persist command runs or one-shot logs. Old `.gdx/runs` directories are legacy user data and are left untouched.

## Script and runtime failures

- Fix the first structured diagnostic before secondary warnings.
- Avoid `:=` for values derived from `Dictionary` or `Variant` unless explicitly typed.
- `godot_native_crash` means Godot exited before returning runtime JSON; inspect the bounded stdout/stderr tails.
- `daemon_non_finite_response` means a project RPC returned INF, -INF, or NaN. Fix the RFC 6901 paths in `details.non_finite_values` so the method returns finite JSON-compatible data.
- Run `asset import` explicitly when the project workflow knows source assets changed.

## Daemon

```powershell
gdx --project .\demo daemon status
gdx --project .\demo daemon start --restart
gdx --project .\demo daemon stop --force
```

Daemon state is per-user temporary state keyed by canonical project path. Stop and stale-session paths clean it automatically. `project_unhealthy` stops a newly created daemon, but an already-running unhealthy daemon is left for the developer to inspect.

The daemon records a content snapshot of active `.gd` files. `daemon_scripts_stale` blocks ordinary RPC after a script is added, modified, or removed; `daemon status` remains available and reports bounded change paths. Restart explicitly, then recreate runtime state before verification or capture. `daemon stop` remains available while scripts are stale.

For `daemon_runtime_outdated`, run `project update --check`, then `project update`, then restart. Touch commands do not fall back to mouse input.

## Screenshots and recordings

- Confirm the main scene, camera, visible content, import state, and target resolution.
- Use project methods, `input click-node`, or `input activate` for deterministic UI setup.
- Use `input tap`, `input drag`, `input pinch`, or `input sequence` for touch gameplay.
- `capture record` launches a fresh scene; it does not record the active daemon.

The project owns visual baselines, image comparisons, thresholds, masks, gameplay assertions, and acceptance criteria. gdx only performs generic capture and command orchestration.
