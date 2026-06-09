# Scene Build Specs

For project-facing CLI and agent documentation, see `../../../docs/en/cli-reference.md`, `../../../docs/en/agent-usage.md`, or the matching files under `../../../docs/zh-CN/`. This page is the compact agent-facing scene spec reference.

Use `gdx --project <project> scene build --spec <spec.json>` to construct scenes through Godot rather than manually editing `.tscn`.

## Shape

```json
{
  "out": "res://scenes/main.tscn",
  "root": {
    "type": "Node2D",
    "name": "Main",
    "script": "res://scripts/main.gd",
    "properties": {
      "position": { "vec2": [0, 0] }
    },
    "groups": ["game_root"],
    "children": []
  }
}
```

Required:

- Top-level `out` must be a `res://` scene path.
- Top-level `root` must be an object.
- Each node `type` must be a Godot class name that instantiates a `Node`.

Optional node fields:

- `name`: node name. Defaults to the type name.
- `script`: `res://` script loaded and attached to the node.
- `properties`: dictionary of Godot property names to values.
- `groups`: array of group names.
- `children`: array of child node specs.

## Property Values

Plain JSON values are passed through directly:

```json
"text"
123
true
```

Use wrappers for Godot variants:

```json
{ "vec2": [640, 360] }
{ "vec3": [1, 2, 3] }
{ "color": [1, 1, 1, 1] }
{ "resource": "res://assets/player.png" }
{ "node_path": "../Player" }
```

`color` may omit alpha when using daemon `node set`; in scene specs, prefer four components.

## 2D Example

```json
{
  "out": "res://scenes/main.tscn",
  "root": {
    "type": "Node2D",
    "name": "Main",
    "script": "res://scripts/main.gd",
    "children": [
      {
        "type": "Label",
        "name": "Title",
        "properties": {
          "text": "Hello gdx",
          "position": { "vec2": [40, 40] }
        }
      },
      {
        "type": "Camera2D",
        "name": "Camera",
        "properties": {
          "enabled": true,
          "position": { "vec2": [640, 360] }
        }
      }
    ]
  }
}
```

## Guidance

- Keep specs deterministic and explicit. Set positions, sizes, camera state, and labels deliberately.
- Create scripts and assets before building a scene that references them.
- Run `asset import` before relying on newly copied assets.
- Run `script check-all` after writing scripts and before runtime capture.
- If `unknown_node_type`, `script_load_failed`, or `resource` loading fails, fix the class name or `res://` path, then rebuild the scene.
