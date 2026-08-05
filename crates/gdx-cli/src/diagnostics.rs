use std::collections::{HashMap, VecDeque};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::error::GdxError;

const MAX_TAIL_LINES: usize = 40;
const MAX_DIAGNOSTIC_CHARS: usize = 1_024;
const MAX_GROUPS: usize = 1_000;
const SUMMARY_GROUPS: usize = 20;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct LogRecord {
    pub severity: String,
    pub code: String,
    pub message: String,
    pub location: Option<String>,
    pub count: u64,
    #[serde(default = "default_diagnostic_source")]
    pub source: String,
    #[serde(default = "default_blocking")]
    pub blocking: bool,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct LogSummary {
    pub errors: u64,
    pub warnings: u64,
    pub unique_errors: usize,
    pub unique_warnings: usize,
    #[serde(default)]
    pub blocking_errors: u64,
    #[serde(default)]
    pub non_blocking_diagnostics: u64,
    pub first_error: Option<LogRecord>,
    pub first_warning: Option<LogRecord>,
    pub groups: Vec<LogRecord>,
    pub groups_truncated: bool,
}

impl LogSummary {
    pub fn merge(&mut self, other: &Self) {
        self.errors += other.errors;
        self.warnings += other.warnings;
        if self.first_error.is_none() {
            self.first_error = other.first_error.clone();
        }
        if self.first_warning.is_none() {
            self.first_warning = other.first_warning.clone();
        }
        let mut groups: Vec<LogRecord> = Vec::new();
        let mut indexes: HashMap<String, usize> = HashMap::new();
        let mut truncated = self.groups_truncated || other.groups_truncated;
        for record in self.groups.iter().chain(&other.groups) {
            let key = record_key(record);
            if let Some(index) = indexes.get(&key).copied() {
                groups[index].count += record.count;
            } else if groups.len() < MAX_GROUPS {
                indexes.insert(key, groups.len());
                groups.push(record.clone());
            } else {
                truncated = true;
            }
        }
        self.groups = groups;
        self.groups_truncated = truncated;
        self.recount_unique();
    }

    pub fn apply_project_context(&mut self, csharp_project: Option<bool>) {
        for record in &mut self.groups {
            classify_record_context(record, csharp_project);
        }
        if csharp_project == Some(false) {
            let removed_errors = self
                .groups
                .iter()
                .filter(|record| record.code == "dotnet_sdk_missing" && record.severity == "error")
                .map(|record| record.count)
                .sum::<u64>();
            let removed_warnings = self
                .groups
                .iter()
                .filter(|record| {
                    record.code == "dotnet_sdk_missing" && record.severity == "warning"
                })
                .map(|record| record.count)
                .sum::<u64>();
            self.groups
                .retain(|record| record.code != "dotnet_sdk_missing");
            self.errors = self.errors.saturating_sub(removed_errors);
            self.warnings = self.warnings.saturating_sub(removed_warnings);
        }
        self.first_error = self
            .groups
            .iter()
            .find(|record| record.severity == "error")
            .cloned();
        self.first_warning = self
            .groups
            .iter()
            .find(|record| record.severity == "warning")
            .cloned();
        self.recount_unique();
    }

    pub fn bounded(&self) -> Self {
        let mut value = self.clone();
        value.groups = value.groups.into_iter().take(SUMMARY_GROUPS).collect();
        value.groups_truncated |= value.groups.len() < self.groups.len();
        value
    }

    pub fn first_hard_error(&self) -> Option<&LogRecord> {
        self.groups
            .iter()
            .find(|record| record.severity == "error" && record.blocking)
    }

    fn recount_unique(&mut self) {
        self.unique_errors = self
            .groups
            .iter()
            .filter(|record| record.severity == "error")
            .count();
        self.unique_warnings = self
            .groups
            .iter()
            .filter(|record| record.severity == "warning")
            .count();
        self.blocking_errors = self
            .groups
            .iter()
            .filter(|record| record.severity == "error" && record.blocking)
            .map(|record| record.count)
            .sum();
        self.non_blocking_diagnostics = self
            .groups
            .iter()
            .filter(|record| !record.blocking)
            .map(|record| record.count)
            .sum();
    }
}

fn default_diagnostic_source() -> String {
    "godot".to_string()
}

fn default_blocking() -> bool {
    true
}

fn classify_record_context(record: &mut LogRecord, csharp_project: Option<bool>) {
    record.source = match record.code.as_str() {
        "dotnet_sdk_missing" | "filesystem_unavailable" => "environment",
        "gdscript_warning_as_error"
        | "gdscript_parse_error"
        | "scene_load_failed"
        | "script_load_failed" => "project",
        _ => "godot",
    }
    .to_string();
    record.blocking = record.code != "dotnet_sdk_missing" || csharp_project != Some(false);
}

pub fn attach_log_diagnostics(
    mut error: GdxError,
    stdout_log: &Path,
    stderr_log: &Path,
) -> GdxError {
    let diagnostics = diagnose_logs_with_context(stdout_log, stderr_log, None);
    if diagnostics != Value::Null {
        error.insert_detail("diagnostics", diagnostics);
    }
    error
}

pub fn attach_log_diagnostics_with_context(
    mut error: GdxError,
    stdout_log: &Path,
    stderr_log: &Path,
    csharp_project: Option<bool>,
) -> GdxError {
    let diagnostics = diagnose_logs_with_context(stdout_log, stderr_log, csharp_project);
    if diagnostics != Value::Null {
        error.insert_detail("diagnostics", diagnostics);
    }
    error
}

#[cfg(test)]
pub fn diagnose_logs(stdout_log: &Path, stderr_log: &Path) -> Value {
    diagnose_logs_with_context(stdout_log, stderr_log, None)
}

fn diagnose_logs_with_context(
    stdout_log: &Path,
    stderr_log: &Path,
    csharp_project: Option<bool>,
) -> Value {
    let stdout_tail = read_tail_with_context(stdout_log, csharp_project);
    let stderr_tail = read_tail_with_context(stderr_log, csharp_project);
    let mut summary = summarize_logs(stdout_log, stderr_log);
    summary.apply_project_context(csharp_project);
    if summary.errors == 0
        && summary.warnings == 0
        && stdout_tail.is_empty()
        && stderr_tail.is_empty()
    {
        return Value::Null;
    }
    let primary = summary
        .first_error
        .as_ref()
        .map(|record| record.code.as_str())
        .unwrap_or("unknown_godot_failure");
    json!({
        "primary_error": primary,
        "stdout_tail": stdout_tail,
        "stderr_tail": stderr_tail,
        "log_summary": summary.bounded(),
    })
}

pub fn summarize_logs(stdout_log: &Path, stderr_log: &Path) -> LogSummary {
    let mut summary = summarize_file(stdout_log);
    summary.merge(&summarize_file(stderr_log));
    summary
}

pub fn classify_logs(stdout_log: &Path, stderr_log: &Path) -> Option<&'static str> {
    let summary = summarize_logs(stdout_log, stderr_log);
    summary
        .first_error
        .as_ref()
        .map(|record| static_code(&record.code))
}

