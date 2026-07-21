---
name: gdx-game-dev
description: Build, modify, run, test, screenshot, record, and export Godot 4.x games with the gdx CLI. Use when Codex needs to create a new Godot game project, attach to an existing Godot project, generate GDScript, build scenes/resources from JSON specs, edit a running scene through the gdx daemon, capture screenshots, record animation clips, send input, inspect game state, run Godot tests, or produce exports through gdx.
---

# gdx Game Development

Use `gdx` as the Godot automation layer. Codex remains responsible for game design, project architecture, GDScript, scene/resource specs, test logic, and interpreting failures. `gdx` applies those decisions to a real Godot 4.x project through JSON-emitting commands.

## Start Here

1. Locate `gdx` and Godot:
   - Prefer `gdx` on `PATH`.
   - If working inside the `gdx` repo and `gdx` is not on `PATH`, run `cargo build --workspace`, then use `target/debug/gdx.exe` on Windows or `target/debug/gdx` elsewhere.
   - Run `gdx doctor`. If Godot is not discoverable, pass `--godot <path>` or set `GDX_GODOT`. For mixed environments, configure standard and .NET builds separately with `GDX_GODOT_STANDARD` / `GDX_GODOT_DOTNET`; gdx selects by project type.
2. Identify the project:
   - New game: `gdx project create --path <project> --name <Name>`.
   - Existing game: `gdx --project <project> project install`, then `gdx --project <project> project inspect`.
   - After rebuilding or upgrading `gdx`, first run `gdx --project <project> project update --check`, then refresh with `gdx --project <project> project update` and restart any running daemon.
   - Use `--project <dir>` on every command that operates on an existing project.
3. Implement in normal project files:
   - Write GDScript under `res://scripts/...`.
   - Create or copy assets under `res://assets/...`.
   - Build scenes with `scene create`, `scene build`, or daemon `node` commands.
4. Verify in a loop:
   - `gdx --project <project> asset import`
   - `gdx --project <project> script check-all` (bounded parallelism defaults to at most four jobs; use `--jobs 1` or `--jobs 2` when diagnosing contention).
   - `gdx --project <project> test run --path res://tests/smoke_test.gd --method run_tests`
   - `gdx --project <project> verify --spec <project>\.gdx\verify.json`
   - For ad hoc runtime checks, use `daemon start --restart`, `call invoke`, `state get`, and `capture daemon`.
   - For animation review, use `capture record --out <project>\.gdx\recording.avi --duration 3 --fps 60` to launch the scene with Godot Movie Writer and write an AVI clip. Add `--input-sequence <json>` when gesture motion needs to be replayed in that fresh scene.
   - Stop long-running sessions with `gdx --project <project> daemon stop`.

## Required Practices

- Treat every final command response as JSON. Stdout contains one success object. In non-TTY use, stderr is line-delimited progress NDJSON and its final line is the failure object. Parse progress incrementally; do not wait silently when `heartbeat` events are available.
- Use `--progress ndjson` for machine-driven long operations and `--warnings summary` by default. Godot child events provide the child PID, subject, timeout, and complete log paths.
- Use Godot class names exactly, such as `Node2D`, `Control`, `Label`, `Sprite2D`, `CharacterBody2D`, `Node3D`, `MeshInstance3D`, and `Camera3D`.
- Use `res://` paths for Godot project resources and ordinary filesystem paths only for external files, specs, and screenshots.
- Prefer `scene build --spec <json>` for larger scene construction and daemon `node create` / `node set` for incremental edits to a running scene.
- Add an explicit game-state method such as `gdx_state()` on important nodes when runtime verification needs structured state. `state get --target /` defaults to `gdx_state()` when no method or property is supplied, and the result reports whether it read a method or property.
- Run Godot-executing `gdx` commands with filesystem/process access sufficient for `.gdx/`, `.godot/imported/`, `user://`, render windows, and child processes. If the current Codex permission profile or Windows Application Control blocks the compiled binary, report the exact blocker and run the runtime gate from Codex Full access mode or an external terminal; `cargo check` is not a substitute for that gate.
- Prefer project-level automation methods such as `gdx_start_run()` with `call invoke` for game UI regressions. Use `input click-node` or `input activate` for generic controls; use `input tap`, `input drag`, `input swipe`, `input pinch`, or `input sequence` for mobile gameplay that handles touch events. Touch commands require a daemon runtime with `touch_sequence`; if they report `daemon_runtime_outdated`, run `project update --check`, then `project update`, then restart the daemon. Do not reinterpret touch gestures as mouse events. Avoid coordinate clicks unless the coordinate itself is under test.
- Avoid `:=` for values derived from `Dictionary` or `Variant` unless the type is explicit; Godot can treat those inference warnings as runtime parse errors.
- Keep daemon sessions short. Start them for interactive edits, input, state reads, and screenshots; stop them when finished.
- Require `daemon start` health to report `rpc_ready`, `scene_loaded`, and `project_healthy: true` before interactive work. Inspect `health.first_error` on `project_unhealthy`; use `--no-health-check` only for deliberate diagnostics.
- Use `runs list` and preview-only `runs prune` to inspect `.gdx/runs` growth. Only pass `runs prune --apply` after reviewing the candidates; active runs and daemon state are protected.
- Do not edit `.tscn` by hand unless the user explicitly asks and the project already follows that pattern.
- Do not add source-project-specific migration logic to `gdx` itself. Fix the game project, scripts, specs, or assets.

## References

- Read `references/gdx-cli.md` for compact command syntax and error handling.
- Read `references/scene-spec.md` before writing a `scene build` JSON spec.
- Read `references/workflows.md` for end-to-end new project, existing project, daemon, screenshot, test, and export workflows.
- Read `references/troubleshooting.md` when Godot, daemon, import, script, screenshot, or export commands fail.
- For full open-source documentation, read `../../docs/en/agent-usage.md` or `../../docs/zh-CN/agent-usage.md`.

## Useful Bundled Files

- Copy or adapt `assets/scene-specs/hello-2d.json` for a minimal 2D scene build spec.
- Copy or adapt `assets/scripts/smoke_test.gd` for `gdx test run`.
- Use `scripts/resolve-gdx.ps1` to locate a usable `gdx` executable from PowerShell.
- Use `scripts/smoke-test.ps1` for a quick local tool check.
- Use `scripts/new-game-smoke.ps1` for a temporary end-to-end project smoke test when Godot is available.
