extends SceneTree

func _init() -> void:
    var action := OS.get_environment("GDX_TOOL_ACTION")
    var params_text := OS.get_environment("GDX_TOOL_PARAMS")
    var params = JSON.parse_string(params_text) if params_text != "" else {}
    if typeof(params) != TYPE_DICTIONARY:
        _fail("invalid_params", "GDX_TOOL_PARAMS must be a JSON object")
        return

    match action:
        "asset_inspect":
            _asset_inspect(params)
        "script_attach":
            _script_attach(params)
        "script_check_all":
            _script_check_all(params)
        "scene_build":
            _scene_build(params)
        "resource_create":
            _resource_create(params)
        "resource_inspect":
            _resource_inspect(params)
        "test_run":
            _test_run(params)
        "project_input_add":
            _project_input_add(params)
        "project_input_remove":
            _project_input_remove(params)
        "project_input_list":
            _project_input_list(params)
        _:
            _fail("unknown_action", "Unknown automation action: %s" % action)

func _asset_inspect(params: Dictionary) -> void:
    var path := str(params.get("path", ""))
    if path == "":
        _fail("missing_path", "path is required")
        return
    var exists := ResourceLoader.exists(path)
    var loaded = load(path) if exists else null
    _ok({
        "path": path,
        "exists": exists,
        "loaded": loaded != null,
        "type": loaded.get_class() if loaded != null else null,
    })

func _script_attach(params: Dictionary) -> void:
    var scene_path := str(params.get("scene", ""))
    var node_path := str(params.get("node", ""))
    var script_path := str(params.get("script", ""))
    var out_path := str(params.get("out", scene_path))
    if scene_path == "" or node_path == "" or script_path == "":
        _fail("missing_params", "scene, node, and script are required")
        return
    var root := _load_scene_root(scene_path)
    if root == null:
        return
    var node := _resolve_node(root, node_path)
    if node == null:
        root.free()
        _fail("node_not_found", "Node not found: %s" % node_path)
        return
    var script = load(script_path)
    if script == null:
        root.free()
        _fail("script_load_failed", "Cannot load script: %s" % script_path)
        return
    node.set_script(script)
    _save_scene_root(root, out_path)

func _script_check_all(params: Dictionary) -> void:
    var root_path := str(params.get("root", "res://"))
    var scripts: Array = []
    _collect_scripts(root_path, scripts)
    var checked: Array = []
    var failed: Array = []
    for path in scripts:
        var loaded = load(path)
        checked.append(path)
        if loaded == null:
            failed.append(path)
    _ok({
        "checked": checked,
        "failed": failed,
        "count": checked.size(),
        "ok": failed.is_empty(),
    }, 0 if failed.is_empty() else 1)

func _scene_build(params: Dictionary) -> void:
    var out_path := str(params.get("out", ""))
    var root_spec: Dictionary = params.get("root", {})
    if out_path == "" or root_spec.is_empty():
        _fail("missing_params", "out and root are required")
        return
    var root := _node_from_spec(root_spec, null)
    if root == null:
        return
    _save_scene_root(root, out_path)

func _resource_create(params: Dictionary) -> void:
    var out_path := str(params.get("out", ""))
    var type_name := str(params.get("type", ""))
    if out_path == "" or type_name == "":
        _fail("missing_params", "out and type are required")
        return
    if not ClassDB.class_exists(type_name):
        _fail("unknown_resource_type", "Unknown type: %s" % type_name)
        return
    var inst = ClassDB.instantiate(type_name)
    if inst == null or not (inst is Resource):
        _fail("invalid_resource_type", "Type must instantiate a Resource: %s" % type_name)
        return
    var resource := inst as Resource
    var properties: Dictionary = params.get("properties", {})
    for key in properties.keys():
        resource.set(str(key), _to_variant(properties[key]))
    var err := ResourceSaver.save(resource, out_path)
    if err != OK:
        _fail("resource_save_failed", "ResourceSaver.save failed: %s" % err)
        return
    _ok({ "out": out_path, "type": resource.get_class() })

