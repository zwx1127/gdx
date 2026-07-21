# Changelog

All notable changes to `gdx` will be documented in this file.

The format follows the spirit of [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project is currently pre-1.0.

## Unreleased

### Added

- NDJSON/human progress reporting, heartbeats, structured timing/log summaries, and persistent per-child run metadata.
- Bounded parallel `script check-all` jobs with per-script/global timeouts and explicit subtree exclusions.
- Run-history listing and preview-first retention pruning.
- Project-aware standard/.NET Godot selection and .NET SDK checks in `doctor`.
- Daemon startup health checks for RPC capabilities, scene loading, and hard project errors.
- File-based JSON arguments for `call invoke` and `node set`.
- Bilingual open-source documentation for users, AI agents, and contributors.
- Standard contribution, security, changelog, and dual-license files.

### Changed

- One-shot Godot children now use managed process trees so cancellation and timeouts preserve logs without leaving descendants behind.
- Log diagnostics stream large files, retain a bounded tail, and aggregate repeated messages without loading complete logs into memory.

## 0.1.0

### Added

- Rust CLI binary named `gdx`.
- Godot 4.x project creation, installation, inspection, settings, autoloads, input maps, assets, scripts, scenes, resources, tests, daemon control, screenshots, verification specs, and export build commands.
- Bundled Codex skill for Godot game development workflows.
