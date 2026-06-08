extends RefCounted

func run_tests() -> Dictionary:
    return {
        "ok": true,
        "checks": ["script_loaded", "json_result"]
    }