func _resource_inspect(params: Dictionary) -> void:
    var path := str(params.get("path", ""))
    if path == "":
        _fail("missing_path", "path is required")
        return
    var exists := ResourceLoader.exists(path)
    var loaded = load(path) if exists else null
    _ok({
        "path": path,
        "exists": exists,
        "loaded": loaded != null,
        "type": loaded.get_class() if loaded != null else null,
    })

func _test_run(params: Dictionary) -> void:
    var path := str(params.get("path", ""))
    var method := str(params.get("method", "run_tests"))
    if path == "":
        _fail("missing_path", "path is required")
        return
    var script = load(path)
    if script == null:
        _fail("script_load_failed", "Cannot load script: %s" % path)
        return
    var inst = script.new()
    if inst == null:
        _fail("script_instantiate_failed", "Cannot instantiate test script: %s" % path)
        return
    if not inst.has_method(method):
        _fail("method_not_found", "Test script has no method: %s" % method)
        return
    var result = inst.call(method)
    _ok({ "path": path, "method": method, "result": _json_safe(result) })

func _project_input_add(params: Dictionary) -> void:
    var action := str(params.get("action", ""))
    if action == "":
        _fail("missing_action", "action is required")
        return
    var event: InputEvent = null
    if params.get("keycode", null) != null:
        var key := InputEventKey.new()
        key.keycode = int(params["keycode"])
        event = key
    elif params.get("mouse_button", null) != null:
        var mouse := InputEventMouseButton.new()
        mouse.button_index = int(params["mouse_button"])
        event = mouse
    else:
        _fail("missing_event", "keycode or mouse_button is required")
        return
    var events: Array = []
    if ProjectSettings.has_setting("input/%s" % action):
        var existing: Dictionary = ProjectSettings.get_setting("input/%s" % action)
        events = existing.get("events", [])
    events.append(event)
    ProjectSettings.set_setting("input/%s" % action, {
        "deadzone": float(params.get("deadzone", 0.5)),
        "events": events,
    })
    var err := ProjectSettings.save()
    if err != OK:
        _fail("project_settings_save_failed", "ProjectSettings.save failed: %s" % err)
        return
    _ok({ "action": action, "events": events.size() })

func _project_input_remove(params: Dictionary) -> void:
    var action := str(params.get("action", ""))
    if action == "":
        _fail("missing_action", "action is required")
        return
    var existed := ProjectSettings.has_setting("input/%s" % action)
    if existed:
        ProjectSettings.clear("input/%s" % action)
        var err := ProjectSettings.save()
        if err != OK:
            _fail("project_settings_save_failed", "ProjectSettings.save failed: %s" % err)
            return
    _ok({ "action": action, "removed": existed })

func _project_input_list(_params: Dictionary) -> void:
    var actions: Array = []
    for setting in ProjectSettings.get_property_list():
        var name := str(setting.get("name", ""))
        if name.begins_with("input/"):
            actions.append(name.substr("input/".length()))
    actions.sort()
    _ok({ "actions": actions })

func _load_scene_root(scene_path: String) -> Node:
    var packed = load(scene_path)
    if packed == null:
        _fail("scene_load_failed", "Cannot load scene: %s" % scene_path)
        return null
    var root = packed.instantiate()
    if root == null:
        _fail("scene_instantiate_failed", "Cannot instantiate scene: %s" % scene_path)
        return null
    return root

func _save_scene_root(root: Node, out_path: String) -> void:
    _set_owner_recursive(root, root)
    var packed := PackedScene.new()
    var pack_err := packed.pack(root)
    if pack_err != OK:
        root.free()
        _fail("pack_failed", "PackedScene.pack failed: %s" % pack_err)
        return
    packed.take_over_path(out_path)
    var save_err := ResourceSaver.save(packed, out_path)
    root.free()
    if save_err != OK:
        _fail("save_failed", "ResourceSaver.save failed: %s" % save_err)
        return
    _ok({ "out": out_path })

