extends Node

const RUNTIME_VERSION := "0.1.1"
const PROTOCOL_VERSION := 1
const SUPPORTED_METHODS := [
    "ping",
    "shutdown",
    "capabilities",
    "scene_tree",
    "add_node",
    "set_property",
    "save_scene",
    "capture",
    "input_event",
    "input_click",
    "touch_sequence",
    "click_node",
    "activate_node",
    "call_method",
    "get_state",
]

var server := TCPServer.new()
var clients: Array[Dictionary] = []
var token := ""
var target_scene := ""
var scene_out := ""
var scene_root: Node = null
var pending_capture: Dictionary = {}
var pending_click: Dictionary = {}
var pending_touch_sequence: Dictionary = {}

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

    print(JSON.stringify({
        "ok": true,
        "event": "daemon_ready",
        "port": port,
        "runtime": _runtime_info(),
    }))

func _process(_delta: float) -> void:
    _accept_clients()
    _read_clients()
    _tick_click()
    _tick_touch_sequence()
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
            _send_result(peer, id, { "pong": true, "runtime": _runtime_info() })
        "capabilities":
            _send_result(peer, id, _runtime_info())
        "shutdown":
            _send_result(peer, id, { "shutdown": true })
            get_tree().quit(0)
        "scene_tree":
            _rpc_scene_tree(peer, id, params)
        "add_node":
            _rpc_add_node(peer, id, params)
        "set_property":
            _rpc_set_property(peer, id, params)
        "save_scene":
            _rpc_save_scene(peer, id, params)
        "capture":
            _rpc_capture(peer, id, params)
        "input_event":
            _rpc_input_event(peer, id, params)
        "input_click":
            _rpc_input_click(peer, id, params)
        "touch_sequence":
            _rpc_touch_sequence(peer, id, params)
        "click_node":
            _rpc_click_node(peer, id, params)
        "activate_node":
            _rpc_activate_node(peer, id, params)
        "call_method":
            _rpc_call_method(peer, id, params)
        "get_state":
            _rpc_get_state(peer, id, params)
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

func _rpc_scene_tree(peer: StreamPeerTCP, id: String, params: Dictionary) -> void:
    if scene_root == null:
        _send_error(peer, id, "scene_not_loaded", "No target scene loaded")
        return
    var options := {
        "include_script": bool(params.get("include_script", false)),
        "include_groups": bool(params.get("include_groups", false)),
        "include_methods": bool(params.get("include_methods", false)),
        "method_prefix": str(params.get("method_prefix", "gdx_")),
    }
    _send_result(peer, id, _node_to_dict(scene_root, "/", options))

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

func _rpc_input_event(peer: StreamPeerTCP, id: String, params: Dictionary) -> void:
    var kind := str(params.get("kind", ""))
    match kind:
        "key":
            var key := InputEventKey.new()
            key.keycode = int(params.get("keycode", 0))
            key.pressed = bool(params.get("pressed", true))
            Input.parse_input_event(key)
        "mouse_button":
            _send_mouse_button(
                int(params.get("button", 1)),
                _to_variant({ "vec2": params.get("position", [0, 0]) }),
                bool(params.get("pressed", true))
            )
        "mouse_motion":
            _send_mouse_motion(_to_variant({ "vec2": params.get("position", [0, 0]) }))
        _:
            _send_error(peer, id, "unknown_input_kind", "Unknown input kind: %s" % kind)
            return
    _send_result(peer, id, { "kind": kind })

func _rpc_input_click(peer: StreamPeerTCP, id: String, params: Dictionary) -> void:
    if not pending_click.is_empty() or not pending_touch_sequence.is_empty():
        _send_error(peer, id, "input_busy", "A click request is already pending")
        return
    var button := int(params.get("button", 1))
    if button <= 0:
        _send_error(peer, id, "invalid_button", "button must be greater than zero")
        return
    var position: Vector2 = _to_variant({ "vec2": params.get("position", [0, 0]) })
    var frames := int(params.get("frames", 2))
    if frames < 0:
        frames = 0
    _send_mouse_motion(position)
    _send_mouse_button(button, position, true)
    pending_click = {
        "peer": peer,
        "id": id,
        "button": button,
        "position": position,
        "frames": frames,
        "frames_left": frames,
        "phase": "release",
        "before": _ui_context(),
    }

func _rpc_touch_sequence(peer: StreamPeerTCP, id: String, params: Dictionary) -> void:
    if not pending_click.is_empty() or not pending_touch_sequence.is_empty():
        _send_error(peer, id, "input_busy", "An input request is already pending")
        return
    var events = params.get("events", [])
    if typeof(events) != TYPE_ARRAY:
        _send_error(peer, id, "invalid_touch_sequence", "events must be an array")
        return
    if events.is_empty():
        _send_error(peer, id, "invalid_touch_sequence", "events must not be empty")
        return
    pending_touch_sequence = {
        "peer": peer,
        "id": id,
        "events": events,
        "cursor": 0,
        "wait_frames": 0,
        "active": {},
        "before": _touch_context({}),
    }

