use std::io::{self, Write};

use serde::Serialize;
use serde_json::{json, Value};

use crate::error::GdxError;
pub fn emit_ok<T: Serialize>(value: &T) {
    let text = serde_json::to_string(&value).expect("success output must serialize");
    let mut stdout = io::stdout().lock();
    writeln!(stdout, "{text}").expect("success output must write");
    stdout.flush().expect("success output must flush");
}

pub fn emit_err(error: GdxError) -> ! {
    let exit_code = error.exit_code;
    let value = error_value(&error);
    emit_err_value(&value, exit_code)
}

pub fn error_value(error: &GdxError) -> Value {
    let mut value = json!({
        "ok": false,
        "error": error.error,
        "message": error.message,
    });

    if let Some(command) = &error.context.command {
        value["command"] = json!(command);
    }
    if let Some(run_id) = &error.context.run_id {
        value["run_id"] = json!(run_id);
    }
    if let Some(stage) = &error.context.stage {
        value["stage"] = json!(stage);
    }
    if let Some(subject) = &error.context.subject {
        value["subject"] = json!(subject);
    }
    if let Some(child_pid) = error.context.child_pid {
        value["child_pid"] = json!(child_pid);
    }
    if let Some(elapsed_ms) = error.context.elapsed_ms {
        value["elapsed_ms"] = json!(elapsed_ms);
    }
    if let Some(timeout_secs) = error.context.timeout_secs {
        value["timeout_secs"] = json!(timeout_secs);
    }
    if let Some(suggestion) = &error.context.suggestion {
        value["suggestion"] = json!(suggestion);
    }
    if let Some(details) = &error.context.details {
        value["details"] = details.clone();
    }
    value
}

pub fn emit_err_value(value: &Value, exit_code: i32) -> ! {
    let text = serde_json::to_string(value).expect("error output must serialize");
    let mut stderr = io::stderr().lock();
    writeln!(stderr, "{text}").expect("error output must write");
    stderr.flush().expect("error output must flush");
    std::process::exit(exit_code);
}
