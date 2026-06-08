extends Node

var server := TCPServer.new()
var clients: Array[Dictionary] = []
var token := ""
var target_scene := ""
var scene_out := ""
var scene_root: Node = null
var pending_capture: Dictionary = {}

func _ready() -> void:
    token = OS.get_environment("GDX_DAEMON_TOKEN")
    target_scene = OS.get_environment("GDX_DAEMON_SCENE")
    var port := int(OS.get_environment("GDX_DAEMON_PORT"))

    if token == "" or port <= 0:
        _fatal("missing_env", "GDX_DAEMON_TOKEN and GDX_DAEMON_PORT are required")
        return

    if target_scene != "":
        scene_out = target_scene
        var packed = load(target_scene)
        if packed == null:
            _fatal("scene_load_failed", "Cannot load scene: %s" % target_scene)
            return
        scene_root = packed.instantiate()
        add_child(scene_root)

    var err := server.listen(port, "127.0.0.1")
    if err != OK:
        _fatal("listen_failed", "Cannot listen on 127.0.0.1:%s" % port)
        return

    print(JSON.stringify({ "ok": true, "event": "daemon_ready", "port": port }))

func _process(_delta: float) -> void:
    _accept_clients()
    _read_clients()
    _tick_capture()

func _accept_clients() -> void:
    while server.is_connection_available():
        var peer := server.take_connection()
        if peer != null:
            clients.append({ "peer": peer, "buffer": "" })

func _read_clients() -> void:
    var keep: Array[Dictionary] = []
    for client in clients:
        var peer: StreamPeerTCP = client["peer"]
        if peer.get_status() == StreamPeerTCP.STATUS_NONE:
            continue
        var available := peer.get_available_bytes()
        if available > 0:
            client["buffer"] += peer.get_utf8_string(available)
        var lines := String(client["buffer"]).split("\n", false)
        if not String(client["buffer"]).ends_with("\n") and lines.size() > 0:
            client["buffer"] = lines[lines.size() - 1]
            lines.remove_at(lines.size() - 1)
        else:
            client["buffer"] = ""
        for line in lines:
            _handle_line(peer, line.strip_edges())
        if peer.get_status() != StreamPeerTCP.STATUS_NONE:
            keep.append(client)
    clients = keep

func _handle_line(peer: StreamPeerTCP, line: String) -> void:
    if line == "":
        return
    var req = JSON.parse_string(line)
    if typeof(req) != TYPE_DICTIONARY:
        _send(peer, { "ok": false, "error": "invalid_json", "message": "Request must be a JSON object" })
        return
    var id := str(req.get("id", ""))
    if str(req.get("token", "")) != token:
        _send(peer, { "ok": false, "id": id, "error": "unauthorized", "message": "Invalid daemon token" })
        return
    var method := str(req.get("method", ""))
    var params: Dictionary = req.get("params", {})
    if typeof(params) != TYPE_DICTIONARY:
        _send(peer, { "ok": false, "id": id, "error": "invalid_params", "message": "params must be an object" })
        return

    match method:
        "ping":
            _send_result(peer, id, { "pong": true })
        "shutdown":
            _send_result(peer, id, { "shutdown": true })
            get_tree().quit(0)
        "scene_tree":
            if scene_root == null:
                _send_error(peer, id, "scene_not_loaded", "No target scene loaded")
            else:
                _send_result(peer, id, _node_to_dict(scene_root, "/"))
        "add_node":
            _rpc_add_node(peer, id, params)
        "set_property":
            _rpc_set_property(peer, id, params)
        "save_scene":
            _rpc_save_scene(peer, id, params)
        "capture":
            _rpc_capture(peer, id, params)
        _:
            _send_error(peer, id, "unknown_method", "Unknown RPC method: %s" % method)

func _rpc_add_node(peer: StreamPeerTCP, id: String, params: Dictionary) -> void:
    if scene_root == null:
        _send_error(peer, id, "scene_not_loaded", "No target scene loaded")
        return
    var parent_path := str(params.get("parent", ""))
    var type_name := str(params.get("type", ""))
    var node_name := str(params.get("name", ""))
    var parent := _resolve_node(parent_path)
    if parent == null:
        _send_error(peer, id, "parent_not_found", "Parent not found: %s" % parent_path)
        return
    if not ClassDB.class_exists(type_name):
        _send_error(peer, id, "unknown_node_type", "Unknown node type: %s" % type_name)
        return
    var inst = ClassDB.instantiate(type_name)
    if inst == null or not (inst is Node):
        _send_error(peer, id, "invalid_node_type", "Type must instantiate a Node: %s" % type_name)
        return
    var node := inst as Node
    node.name = node_name
    parent.add_child(node)
    node.owner = scene_root
    _set_owner_recursive(node, scene_root)
    _send_result(peer, id, { "path": _path_for_node(node), "type": type_name, "name": node.name })