func _rpc_click_node(peer: StreamPeerTCP, id: String, params: Dictionary) -> void:
    if not pending_click.is_empty() or not pending_touch_sequence.is_empty():
        _send_error(peer, id, "input_busy", "A click request is already pending")
        return
    var target_path := str(params.get("target", ""))
    var target := _resolve_node(target_path)
    if target == null:
        _send_error(peer, id, "target_not_found", "Target not found: %s" % target_path)
        return
    var position = _click_position_for_node(target)
    if position == null:
        _send_error(peer, id, "target_not_clickable", "Target has no screen position: %s" % target_path)
        return
    var button := int(params.get("button", 1))
    if button <= 0:
        _send_error(peer, id, "invalid_button", "button must be greater than zero")
        return
    var frames := int(params.get("frames", 2))
    if frames < 0:
        frames = 0
    _send_mouse_motion(position)
    _send_mouse_button(button, position, true)
    pending_click = {
        "peer": peer,
        "id": id,
        "button": button,
        "position": position,
        "frames": frames,
        "frames_left": frames,
        "phase": "release",
        "target": target_path,
        "before": _ui_context(),
    }

func _rpc_activate_node(peer: StreamPeerTCP, id: String, params: Dictionary) -> void:
    var target_path := str(params.get("target", ""))
    var target := _resolve_node(target_path)
    if target == null:
        _send_error(peer, id, "target_not_found", "Target not found: %s" % target_path)
        return
    var before := _ui_context()
    if target is BaseButton:
        target.emit_signal("pressed")
        _send_result(peer, id, {
            "target": target_path,
            "kind": "activate",
            "before": before,
            "after": _ui_context(),
        })
        return
    _send_error(peer, id, "target_not_activatable", "Target is not a BaseButton: %s" % target_path)

func _rpc_call_method(peer: StreamPeerTCP, id: String, params: Dictionary) -> void:
    var target_path := str(params.get("target", ""))
    var method := str(params.get("method", ""))
    if method == "":
        _send_error(peer, id, "missing_method", "method is required")
        return
    var target := _resolve_target(target_path)
    if target == null:
        _send_error(peer, id, "target_not_found", "Target not found: %s" % target_path)
        return
    if not target.has_method(method):
        _send_error(peer, id, "method_not_found", "Method not found: %s" % method, {
            "target": target_path,
            "method": method,
            "candidates": _method_candidates(method),
        })
        return
    var args: Array = params.get("args", [])
    var converted: Array = []
    for arg in args:
        converted.append(_to_variant(arg))
    var result = target.callv(method, converted)
    _send_result(peer, id, { "target": target_path, "method": method, "result": _json_safe(result) })

func _rpc_get_state(peer: StreamPeerTCP, id: String, params: Dictionary) -> void:
    var target_path := str(params.get("target", ""))
    var target := _resolve_target(target_path)
    if target == null:
        _send_error(peer, id, "target_not_found", "Target not found: %s" % target_path)
        return
    var raw_method = params.get("method", "gdx_state")
    var method := "gdx_state"
    if raw_method != null and str(raw_method) != "":
        method = str(raw_method)
    if target.has_method(method):
        _send_result(peer, id, {
            "target": target_path,
            "source": "method",
            "method": method,
            "state": _json_safe(target.call(method)),
        })
        return
    var raw_property = params.get("property", "")
    var property := ""
    if raw_property != null:
        property = str(raw_property)
    if property != "":
        _send_result(peer, id, {
            "target": target_path,
            "source": "property",
            "property": property,
            "state": _json_safe(target.get(property)),
        })
        return
    _send_error(peer, id, "state_not_available", "Target has no state method and no property was requested", {
        "target": target_path,
        "method": method,
    })

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

func _tick_click() -> void:
    if pending_click.is_empty():
        return
    if int(pending_click["frames_left"]) > 0:
        pending_click["frames_left"] = int(pending_click["frames_left"]) - 1
        return
    var phase := str(pending_click["phase"])
    if phase == "release":
        _send_mouse_button(
            int(pending_click["button"]),
            pending_click["position"],
            false
        )
        pending_click["phase"] = "finish"
        pending_click["frames_left"] = int(pending_click["frames"])
        return
    var peer: StreamPeerTCP = pending_click["peer"]
    var id: String = pending_click["id"]
    _send_result(peer, id, {
        "kind": "click",
        "button": pending_click["button"],
        "position": _json_safe(pending_click["position"]),
        "target": str(pending_click.get("target", "")),
        "before": pending_click["before"],
        "after": _ui_context(),
    })
    pending_click = {}

