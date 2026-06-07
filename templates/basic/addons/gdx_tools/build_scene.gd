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
    "Node3D": true,
    "Camera3D": true,
    "MeshInstance3D": true,
    "DirectionalLight3D": true,
    "OmniLight3D": true,
    "SpotLight3D": true,
    "StaticBody3D": true,
    "CharacterBody3D": true,
    "RigidBody3D": true,
    "CollisionShape3D": true,
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
    if value.has("vector2"):
        return Vector2(float(value["vector2"][0]), float(value["vector2"][1]))
    if value.has("vec3"):
        return Vector3(float(value["vec3"][0]), float(value["vec3"][1]), float(value["vec3"][2]))
    if value.has("vector3"):
        return Vector3(float(value["vector3"][0]), float(value["vector3"][1]), float(value["vector3"][2]))
    if value.has("color"):
        return Color(float(value["color"][0]), float(value["color"][1]), float(value["color"][2]), float(value["color"][3]))
    if value.has("transform3d"):
        return _to_transform3d(value["transform3d"])
    if value.has("mesh"):
        return _to_mesh(value["mesh"])
    if value.has("material"):
        return _to_material(value["material"])
    if value.has("shape3d"):
        return _to_shape3d(value["shape3d"])
    if value.has("resource"):
        return load(str(value["resource"]))
    if value.has("node_path"):
        return NodePath(str(value["node_path"]))
    return value

func _to_transform3d(value: Dictionary) -> Transform3D:
    var origin_values: Array = value.get("origin", [0, 0, 0])
    var origin := Vector3(float(origin_values[0]), float(origin_values[1]), float(origin_values[2]))
    var basis_values: Array = value.get("basis", [[1, 0, 0], [0, 1, 0], [0, 0, 1]])
    var x := Vector3(float(basis_values[0][0]), float(basis_values[0][1]), float(basis_values[0][2]))
    var y := Vector3(float(basis_values[1][0]), float(basis_values[1][1]), float(basis_values[1][2]))
    var z := Vector3(float(basis_values[2][0]), float(basis_values[2][1]), float(basis_values[2][2]))
    return Transform3D(Basis(x, y, z), origin)

func _to_mesh(value: Dictionary):
    var type_name := str(value.get("type", ""))
    match type_name:
        "box":
            var mesh := BoxMesh.new()
            var size: Array = value.get("size", [1, 1, 1])
            mesh.size = Vector3(float(size[0]), float(size[1]), float(size[2]))
            return mesh
        "sphere":
            var mesh := SphereMesh.new()
            mesh.radius = float(value.get("radius", 0.5))
            mesh.height = float(value.get("height", mesh.radius * 2.0))
            return mesh
        "plane":
            var mesh := PlaneMesh.new()
            var size: Array = value.get("size", [1, 1])
            mesh.size = Vector2(float(size[0]), float(size[1]))
            return mesh
        _:
            _fail("unsupported_mesh", "Unsupported mesh type: %s" % type_name)
            return null

func _to_material(value: Dictionary) -> StandardMaterial3D:
    var material := StandardMaterial3D.new()
    if value.has("color"):
        material.albedo_color = _to_variant({ "color": value["color"] })
    return material

func _to_shape3d(value: Dictionary):
    var type_name := str(value.get("type", ""))
    match type_name:
        "box":
            var shape := BoxShape3D.new()
            var size: Array = value.get("size", [1, 1, 1])
            shape.size = Vector3(float(size[0]), float(size[1]), float(size[2]))
            return shape
        "sphere":
            var shape := SphereShape3D.new()
            shape.radius = float(value.get("radius", 0.5))
            return shape
        "capsule":
            var shape := CapsuleShape3D.new()
            shape.radius = float(value.get("radius", 0.5))
            shape.height = float(value.get("height", 2.0))
            return shape
        _:
            _fail("unsupported_shape3d", "Unsupported shape3d type: %s" % type_name)
            return null

func _fail(code: String, message: String) -> void:
    push_error(message)
    print(JSON.stringify({ "ok": false, "error": code, "message": message }))
    quit(1)
