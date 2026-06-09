# Developing gdx

`gdx` is a Rust workspace with one CLI crate and bundled Godot runtime resources.

## Repository layout

- `crates/gdx-cli/src/`: CLI commands, context, Godot process handling, daemon RPC, diagnostics, and JSON output.
- `crates/gdx-cli/resources/addons/`: Godot scripts installed into target projects by `project install`.
- `docs/`: user, agent, and developer documentation.
- `skills/gdx-game-dev/`: Codex skill and compact command references.
- `tests/e2e/`: PowerShell scripts that create temporary Godot projects and exercise real workflows.

## Build and test

```powershell
cargo fmt --check
cargo test --workspace
cargo build --workspace
```

When Godot is available:

```powershell
$env:GDX_GODOT = "C:\Path\To\Godot_v4.x.exe"
target\debug\gdx.exe doctor
.\tests\e2e\hello_world.ps1 -Godot $env:GDX_GODOT
```

Run the E2E script closest to your change. Broaden to daemon, scene edit, UI click, 3D, or export workflows when touching shared behavior.

## Adding or changing commands

Keep the CLI contract stable:

- Successful output is JSON on stdout.
- Failures are JSON on stderr.
- Error codes should be stable, narrow, and useful to automation.
- Include `suggestion` text when the user or agent can take a clear next action.
- Include Godot stdout/stderr log artifact paths when a Godot process was started.

Update:

- `README.md` and `README.zh-CN.md` for user-visible workflows.
- `docs/en/cli-reference.md` and `docs/zh-CN/cli-reference.md` for command syntax.
- `docs/en/agent-usage.md` and `docs/zh-CN/agent-usage.md` for automation behavior.
- `skills/gdx-game-dev/` when agent-facing behavior changes.

## Godot runtime resources

Runtime files under `crates/gdx-cli/resources/addons/` are copied into target projects. Treat them as part of the public automation contract. Changes there should be verified with real Godot runs when possible.

## Documentation style

- Keep README files short enough to be project entry points.
- Put long command details in `docs/`.
- Keep English and Chinese docs structurally aligned.
- Use PowerShell examples for Windows E2E paths, and keep commands copyable.
