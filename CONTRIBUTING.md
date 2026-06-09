# Contributing to gdx

Thanks for improving `gdx`.

## Development setup

Install Rust stable, then build and test the workspace:

```powershell
cargo build --workspace
cargo test --workspace
```

For Godot-backed workflows, install Godot 4.x and make it discoverable:

```powershell
$env:GDX_GODOT = "C:\Path\To\Godot_v4.x.exe"
target\debug\gdx.exe doctor
```

## Pull request expectations

- Keep each PR focused on one behavior, workflow, or documentation topic.
- Add or update tests for behavior changes.
- Update `README.md`, `README.zh-CN.md`, `docs/`, and `skills/gdx-game-dev/` when public CLI behavior changes.
- Keep command output machine-readable JSON.
- Prefer narrow error codes, useful `suggestion` text, and attached Godot log artifacts for failures.
- Do not commit local runtime files such as `.env`, `.agent-relay/`, `.godot/`, `.gdx/runs/`, generated screenshots, exports, or private project assets.

## Project structure

- `crates/gdx-cli/src/`: Rust CLI, command routing, Godot process handling, JSON output, and diagnostics.
- `crates/gdx-cli/resources/addons/`: Godot runtime scripts installed into target projects.
- `docs/`: user, agent, and developer documentation.
- `skills/gdx-game-dev/`: Codex skill and compact agent-facing references.
- `tests/e2e/`: Windows PowerShell E2E scripts that exercise real Godot workflows.

## Local checks

Run the Rust checks before opening a PR:

```powershell
cargo fmt --check
cargo test --workspace
cargo build --workspace
```

When Godot is available, run at least the relevant E2E script:

```powershell
.\tests\e2e\hello_world.ps1 -Godot "C:\Path\To\Godot_v4.x.exe"
```

Use the narrowest E2E script that covers your change, then broaden when touching daemon, scene, input, capture, or export behavior.

## Reporting bugs

Open an issue with:

- OS and shell.
- Rust version.
- Godot version and whether it is on `PATH`.
- Exact `gdx` command.
- Redacted stdout/stderr JSON.
- Relevant Godot log artifact paths or redacted log excerpts.
- Whether the project is new or an existing Godot project.

Do not publish private project paths, proprietary game assets, credentials, prompts, agent output, or screenshots that should not be public.
