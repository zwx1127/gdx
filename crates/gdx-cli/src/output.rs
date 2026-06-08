use serde::Serialize;
use serde_json::json;

use crate::error::GdxError;

pub fn emit_ok<T: Serialize>(value: T) {
    println!(
        "{}",
        serde_json::to_string_pretty(&value).expect("success output must serialize")
    );
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

    eprintln!(
        "{}",
        serde_json::to_string_pretty(&value).expect("error output must serialize")
    );
    std::process::exit(error.exit_code);
}
