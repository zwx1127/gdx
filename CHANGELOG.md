# Changelog

All notable changes to `gdx` will be documented in this file.

The format follows the spirit of [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project is currently pre-1.0.

## Unreleased

### Added

- NDJSON/human stderr progress reporting and heartbeats.
- Bounded parallel `script check-all` jobs with per-script/global timeouts and explicit subtree exclusions.
- Project-aware standard/.NET Godot selection and .NET SDK checks in `doctor`.
- Doctor provenance for executable path, CLI version, build commit, and managed daemon runtime compatibility.
- Daemon startup health checks for RPC capabilities, scene loading, and hard project errors.
- Daemon GDScript content snapshots with explicit stale-source errors and `script_sync` status reporting.
- Structured `daemon_non_finite_response` diagnostics with RFC 6901 paths for INF, -INF, and NaN values.
- Repeatable string `--arg`, `--args-json -` stdin support, and source-positioned parse diagnostics.
- Bilingual open-source documentation for users, AI agents, and contributors.
- Standard contribution, security, changelog, and dual-license files.

### Changed

- gdx is stateless: success is one compact stdout JSON object, while progress and failures use stderr.
- One-shot Godot children use managed process trees and temporary logs so cancellation and timeouts do not leave descendants or project-local history behind.
- Log diagnostics stream large files, retain a bounded failure tail, and aggregate repeated messages without loading complete logs into memory.
- Environment diagnostics keep Mono/.NET SDK suggestions for GDScript-only projects in `doctor` while omitting them from ordinary runtime and daemon failure diagnostics.
- Daemon startup now rejects missing or mismatched managed runtime files before spawning Godot.
- Daemon session state and current logs now live in a per-user OS temporary directory and are removed on stop, stale detection, or failed startup.
- `project update` is the single managed-addon install/update command; `project inspect`, daemon status, verify, and script-check results are compact summaries.

### Removed

- Project-local run/result persistence, the `runs` command group, `--progress-file`, and the PowerShell subprocess wrapper.
- Redundant `project install`, `asset copy`, `input swipe`, `input send --release`, `project inspect --max-files`, and `project create --force` interfaces.
- `call --args-file`, `call --args-stdin`, and the global `--warnings` result-shaping option.

### Scope

- Visual baselines, image comparison thresholds/masks, product acceptance logic, and the decision to import changed project assets remain project responsibilities rather than generic gdx behavior.

## 0.1.0

### Added

- Rust CLI binary named `gdx`.
- Godot 4.x project creation, installation, inspection, settings, autoloads, input maps, assets, scripts, scenes, resources, tests, daemon control, screenshots, verification specs, and export build commands.
- Bundled Codex skill for Godot game development workflows.
