use serde_json::json;

use crate::commands::Cli;
use crate::error::{GdxError, GdxResult};
use crate::godot;
use crate::project::godot_path_string;

pub fn run(cli: &Cli) -> GdxResult<serde_json::Value> {
    let binary = godot::locate_godot(cli.godot.as_deref())?;
    let version = godot::run_version(&binary, 10)?;
    if !version.starts_with('4') {
        return Err(GdxError::validation(
            "unsupported_godot_version",
            format!("Expected Godot 4.x, got {version}"),
        ));
    }

    let os = std::env::consts::OS;
    let display_detected = match os {
        "linux" => {
            std::env::var_os("DISPLAY").is_some() || std::env::var_os("WAYLAND_DISPLAY").is_some()
        }
        _ => true,
    };
    let suggestion = if os == "linux" && !display_detected {
        Some("Runtime capture may fail; run in a desktop session or wrap Godot with xvfb-run -a.")
    } else {
        None
    };

    Ok(json!({
        "ok": true,
        "command": "doctor",
        "godot": {
            "path": godot_path_string(&binary),
            "version": version
        },
        "platform": os,
        "runtime_capture": {
            "requires_display": true,
            "display_detected": display_detected,
            "suggestion": suggestion
        },
        "env": {
            "GDX_GODOT": std::env::var("GDX_GODOT").ok()
        }
    }))
}