func _rpc_set_property(peer: StreamPeerTCP, id: String, params: Dictionary) -> void:
    var node_path := str(params.get("node", ""))
    var property := str(params.get("property", ""))
    var node := _resolve_node(node_path)
    if node == null:
        _send_error(peer, id, "node_not_found", "Node not found: %s" % node_path)
        return
    if property == "":
        _send_error(peer, id, "invalid_property", "property is required")
        return
    node.set(property, _to_variant(params.get("value")))
    _send_result(peer, id, { "path": _path_for_node(node), "property": property })

func _rpc_save_scene(peer: StreamPeerTCP, id: String, params: Dictionary) -> void:
    if scene_root == null:
        _send_error(peer, id, "scene_not_loaded", "No target scene loaded")
        return
    var raw_out = params.get("out", "")
    var out_path := ""
    if raw_out != null:
        out_path = str(raw_out)
    if out_path == "":
        out_path = scene_out
    if out_path == "":
        _send_error(peer, id, "missing_out", "save_scene requires out when daemon was not started with a scene")
        return
    var save_root = scene_root.duplicate(7) as Node
    if save_root == null:
        _send_error(peer, id, "duplicate_failed", "Cannot duplicate scene root")
        return
    _set_owner_recursive(save_root, save_root)
    var packed := PackedScene.new()
    var pack_err := packed.pack(save_root)
    if pack_err != OK:
        save_root.free()
        _send_error(peer, id, "pack_failed", "PackedScene.pack failed: %s" % pack_err)
        return
    packed.take_over_path(out_path)
    var save_err := ResourceSaver.save(packed, out_path)
    save_root.free()
    if save_err != OK:
        _send_error(peer, id, "save_failed", "ResourceSaver.save failed: %s" % save_err)
        return
    scene_out = out_path
    _send_result(peer, id, { "out": out_path })

func _rpc_capture(peer: StreamPeerTCP, id: String, params: Dictionary) -> void:
    if not pending_capture.is_empty():
        _send_error(peer, id, "capture_busy", "A capture request is already pending")
        return
    var out_path := str(params.get("out", ""))
    if out_path == "":
        _send_error(peer, id, "missing_out", "capture requires out")
        return
    pending_capture = {
        "peer": peer,
        "id": id,
        "out": out_path,
        "frames": int(params.get("frames", 10)),
        "armed": false,
    }

func _tick_capture() -> void:
    if pending_capture.is_empty():
        return
    if int(pending_capture["frames"]) > 0:
        pending_capture["frames"] = int(pending_capture["frames"]) - 1
        return
    if bool(pending_capture["armed"]):
        return
    pending_capture["armed"] = true
    _finish_capture()

func _finish_capture() -> void:
    await RenderingServer.frame_post_draw
    var peer: StreamPeerTCP = pending_capture["peer"]
    var id: String = pending_capture["id"]
    var out_path: String = pending_capture["out"]
    var img := get_viewport().get_texture().get_image()
    var err := img.save_png(out_path)
    if err != OK:
        _send_error(peer, id, "save_png_failed", "Cannot save PNG: %s" % out_path)
    else:
        _send_result(peer, id, { "capture": out_path })
    pending_capture = {}

func _resolve_node(path: String) -> Node:
    if scene_root == null:
        return null
    if path == "" or path == "/":
        return scene_root
    var clean := path
    if clean.begins_with("/"):
        clean = clean.substr(1)
    return scene_root.get_node_or_null(NodePath(clean))

func _node_to_dict(node: Node, path: String) -> Dictionary:
    var children: Array = []
    for child in node.get_children():
        children.append(_node_to_dict(child, _path_for_node(child)))
    return {
        "path": path,
        "name": node.name,
        "type": node.get_class(),
        "children": children,
    }

func _path_for_node(node: Node) -> String:
    if node == scene_root:
        return "/"
    var rel := scene_root.get_path_to(node)
    return "/" + str(rel)

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
            return null

func _send_result(peer: StreamPeerTCP, id: String, result: Dictionary) -> void:
    _send(peer, { "ok": true, "id": id, "result": result })

func _send_error(peer: StreamPeerTCP, id: String, code: String, message: String) -> void:
    _send(peer, { "ok": false, "id": id, "error": code, "message": message })

func _send(peer: StreamPeerTCP, payload: Dictionary) -> void:
    var line := JSON.stringify(payload) + "\n"
    peer.put_data(line.to_utf8_buffer())
    peer.disconnect_from_host()

func _fatal(code: String, message: String) -> void:
    push_error(message)
    print(JSON.stringify({ "ok": false, "error": code, "message": message }))
    get_tree().quit(1)