func _tick_touch_sequence() -> void:
    if pending_touch_sequence.is_empty():
        return
    if int(pending_touch_sequence["wait_frames"]) > 0:
        pending_touch_sequence["wait_frames"] = int(pending_touch_sequence["wait_frames"]) - 1
        return
    while int(pending_touch_sequence["wait_frames"]) <= 0:
        var cursor := int(pending_touch_sequence["cursor"])
        var events: Array = pending_touch_sequence["events"]
        if cursor >= events.size():
            var result_peer: StreamPeerTCP = pending_touch_sequence["peer"]
            var result_id: String = pending_touch_sequence["id"]
            var active: Dictionary = pending_touch_sequence["active"]
            _send_result(result_peer, result_id, {
                "kind": "touch_sequence",
                "events": events.size(),
                "before": pending_touch_sequence["before"],
                "after": _touch_context(active),
            })
            pending_touch_sequence = {}
            return
        pending_touch_sequence["cursor"] = cursor + 1
        var error := _process_touch_sequence_event(events[cursor])
        if error != "":
            var error_peer: StreamPeerTCP = pending_touch_sequence["peer"]
            var error_id: String = pending_touch_sequence["id"]
            pending_touch_sequence = {}
            _send_error(error_peer, error_id, "invalid_touch_sequence", error)
            return
        if int(pending_touch_sequence["wait_frames"]) > 0:
            return

func _process_touch_sequence_event(raw_event) -> String:
    if typeof(raw_event) != TYPE_DICTIONARY:
        return "touch sequence events must be objects"
    var event: Dictionary = raw_event
    var kind := str(event.get("kind", ""))
    match kind:
        "wait":
            var frames := int(event.get("frames", 0))
            if frames < 0:
                frames = 0
            pending_touch_sequence["wait_frames"] = frames
        "touch":
            var touch_index := int(event.get("index", 0))
            if touch_index < 0:
                return "touch index must be greater than or equal to zero"
            var touch_position: Vector2 = _to_variant({ "vec2": event.get("position", [0, 0]) })
            var touch_pressed := bool(event.get("pressed", true))
            _send_screen_touch(touch_index, touch_position, touch_pressed)
            var touch_active: Dictionary = pending_touch_sequence["active"]
            if touch_pressed:
                touch_active[touch_index] = touch_position
            else:
                touch_active.erase(touch_index)
            pending_touch_sequence["active"] = touch_active
        "drag":
            var drag_index := int(event.get("index", 0))
            if drag_index < 0:
                return "touch index must be greater than or equal to zero"
            var drag_position: Vector2 = _to_variant({ "vec2": event.get("position", [0, 0]) })
            var drag_relative: Vector2 = _to_variant({ "vec2": event.get("relative", [0, 0]) })
            _send_screen_drag(drag_index, drag_position, drag_relative)
            var drag_active: Dictionary = pending_touch_sequence["active"]
            drag_active[drag_index] = drag_position
            pending_touch_sequence["active"] = drag_active
        _:
            return "unknown touch event kind: %s" % kind
    return ""

func _send_mouse_motion(position: Vector2) -> void:
    if get_viewport().has_method("warp_mouse"):
        get_viewport().call("warp_mouse", position)
    var motion := InputEventMouseMotion.new()
    motion.position = position
    motion.global_position = position
    Input.parse_input_event(motion)

func _send_mouse_button(button: int, position: Vector2, pressed: bool) -> void:
    var mouse := InputEventMouseButton.new()
    mouse.button_index = button
    mouse.position = position
    mouse.global_position = position
    mouse.pressed = pressed
    Input.parse_input_event(mouse)

func _send_screen_touch(index: int, position: Vector2, pressed: bool) -> void:
    var touch := InputEventScreenTouch.new()
    touch.index = index
    touch.position = position
    touch.pressed = pressed
    Input.parse_input_event(touch)

func _send_screen_drag(index: int, position: Vector2, relative: Vector2) -> void:
    var drag := InputEventScreenDrag.new()
    drag.index = index
    drag.position = position
    drag.relative = relative
    drag.velocity = Vector2.ZERO
    Input.parse_input_event(drag)

func _touch_context(active: Dictionary) -> Dictionary:
    var indexes: Array = active.keys()
    indexes.sort()
    var touches: Array = []
    for index in indexes:
        touches.append({
            "index": int(index),
            "position": _json_safe(active[index]),
        })
    return {
        "active_touch_indexes": indexes,
        "active_touches": touches,
    }

func _ui_context() -> Dictionary:
    var viewport := get_viewport()
    var hovered: Control = null
    var focused: Control = null
    if viewport.has_method("gui_get_hovered_control"):
        hovered = viewport.call("gui_get_hovered_control")
    if viewport.has_method("gui_get_focus_owner"):
        focused = viewport.call("gui_get_focus_owner")
    return {
        "mouse_position": _json_safe(viewport.get_mouse_position()),
        "hovered": _node_debug_path(hovered),
        "focused": _node_debug_path(focused),
    }

