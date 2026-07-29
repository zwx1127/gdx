# AI Usage

This page is kept for compatibility with older links.

- English: [Agent usage](en/agent-usage.md)
- Chinese: [Agent 使用指南](zh-CN/agent-usage.md)

`gdx` is intended to be driven by agents through subprocess calls. Success is one compact JSON line on stdout; progress and failures use stderr. Failures may include bounded diagnostics and suggestions. Retain output with ordinary shell redirection when needed.

Codex or another coding agent remains responsible for game design, project architecture, GDScript, scene/resource specs, test logic, and interpreting failures. `gdx` only applies those decisions to a real Godot 4.x project.