#[cfg(test)]
pub fn read_tail(path: &Path) -> String {
    read_tail_with_context(path, None)
}

fn read_tail_with_context(path: &Path, csharp_project: Option<bool>) -> String {
    let Ok(file) = File::open(path) else {
        return String::new();
    };
    let mut lines = VecDeque::with_capacity(MAX_TAIL_LINES);
    let mut suppress_dotnet_location = false;
    for line in BufReader::new(file).lines().map_while(Result::ok) {
        let trimmed = line.trim_end();
        if csharp_project == Some(false) {
            if classify_message(trimmed) == "dotnet_sdk_missing" {
                suppress_dotnet_location = true;
                continue;
            }
            if suppress_dotnet_location && trimmed.trim_start().starts_with("at:") {
                suppress_dotnet_location = false;
                continue;
            }
            suppress_dotnet_location = false;
        }
        if lines.len() == MAX_TAIL_LINES {
            lines.pop_front();
        }
        lines.push_back(truncate_text(trimmed, MAX_DIAGNOSTIC_CHARS));
    }
    lines.into_iter().collect::<Vec<_>>().join("\n")
}

fn summarize_file(path: &Path) -> LogSummary {
    let Ok(file) = File::open(path) else {
        return LogSummary::default();
    };
    let mut records: Vec<LogRecord> = Vec::new();
    let mut indexes: HashMap<String, usize> = HashMap::new();
    let mut groups_truncated = false;
    let mut first_error = None;
    let mut first_warning = None;
    let mut errors = 0;
    let mut warnings = 0;
    let mut current: Option<LogRecord> = None;

    let flush = |current: &mut Option<LogRecord>,
                 records: &mut Vec<LogRecord>,
                 indexes: &mut HashMap<String, usize>,
                 groups_truncated: &mut bool,
                 first_error: &mut Option<LogRecord>,
                 first_warning: &mut Option<LogRecord>,
                 errors: &mut u64,
                 warnings: &mut u64| {
        let Some(mut record) = current.take() else {
            return;
        };
        record.count = 1;
        if record.severity == "error" {
            *errors += 1;
            if first_error.is_none() {
                *first_error = Some(record.clone());
            }
        } else {
            *warnings += 1;
            if first_warning.is_none() {
                *first_warning = Some(record.clone());
            }
        }
        let key = record_key(&record);
        if let Some(index) = indexes.get(&key).copied() {
            records[index].count += 1;
        } else if records.len() < MAX_GROUPS {
            indexes.insert(key, records.len());
            records.push(record);
        } else {
            *groups_truncated = true;
        }
    };

    for line in BufReader::new(file).lines().map_while(Result::ok) {
        if let Some((severity, message)) = record_header(&line) {
            flush(
                &mut current,
                &mut records,
                &mut indexes,
                &mut groups_truncated,
                &mut first_error,
                &mut first_warning,
                &mut errors,
                &mut warnings,
            );
            let message = truncate_text(message.trim(), MAX_DIAGNOSTIC_CHARS);
            current = Some(LogRecord {
                severity: severity.to_string(),
                code: classify_message(&message).to_string(),
                message,
                location: None,
                count: 1,
                source: "godot".to_string(),
                blocking: true,
            });
        } else if classify_message(&truncate_text(&line, MAX_DIAGNOSTIC_CHARS))
            == "godot_native_crash"
        {
            flush(
                &mut current,
                &mut records,
                &mut indexes,
                &mut groups_truncated,
                &mut first_error,
                &mut first_warning,
                &mut errors,
                &mut warnings,
            );
            current = Some(LogRecord {
                severity: "error".to_string(),
                code: "godot_native_crash".to_string(),
                message: truncate_text(line.trim(), MAX_DIAGNOSTIC_CHARS),
                location: None,
                count: 1,
                source: "godot".to_string(),
                blocking: true,
            });
        } else if let Some(record) = current.as_mut() {
            let trimmed = line.trim();
            if record.location.is_none() && trimmed.starts_with("at:") {
                record.location = Some(truncate_text(trimmed, MAX_DIAGNOSTIC_CHARS));
            }
        }
    }
    flush(
        &mut current,
        &mut records,
        &mut indexes,
        &mut groups_truncated,
        &mut first_error,
        &mut first_warning,
        &mut errors,
        &mut warnings,
    );

    let mut summary = LogSummary {
        errors,
        warnings,
        unique_errors: 0,
        unique_warnings: 0,
        blocking_errors: 0,
        non_blocking_diagnostics: 0,
        first_error,
        first_warning,
        groups: records,
        groups_truncated,
    };
    summary.recount_unique();
    summary
}

