use std::io::{self, Write};

use serde::Serialize;
use serde_json::json;

use crate::error::GdxError;
use crate::progress::ResolvedProgressMode;

pub fn emit_ok<T: Serialize>(value: T) {
    let text = serde_json::to_string_pretty(&value).expect("success output must serialize");
    let mut stdout = io::stdout().lock();
    writeln!(stdout, "{text}").expect("success output must write");
    stdout.flush().expect("success output must flush");
}

pub fn emit_err(error: GdxError, progress_mode: ResolvedProgressMode) -> ! {
    let mut value = json!({
        "ok": false,
        "error": error.error,
        "message": error.message,
    });

    if let Some(command) = error.context.command {
        value["command"] = json!(command);
    }
    if let Some(run_id) = error.context.run_id {
        value["run_id"] = json!(run_id);
    }
    if let Some(stage) = error.context.stage {
        value["stage"] = json!(stage);
    }
    if let Some(subject) = error.context.subject {
        value["subject"] = json!(subject);
    }
    if let Some(child_pid) = error.context.child_pid {
        value["child_pid"] = json!(child_pid);
    }
    if let Some(elapsed_ms) = error.context.elapsed_ms {
        value["elapsed_ms"] = json!(elapsed_ms);
        value["duration_ms"] = json!(elapsed_ms);
        value["timing"] = json!({ "total_ms": elapsed_ms });
    }
    value["progress_mode"] = json!(progress_mode.as_str());
    if let Some(timeout_secs) = error.context.timeout_secs {
        value["timeout_secs"] = json!(timeout_secs);
    }
    if let Some(suggestion) = error.context.suggestion {
        value["suggestion"] = json!(suggestion);
    }
    if !error.context.artifacts.is_empty() {
        value["artifacts"] = json!(error.context.artifacts);
    }
    if let Some(details) = error.context.details {
        if let Some(cleanup) = details.get("cleanup") {
            value["cleanup"] = cleanup.clone();
        }
        if let Some(log_summary) = details.get("log_summary") {
            value["log_summary"] = log_summary.clone();
        }
        value["details"] = details;
    }

    let text = if progress_mode == ResolvedProgressMode::Ndjson {
        serde_json::to_string(&value)
    } else {
        serde_json::to_string_pretty(&value)
    }
    .expect("error output must serialize");
    let mut stderr = io::stderr().lock();
    writeln!(stderr, "{text}").expect("error output must write");
    stderr.flush().expect("error output must flush");
    std::process::exit(error.exit_code);
}