func _node_from_spec(spec: Dictionary, owner: Node) -> Node:
    var type_name := str(spec.get("type", "Node"))
    if not ClassDB.class_exists(type_name):
        _fail("unknown_node_type", "Unknown node type: %s" % type_name)
        return null
    var inst = ClassDB.instantiate(type_name)
    if inst == null or not (inst is Node):
        _fail("invalid_node_type", "Type must instantiate a Node: %s" % type_name)
        return null
    var node := inst as Node
    node.name = str(spec.get("name", type_name))
    if owner != null:
        node.owner = owner
    if spec.has("script"):
        var script = load(str(spec["script"]))
        if script == null:
            node.free()
            _fail("script_load_failed", "Cannot load script: %s" % str(spec["script"]))
            return null
        node.set_script(script)
    var properties: Dictionary = spec.get("properties", {})
    for key in properties.keys():
        node.set(str(key), _to_variant(properties[key]))
    for group in spec.get("groups", []):
        node.add_to_group(str(group), true)
    for child_spec in spec.get("children", []):
        if typeof(child_spec) != TYPE_DICTIONARY:
            continue
        var child := _node_from_spec(child_spec, owner if owner != null else node)
        if child == null:
            node.free()
            return null
        node.add_child(child)
        child.owner = owner if owner != null else node
        _set_owner_recursive(child, owner if owner != null else node)
    return node

func _collect_scripts(root_path: String, out: Array) -> void:
    var dir := DirAccess.open(root_path)
    if dir == null:
        return
    dir.list_dir_begin()
    while true:
        var name := dir.get_next()
        if name == "":
            break
        if name.begins_with("."):
            continue
        var path := root_path.path_join(name)
        if dir.current_is_dir():
            _collect_scripts(path, out)
        elif path.ends_with(".gd"):
            out.append(path)
    dir.list_dir_end()

func _resolve_node(root: Node, path: String) -> Node:
    if path == "" or path == "/":
        return root
    var clean := path.substr(1) if path.begins_with("/") else path
    return root.get_node_or_null(NodePath(clean))

func _set_owner_recursive(node: Node, owner: Node) -> void:
    for child in node.get_children():
        child.owner = owner
        _set_owner_recursive(child, owner)

func _to_variant(value):
    if typeof(value) != TYPE_DICTIONARY:
        return value
    if value.has("vec2"):
        var v2: Array = value["vec2"]
        return Vector2(float(v2[0]), float(v2[1]))
    if value.has("vec3"):
        var v3: Array = value["vec3"]
        return Vector3(float(v3[0]), float(v3[1]), float(v3[2]))
    if value.has("color"):
        var c: Array = value["color"]
        return Color(float(c[0]), float(c[1]), float(c[2]), float(c[3] if c.size() > 3 else 1.0))
    if value.has("resource"):
        return load(str(value["resource"]))
    if value.has("node_path"):
        return NodePath(str(value["node_path"]))
    return value

func _json_safe(value):
    match typeof(value):
        TYPE_NIL, TYPE_BOOL, TYPE_INT, TYPE_FLOAT, TYPE_STRING:
            return value
        TYPE_ARRAY:
            var arr := []
            for item in value:
                arr.append(_json_safe(item))
            return arr
        TYPE_DICTIONARY:
            var dict := {}
            for key in value.keys():
                dict[str(key)] = _json_safe(value[key])
            return dict
        _:
            return str(value)

func _ok(result: Dictionary, exit_code := 0) -> void:
    print(JSON.stringify({ "ok": exit_code == 0, "result": result }))
    quit(exit_code)

func _fail(code: String, message: String) -> void:
    push_error(message)
    print(JSON.stringify({ "ok": false, "error": code, "message": message }))
    quit(1)
