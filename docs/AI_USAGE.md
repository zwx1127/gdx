# AI Usage

This page is kept for compatibility with older links.

- English: [Agent usage](en/agent-usage.md)
- Chinese: [Agent 使用指南](zh-CN/agent-usage.md)

`gdx` is intended to be driven by agents through subprocess calls. Every command emits JSON. Failures are emitted to stderr as JSON and may include Godot log artifacts, diagnostics, and suggestions.

Codex or another coding agent remains responsible for game design, project architecture, GDScript, scene/resource specs, test logic, and interpreting failures. `gdx` only applies those decisions to a real Godot 4.x project.
