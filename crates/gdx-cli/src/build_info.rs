use serde_json::{json, Value};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const COMMIT: &str = env!("GDX_BUILD_COMMIT");

pub fn enrich(value: &mut Value) {
    if !value.is_object() {
        return;
    }
    if !value.get("gdx").is_some_and(Value::is_object) {
        value["gdx"] = json!({});
    }
    value["gdx"]["executable_path"] = current_executable();
    value["gdx"]["version"] = json!(VERSION);
    value["gdx"]["build_commit"] = json!(COMMIT);
}

fn current_executable() -> Value {
    std::env::current_exe()
        .ok()
        .map(|path| crate::project::godot_path_string(&path))
        .map(Value::String)
        .unwrap_or(Value::Null)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provenance_merges_into_existing_gdx_object() {
        let mut value = json!({ "ok": true, "gdx": { "installed": true } });
        enrich(&mut value);

        assert_eq!(value["gdx"]["installed"], true);
        assert_eq!(value["gdx"]["version"], VERSION);
        assert!(value["gdx"]["executable_path"].is_string());
    }
}
