extends Node

var frames_left := 10
var capture_out := ""
var started_capture := false
var record_mode := false
var input_events: Array = []
var input_cursor := 0
var input_wait_frames := 0

func _ready() -> void:
    var target_scene := OS.get_environment("GDX_TARGET_SCENE")
    capture_out = OS.get_environment("GDX_CAPTURE_OUT")
    var input_sequence_path := OS.get_environment("GDX_INPUT_SEQUENCE")
    record_mode = input_sequence_path != ""
    var frames_env := OS.get_environment("GDX_CAPTURE_FRAMES")
    if frames_env != "":
        frames_left = int(frames_env)

    if target_scene == "" or (not record_mode and capture_out == ""):
        _fail("missing_env", "GDX_TARGET_SCENE and GDX_CAPTURE_OUT are required for capture; GDX_INPUT_SEQUENCE can replace GDX_CAPTURE_OUT for recording")
        return

    if record_mode:
        input_events = _read_input_sequence(input_sequence_path)
        if input_events.is_empty():
            return

    var packed = load(target_scene)
    if packed == null:
        _fail("scene_load_failed", "Cannot load scene: %s" % target_scene)
        return

    var inst = packed.instantiate()
    add_child(inst)

func _process(_delta: float) -> void:
    if record_mode:
        _tick_input_sequence()
        return

    if started_capture:
        return

    if frames_left > 0:
        frames_left -= 1
        return

    started_capture = true
    _capture_after_draw()

func _capture_after_draw() -> void:
    await RenderingServer.frame_post_draw
    var img := get_viewport().get_texture().get_image()
    var err := img.save_png(capture_out)
    if err != OK:
        _fail("save_png_failed", "Cannot save PNG: %s" % capture_out)
        return
    print(JSON.stringify({ "ok": true, "capture": capture_out }))
    get_tree().quit(0)

func _read_input_sequence(path: String) -> Array:
    if not FileAccess.file_exists(path):
        _fail("input_sequence_not_found", "Input sequence file does not exist: %s" % path)
        return []
    var text := FileAccess.get_file_as_string(path)
    var parsed = JSON.parse_string(text)
    if typeof(parsed) != TYPE_DICTIONARY:
        _fail("invalid_input_sequence", "Input sequence must be a JSON object")
        return []
    var events = parsed.get("events", [])
    if typeof(events) != TYPE_ARRAY:
        _fail("invalid_input_sequence", "Input sequence events must be an array")
        return []
    if events.is_empty():
        _fail("invalid_input_sequence", "Input sequence events must not be empty")
        return []
    return events

func _tick_input_sequence() -> void:
    if input_cursor >= input_events.size():
        return
    if input_wait_frames > 0:
        input_wait_frames -= 1
        return
    while input_wait_frames <= 0 and input_cursor < input_events.size():
        var raw_event = input_events[input_cursor]
        input_cursor += 1
        var error := _process_input_sequence_event(raw_event)
        if error != "":
            _fail("invalid_input_sequence", error)
            return
        if input_wait_frames > 0:
            return

func _process_input_sequence_event(raw_event) -> String:
    if typeof(raw_event) != TYPE_DICTIONARY:
        return "input sequence events must be objects"
    var event: Dictionary = raw_event
    var kind := str(event.get("kind", ""))
    match kind:
        "wait":
            var frames := int(event.get("frames", 0))
            if frames < 0:
                frames = 0
            input_wait_frames = frames
        "touch":
            var index := int(event.get("index", 0))
            if index < 0:
                return "touch index must be greater than or equal to zero"
            _send_screen_touch(index, _to_vec2(event.get("position", [0, 0])), bool(event.get("pressed", true)))
        "drag":
            var index := int(event.get("index", 0))
            if index < 0:
                return "touch index must be greater than or equal to zero"
            _send_screen_drag(index, _to_vec2(event.get("position", [0, 0])), _to_vec2(event.get("relative", [0, 0])))
        _:
            return "unknown input sequence event kind: %s" % kind
    return ""

func _to_vec2(value) -> Vector2:
    if typeof(value) == TYPE_ARRAY and value.size() == 2:
        return Vector2(float(value[0]), float(value[1]))
    return Vector2.ZERO

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

func _fail(code: String, message: String) -> void:
    push_error(message)
    print(JSON.stringify({ "ok": false, "error": code, "message": message }))
    get_tree().quit(1)
