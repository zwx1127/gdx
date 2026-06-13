use std::fs;
use std::path::Path;

use serde_json::{json, Value};

use crate::error::GdxError;
use crate::project::godot_path_string;

const MAX_TAIL_LINES: usize = 40;

pub fn attach_log_diagnostics(
    mut error: GdxError,
    stdout_log: &Path,
    stderr_log: &Path,
) -> GdxError {
    let diagnostics = diagnose_logs(stdout_log, stderr_log);
    if diagnostics != Value::Null {
        let details = match error.details.take() {
            Some(mut existing) if existing.is_object() => {
                existing["diagnostics"] = diagnostics;
                existing
            }
            Some(existing) => json!({
                "context": existing,
                "diagnostics": diagnostics
            }),
            None => json!({
                "diagnostics": diagnostics
            }),
        };
        error = error.with_details(details);
    }
    error
}

pub fn diagnose_logs(stdout_log: &Path, stderr_log: &Path) -> Value {
    let stdout_tail = read_tail(stdout_log);
    let stderr_tail = read_tail(stderr_log);
    let combined = format!("{stdout_tail}\n{stderr_tail}");
    let primary = classify(&combined);

    if primary.is_none() && stdout_tail.is_empty() && stderr_tail.is_empty() {
        return Value::Null;
    }

    json!({
        "primary_error": primary.unwrap_or("unknown_godot_failure"),
        "stdout_log": godot_path_string(stdout_log),
        "stderr_log": godot_path_string(stderr_log),
        "stdout_tail": stdout_tail,
        "stderr_tail": stderr_tail,
    })
}

pub fn classify_logs(stdout_log: &Path, stderr_log: &Path) -> Option<&'static str> {
    let stdout_tail = read_tail(stdout_log);
    let stderr_tail = read_tail(stderr_log);
    let combined = format!("{stdout_tail}\n{stderr_tail}");
    classify(&combined)
}

pub fn read_tail(path: &Path) -> String {
    let text = fs::read_to_string(path).unwrap_or_default();
    if text.trim().is_empty() {
        return String::new();
    }
    let lines: Vec<&str> = text.lines().collect();
    let start = lines.len().saturating_sub(MAX_TAIL_LINES);
    lines[start..].join("\n")
}

fn classify(text: &str) -> Option<&'static str> {
    let lower = text.to_lowercase();
    if lower.contains("crashhandlerexception")
        || lower.contains("program crashed with signal 11")
        || lower.contains("0xc0000005")
    {
        Some("godot_native_crash")
    } else if lower.contains("warning treated as error")
        || lower.contains("cannot infer the type")
        || lower.contains("will be typed as variant")
    {
        Some("gdscript_warning_as_error")
    } else if lower.contains("parse error") || lower.contains("parser error") {
        Some("gdscript_parse_error")
    } else if lower.contains("scene_load_failed") || lower.contains("cannot load scene") {
        Some("scene_load_failed")
    } else if lower.contains("script_load_failed") || lower.contains("cannot load script") {
        Some("script_load_failed")
    } else if lower.contains("cannot connect to daemon") || lower.contains("daemon_connect_failed")
    {
        Some("daemon_connect_failed")
    } else if lower.contains("exited early") || lower.contains("daemon_exited") {
        Some("daemon_exited")
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_warning_as_error() {
        let text =
            "Cannot infer the type of \"content_bottom\" variable.\nWarning treated as error.";

        assert_eq!(classify(text), Some("gdscript_warning_as_error"));
    }

    #[test]
    fn classifies_parse_error() {
        assert_eq!(
            classify("Parse Error: Unexpected identifier"),
            Some("gdscript_parse_error")
        );
    }

    #[test]
    fn classifies_native_crash() {
        assert_eq!(
            classify("CrashHandlerException: Program crashed with signal 11"),
            Some("godot_native_crash")
        );
        assert_eq!(
            classify("Godot exited with status 0xc0000005"),
            Some("godot_native_crash")
        );
    }
}