fn record_header(line: &str) -> Option<(&'static str, &str)> {
    if let Some(message) = line.strip_prefix("WARNING:") {
        Some(("warning", message))
    } else if let Some(message) = line.strip_prefix("SCRIPT ERROR:") {
        Some(("error", message))
    } else {
        line.strip_prefix("ERROR:")
            .map(|message| ("error", message))
    }
}

fn truncate_text(text: &str, max_chars: usize) -> String {
    let Some((end, _)) = text.char_indices().nth(max_chars) else {
        return text.to_string();
    };
    let mut value = text[..end].to_string();
    value.push('…');
    value
}

fn record_key(record: &LogRecord) -> String {
    format!(
        "{}\0{}\0{}",
        record.severity,
        record.message,
        record.location.as_deref().unwrap_or_default()
    )
}

fn classify_message(message: &str) -> &'static str {
    let lower = message.to_lowercase();
    if lower.contains(".net sdk not found")
        || lower.contains("failed to load hostfxr")
        || lower.contains("failed to get list of installed sdks")
    {
        "dotnet_sdk_missing"
    } else if lower.contains("crashhandlerexception")
        || lower.contains("program crashed with signal 11")
        || lower.contains("0xc0000005")
    {
        "godot_native_crash"
    } else if lower.contains("warning treated as error")
        || lower.contains("cannot infer the type")
        || lower.contains("will be typed as variant")
    {
        "gdscript_warning_as_error"
    } else if lower.contains("parse error") || lower.contains("parser error") {
        "gdscript_parse_error"
    } else if lower.contains("cannot load scene") || lower.contains("scene_load_failed") {
        "scene_load_failed"
    } else if lower.contains("cannot load script") || lower.contains("script_load_failed") {
        "script_load_failed"
    } else if lower.contains("failed to open") || lower.contains("permission denied") {
        "filesystem_unavailable"
    } else {
        "godot_error"
    }
}

