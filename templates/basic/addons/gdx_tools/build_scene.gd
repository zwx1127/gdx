extends SceneTree

const ALLOWED_TYPES := {
    "Node": true,
    "Node2D": true,
    "Sprite2D": true,
    "Label": true,
    "Camera2D": true,
    "Area2D": true,
    "CollisionShape2D": true,
    "CharacterBody2D": true,
    "RigidBody2D": true,
    "StaticBody2D": true,
}

func _init() -> void:
    var spec_path := OS.get_environment("GDX_SCENE_SPEC")
    var out_path := OS.get_environment("GDX_SCENE_OUT")

    if spec_path == "" or out_path == "":
        _fail("missing_env", "GDX_SCENE_SPEC and GDX_SCENE_OUT are required")
        return

    var text := FileAccess.get_file_as_string(spec_path)
    if text == "":
        _fail("read_spec_failed", "Cannot read scene spec: %s" % spec_path)
        return

    var spec = JSON.parse_string(text)
    if typeof(spec) != TYPE_DICTIONARY:
        _fail("invalid_json", "Scene spec must be a JSON object")
        return

    var root_spec = spec.get("root")
    if typeof(root_spec) != TYPE_DICTIONARY:
        _fail("invalid_scene_spec", "Scene spec requires a root object")
        return

    var root := _build_node(root_spec)
    if root == null:
        return

    _set_owner_recursive(root, root)

    var packed := PackedScene.new()
    var pack_err := packed.pack(root)
    if pack_err != OK:
        _fail("pack_failed", "PackedScene.pack failed: %s" % pack_err)
        return

    var save_err := ResourceSaver.save(packed, out_path)
    if save_err != OK:
        _fail("save_failed", "ResourceSaver.save failed: %s" % save_err)
        return

    print(JSON.stringify({ "ok": true, "out": out_path }))
    quit(0)

func _build_node(spec: Dictionary) -> Node:
    var type_name := str(spec.get("type", ""))
    var node_name := str(spec.get("name", type_name))

    if not ALLOWED_TYPES.has(type_name):
        _fail("unsupported_node_type", "Unsupported node type: %s" % type_name)
        return null

    if not ClassDB.class_exists(type_name):
        _fail("unknown_node_type", "Unknown node type: %s" % type_name)
        return null

    var node = ClassDB.instantiate(type_name) as Node
    if node == null:
        _fail("instantiate_failed", "Cannot instantiate node type: %s" % type_name)
        return null

    node.name = node_name

    if spec.has("script"):
        var script_path := str(spec["script"])
        var script_res = load(script_path)
        if script_res == null:
            _fail("script_load_failed", "Cannot load script: %s" % script_path)
            return null
        node.set_script(script_res)

    var props: Dictionary = spec.get("properties", {})
    for key in props.keys():
        node.set(str(key), _to_variant(props[key]))

    var children: Array = spec.get("children", [])
    for child_spec in children:
        if typeof(child_spec) != TYPE_DICTIONARY:
            _fail("invalid_child", "Child scene spec must be an object")
            return null
        var child := _build_node(child_spec)
        if child == null:
            return null
        node.add_child(child)

    return node

func _set_owner_recursive(node: Node, owner: Node) -> void:
    for child in node.get_children():
        child.owner = owner
        _set_owner_recursive(child, owner)

func _to_variant(value):
    if typeof(value) != TYPE_DICTIONARY:
        return value
    if value.has("vec2"):
        return Vector2(float(value["vec2"][0]), float(value["vec2"][1]))
    if value.has("vec3"):
        return Vector3(float(value["vec3"][0]), float(value["vec3"][1]), float(value["vec3"][2]))
    if value.has("color"):
        return Color(float(value["color"][0]), float(value["color"][1]), float(value["color"][2]), float(value["color"][3]))
    if value.has("resource"):
        return load(str(value["resource"]))
    if value.has("node_path"):
        return NodePath(str(value["node_path"]))
    return value

func _fail(code: String, message: String) -> void:
    push_error(message)
    print(JSON.stringify({ "ok": false, "error": code, "message": message }))
    quit(1)
