# Security Policy

## Supported versions

`gdx` is pre-1.0. Security fixes are handled on the `main` branch until release branches exist.

## Reporting a vulnerability

Please do not open a public issue for a vulnerability that exposes local machine access, private project files, prompts, generated agent output, daemon session tokens, or credentials.

Report privately through GitHub Security Advisories when available. If advisories are not available for this repository, contact the maintainer through a private channel and include only the minimum redacted details needed to reproduce the issue.

## Security model

- `gdx` is intended to run on a trusted local machine against trusted Godot projects.
- The daemon binds to `127.0.0.1` and uses per-session state under `.gdx/daemon/`.
- Commands can copy assets, create files, edit scenes, run Godot, execute project scripts, capture screenshots, and build exports.
- Godot project code is treated as executable code. Only run `gdx` on projects you are willing to execute locally.
- Debug logs and diagnostics may include local paths, Godot output, script errors, and project structure.

## Sensitive data

Do not publish:

- `.env` or other local configuration with secrets.
- `.gdx/daemon/session.json`.
- Godot logs containing private paths or project details.
- Generated screenshots of private projects.
- Proprietary assets copied into a test project.
- Prompt text, agent output, or verification state that should remain private.