fn static_code(code: &str) -> &'static str {
    match code {
        "dotnet_sdk_missing" => "dotnet_sdk_missing",
        "godot_native_crash" => "godot_native_crash",
        "gdscript_warning_as_error" => "gdscript_warning_as_error",
        "gdscript_parse_error" => "gdscript_parse_error",
        "scene_load_failed" => "scene_load_failed",
        "script_load_failed" => "script_load_failed",
        "filesystem_unavailable" => "filesystem_unavailable",
        _ => "godot_error",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::{BufWriter, Write};

    #[test]
    fn streams_and_deduplicates_warning_records() {
        let path = std::env::temp_dir().join(format!(
            "gdx_log_summary_{}_{}.log",
            std::process::id(),
            uuid::Uuid::new_v4().simple()
        ));
        fs::write(
            &path,
            "WARNING: repeated\n   at: load (a.cpp:1)\nWARNING: repeated\n   at: load (a.cpp:1)\nERROR: Parse Error: bad\n",
        )
        .unwrap();
        let summary = summarize_file(&path);
        assert_eq!(summary.warnings, 2);
        assert_eq!(summary.unique_warnings, 1);
        assert_eq!(summary.errors, 1);
        assert_eq!(
            summary
                .groups
                .iter()
                .find(|v| v.severity == "warning")
                .unwrap()
                .count,
            2
        );
        assert_eq!(summary.first_error.unwrap().code, "gdscript_parse_error");
        let _ = fs::remove_file(path);
    }

    #[test]
    fn dotnet_error_is_hidden_for_gdscript_and_blocking_for_csharp() {
        let summary = LogSummary {
            errors: 1,
            groups: vec![LogRecord {
                severity: "error".into(),
                code: "dotnet_sdk_missing".into(),
                message: ".NET Sdk not found".into(),
                count: 1,
                ..LogRecord::default()
            }],
            ..LogSummary::default()
        };
        let mut gdscript = summary.clone();
        gdscript.apply_project_context(Some(false));
        assert!(gdscript.first_hard_error().is_none());
        assert_eq!(gdscript.errors, 0);
        assert!(gdscript.groups.is_empty());

        let mut csharp = summary;
        csharp.apply_project_context(Some(true));
        assert!(csharp.first_hard_error().is_some());
        assert_eq!(csharp.blocking_errors, 1);
    }

    #[test]
    fn gdscript_diagnostics_hide_dotnet_tail_but_keep_other_errors() {
        let path = std::env::temp_dir().join(format!(
            "gdx_dotnet_tail_{}_{}.log",
            std::process::id(),
            uuid::Uuid::new_v4().simple()
        ));
        fs::write(
            &path,
            "ERROR: .NET SDK not found\n   at: modules/mono/mono_gd/gd_mono.cpp:1\nERROR: Parse Error: broken script\n",
        )
        .unwrap();

        let diagnostics = diagnose_logs_with_context(&path, Path::new("missing"), Some(false));

        assert_eq!(diagnostics["primary_error"], "gdscript_parse_error");
        assert!(!diagnostics["stdout_tail"]
            .as_str()
            .unwrap()
            .contains(".NET"));
        assert!(diagnostics["stdout_tail"]
            .as_str()
            .unwrap()
            .contains("Parse Error"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn dotnet_only_diagnostic_is_hidden_for_gdscript_but_retained_without_context() {
        let path = std::env::temp_dir().join(format!(
            "gdx_dotnet_only_{}_{}.log",
            std::process::id(),
            uuid::Uuid::new_v4().simple()
        ));
        fs::write(
            &path,
            "ERROR: Failed to load hostfxr\n   at: modules/mono/mono_gd/gd_mono.cpp:1\n",
        )
        .unwrap();

        let gdscript = diagnose_logs_with_context(&path, Path::new("missing"), Some(false));
        let unscoped = diagnose_logs(&path, Path::new("missing"));

        assert_eq!(gdscript, Value::Null);
        assert_eq!(unscoped["primary_error"], "dotnet_sdk_missing");
        let _ = fs::remove_file(path);
    }

    #[test]
    fn recognizes_unprefixed_native_crash_lines() {
        let path = std::env::temp_dir().join(format!(
            "gdx_native_crash_{}_{}.log",
            std::process::id(),
            uuid::Uuid::new_v4().simple()
        ));
        fs::write(&path, "CrashHandlerException: 0xC0000005\n").unwrap();

        let summary = summarize_file(&path);

        assert_eq!(summary.first_error.unwrap().code, "godot_native_crash");
        let _ = fs::remove_file(path);
    }

    #[test]
    fn bounds_diagnostic_messages_and_tail_lines() {
        let path = std::env::temp_dir().join(format!(
            "gdx_wide_log_{}_{}.log",
            std::process::id(),
            uuid::Uuid::new_v4().simple()
        ));
        fs::write(
            &path,
            format!("ERROR: Parse Error: {}\n", "x".repeat(100_000)),
        )
        .unwrap();

        let summary = summarize_file(&path);
        let tail = read_tail(&path);

        assert!(summary.first_error.unwrap().message.chars().count() <= 1_025);
        assert!(tail.chars().count() <= 1_025);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn streams_one_hundred_mib_without_expanding_unique_groups() {
        let path = std::env::temp_dir().join(format!(
            "gdx_large_log_{}_{}.log",
            std::process::id(),
            uuid::Uuid::new_v4().simple()
        ));
        let line = format!("WARNING: repeated {}\n", "x".repeat(980));
        let iterations = (100usize * 1024 * 1024).div_ceil(line.len());
        let mut writer = BufWriter::new(fs::File::create(&path).unwrap());
        for _ in 0..iterations {
            writer.write_all(line.as_bytes()).unwrap();
        }
        writer.flush().unwrap();

        let summary = summarize_file(&path);

        assert_eq!(summary.warnings, iterations as u64);
        assert_eq!(summary.unique_warnings, 1);
        assert_eq!(summary.groups.len(), 1);
        let _ = fs::remove_file(path);
    }
}
