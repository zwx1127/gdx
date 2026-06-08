extends SceneTree

func _init() -> void:
    var out_path := OS.get_environment("GDX_SCENE_OUT")
    var root_type := OS.get_environment("GDX_SCENE_ROOT_TYPE")
    var root_name := OS.get_environment("GDX_SCENE_ROOT_NAME")
    var force := OS.get_environment("GDX_SCENE_FORCE") == "true"

    if out_path == "" or root_type == "" or root_name == "":
        _fail("missing_env", "GDX_SCENE_OUT, GDX_SCENE_ROOT_TYPE, and GDX_SCENE_ROOT_NAME are required")
        return

    if not force and ResourceLoader.exists(out_path):
        _fail("scene_exists", "Scene already exists: %s" % out_path)
        return

    if not ClassDB.class_exists(root_type):
        _fail("unknown_node_type", "Unknown node type: %s" % root_type)
        return

    var root = ClassDB.instantiate(root_type)
    if root == null or not (root is Node):
        _fail("invalid_node_type", "Root type must instantiate a Node: %s" % root_type)
        return

    root.name = root_name
    var packed := PackedScene.new()
    var pack_err := packed.pack(root)
    if pack_err != OK:
        root.free()
        _fail("pack_failed", "PackedScene.pack failed: %s" % pack_err)
        return

    var save_err := ResourceSaver.save(packed, out_path)
    root.free()
    if save_err != OK:
        _fail("save_failed", "ResourceSaver.save failed: %s" % save_err)
        return

    print(JSON.stringify({ "ok": true, "out": out_path, "root_type": root_type, "name": root_name }))
    quit(0)

func _fail(code: String, message: String) -> void:
    push_error(message)
    print(JSON.stringify({ "ok": false, "error": code, "message": message }))
    quit(1)