func _click_position_for_node(node: Node):
    if node is Control:
        var control := node as Control
        return control.get_global_rect().get_center()
    if node is Node2D:
        return (node as Node2D).global_position
    if node is CanvasItem:
        var item := node as CanvasItem
        return item.get_global_transform_with_canvas().origin
    return null

func _node_debug_path(node: Node) -> String:
    if node == null:
        return ""
    if scene_root != null and (node == scene_root or scene_root.is_ancestor_of(node)):
        return _path_for_node(node)
    return str(node.get_path())

func _resolve_node(path: String) -> Node:
    if scene_root == null:
        return null
    if path == "" or path == "/":
        return scene_root
    var clean := path
    if clean.begins_with("/"):
        clean = clean.substr(1)
    return scene_root.get_node_or_null(NodePath(clean))

func _resolve_target(path: String) -> Object:
    if path.begins_with("/root/"):
        return get_tree().root.get_node_or_null(NodePath(path.substr(6)))
    return _resolve_node(path)

func _runtime_info() -> Dictionary:
    return {
        "runtime_version": RUNTIME_VERSION,
        "protocol_version": PROTOCOL_VERSION,
        "methods": SUPPORTED_METHODS.duplicate(),
        "input": {
            "key": true,
            "mouse": true,
            "touch": true,
            "multi_touch": true,
            "gestures": ["tap", "drag", "swipe", "pinch", "sequence"],
        },
    }

func _node_to_dict(node: Node, path: String, options: Dictionary = {}) -> Dictionary:
    var children: Array = []
    for child in node.get_children():
        children.append(_node_to_dict(child, _path_for_node(child), options))
    var result := {
        "path": path,
        "name": node.name,
        "type": node.get_class(),
        "children": children,
    }
    if bool(options.get("include_script", false)):
        var script = node.get_script()
        if script != null:
            result["script"] = str(script.resource_path)
            if script.has_method("get_global_name"):
                result["script_class"] = str(script.call("get_global_name"))
    if bool(options.get("include_groups", false)):
        result["groups"] = _node_groups(node)
    if bool(options.get("include_methods", false)):
        result["methods"] = _callable_methods(node, str(options.get("method_prefix", "gdx_")))
    return result

func _node_groups(node: Node) -> Array:
    var groups: Array = []
    for group in node.get_groups():
        groups.append(str(group))
    groups.sort()
    return groups

func _callable_methods(node: Object, prefix: String) -> Array:
    var methods: Array = []
    for method_info in node.get_method_list():
        var method_name := str(method_info.get("name", ""))
        if prefix == "" or method_name.begins_with(prefix):
            methods.append(method_name)
    methods.sort()
    return methods

func _method_candidates(method: String) -> Array:
    var candidates: Array = []
    if scene_root == null or method == "":
        return candidates
    _collect_method_candidates(scene_root, method, candidates)
    return candidates

func _collect_method_candidates(node: Node, method: String, candidates: Array) -> void:
    if node.has_method(method):
        candidates.append({
            "path": _path_for_node(node),
            "name": str(node.name),
            "type": node.get_class(),
        })
    for child in node.get_children():
        _collect_method_candidates(child, method, candidates)

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
        if typeof(value) == TYPE_ARRAY:
            var converted_array: Array = []
            for item in value:
                converted_array.append(_to_variant(item))
            return converted_array
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
    var converted_dict := {}
    for key in value.keys():
        converted_dict[key] = _to_variant(value[key])
    return converted_dict

func _json_safe(value):
    match typeof(value):
        TYPE_NIL, TYPE_BOOL, TYPE_INT, TYPE_FLOAT, TYPE_STRING:
            return value
        TYPE_VECTOR2:
            return { "vec2": [value.x, value.y] }
        TYPE_VECTOR3:
            return { "vec3": [value.x, value.y, value.z] }
        TYPE_COLOR:
            return { "color": [value.r, value.g, value.b, value.a] }
        TYPE_ARRAY:
            var arr: Array = []
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

func _send_error(peer: StreamPeerTCP, id: String, code: String, message: String, details: Dictionary = {}) -> void:
    var payload := { "ok": false, "id": id, "error": code, "message": message }
    if not details.is_empty():
        payload["details"] = details
    _send(peer, payload)

func _send(peer: StreamPeerTCP, payload: Dictionary) -> void:
    var line := JSON.stringify(payload) + "\n"
    peer.put_data(line.to_utf8_buffer())
    peer.disconnect_from_host()

func _fatal(code: String, message: String) -> void:
    push_error(message)
    print(JSON.stringify({ "ok": false, "error": code, "message": message }))
    get_tree().quit(1)
