use std::io::{self, Write};

use serde::Serialize;
use serde_json::json;

use crate::error::GdxError;

pub fn emit_ok<T: Serialize>(value: T) {
    let text = serde_json::to_string_pretty(&value).expect("success output must serialize");
    let mut stdout = io::stdout().lock();
    writeln!(stdout, "{text}").expect("success output must write");
    stdout.flush().expect("success output must flush");
}

pub fn emit_err(error: GdxError) -> ! {
    let mut value = json!({
        "ok": false,
        "error": error.error,
        "message": error.message,
    });

    if let Some(suggestion) = error.suggestion {
        value["suggestion"] = json!(suggestion);
    }
    if !error.artifacts.is_empty() {
        value["artifacts"] = json!(error.artifacts);
    }
    if let Some(details) = error.details {
        value["details"] = details;
    }

    let text = serde_json::to_string_pretty(&value).expect("error output must serialize");
    let mut stderr = io::stderr().lock();
    writeln!(stderr, "{text}").expect("error output must write");
    stderr.flush().expect("error output must flush");
    std::process::exit(error.exit_code);
}
