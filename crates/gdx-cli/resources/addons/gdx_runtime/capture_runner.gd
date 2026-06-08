extends Node

var frames_left := 10
var capture_out := ""
var started_capture := false

func _ready() -> void:
    var target_scene := OS.get_environment("GDX_TARGET_SCENE")
    capture_out = OS.get_environment("GDX_CAPTURE_OUT")
    var frames_env := OS.get_environment("GDX_CAPTURE_FRAMES")
    if frames_env != "":
        frames_left = int(frames_env)

    if target_scene == "" or capture_out == "":
        _fail("missing_env", "GDX_TARGET_SCENE and GDX_CAPTURE_OUT are required")
        return

    var packed = load(target_scene)
    if packed == null:
        _fail("scene_load_failed", "Cannot load scene: %s" % target_scene)
        return

    var inst = packed.instantiate()
    add_child(inst)

func _process(_delta: float) -> void:
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

func _fail(code: String, message: String) -> void:
    push_error(message)
    print(JSON.stringify({ "ok": false, "error": code, "message": message }))
    get_tree().quit(1)
