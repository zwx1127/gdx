use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::Value;
use uuid::Uuid;

use crate::constants::GDX_TOOLS_AUTOMATION_RES;
use crate::diagnostics::{
    attach_log_diagnostics, classify_logs, summarize_logs, LogSummary, WarningMode,
};
use crate::error::{GdxError, GdxResult};
use crate::process::{cancellation_requested, ManagedChild};
use crate::progress::ProgressReporter;
use crate::project::godot_path_string;
use crate::run_store::RunHandle;
use crate::traversal::is_generated_path;

const STANDARD_GODOT_CANDIDATES: &[&str] = &[
    "godot",
    "godot4",
    "Godot_v4.6-stable_win64.exe",
    "Godot_v4.5-stable_win64.exe",
    "Godot_v4.4-stable_win64.exe",
    "Godot_v4.3-stable_win64.exe",
    "Godot_v4.2-stable_win64.exe",
    "Godot_v4.1-stable_win64.exe",
    "Godot_v4.0-stable_win64.exe",
    "Godot_v4.6-stable",
    "Godot_v4.5-stable",
    "Godot_v4.4-stable",
    "Godot_v4.3-stable",
    "Godot_v4.2-stable",
    "Godot_v4.1-stable",
    "Godot_v4.0-stable",
];

const DOTNET_GODOT_CANDIDATES: &[&str] = &[
    "Godot_v4.6-stable_mono_win64_console.exe",
    "Godot_v4.5-stable_mono_win64_console.exe",
    "Godot_v4.4-stable_mono_win64_console.exe",
    "godot-mono",
];

#[derive(Debug, Clone)]
pub struct GodotCommand {
    pub binary: PathBuf,
    pub project: PathBuf,
    pub args: Vec<String>,
    pub envs: Vec<(String, String)>,
    pub timeout_secs: u64,
    pub deadline: Option<Instant>,
    pub operation: String,
    pub subject: Option<String>,
}

#[derive(Debug, Clone)]
pub struct GodotRunResult {
    pub run_id: String,
    pub operation: String,
    pub subject: Option<String>,
    pub child_pid: u32,
    pub status_code: i32,
    pub duration_ms: u128,
    pub timeout_secs: u64,
    pub stdout_log: PathBuf,
    pub stderr_log: PathBuf,
    pub last_json: Option<Value>,
    pub log_summary: LogSummary,
}

pub struct GodotSelection<'a> {
    pub explicit: Option<&'a Path>,
    pub standard: Option<&'a Path>,
    pub dotnet: Option<&'a Path>,
    pub project: Option<&'a Path>,
}

pub fn locate_godot(selection: GodotSelection<'_>) -> GdxResult<PathBuf> {
    if let Some(path) = selection.explicit {
        return validate_binary(path);
    }

    let project_has_csharp = selection.project.map(project_has_csharp).unwrap_or(false);
    let typed = if project_has_csharp {
        selection.dotnet
    } else {
        selection.standard
    };
    if let Some(path) = typed {
        return validate_binary(path);
    }

    if let Ok(path) = std::env::var("GDX_GODOT") {
        if !path.trim().is_empty() {
            return validate_binary(Path::new(&path));
        }
    }

    let candidate = if project_has_csharp {
        find_dotnet_on_path()
    } else {
        find_standard_on_path()
    };
    if let Some(path) = candidate {
        return Ok(path);
    }

    Err(GdxError::not_found(
        "godot_not_found",
        "Could not find Godot. Pass --godot <path> or set GDX_GODOT.",
    ))
}

pub fn find_standard_on_path() -> Option<PathBuf> {
    find_on_path(STANDARD_GODOT_CANDIDATES)
}

pub fn find_dotnet_on_path() -> Option<PathBuf> {
    find_on_path(DOTNET_GODOT_CANDIDATES)
}

fn find_on_path(candidates: &[&str]) -> Option<PathBuf> {
    for candidate in candidates {
        if let Ok(path) = which::which(candidate) {
            return Some(path);
        }
    }
    None
}

fn validate_binary(path: &Path) -> GdxResult<PathBuf> {
    if path.is_file() {
        Ok(path.to_path_buf())
    } else {
        Err(GdxError::not_found(
            "godot_not_found",
            format!("Godot binary does not exist: {}", path.display()),
        ))
    }
}

pub fn run_version(
    binary: &Path,
    timeout_secs: u64,
    progress: &ProgressReporter,
) -> GdxResult<String> {
    let run_id = format!("gdx_godot_version_{}", Uuid::new_v4().simple());
    let stdout_log = std::env::temp_dir().join(format!("{run_id}.stdout.log"));
    let stderr_log = std::env::temp_dir().join(format!("{run_id}.stderr.log"));
    let stdout_file = File::create(&stdout_log).map_err(|err| {
        GdxError::tool(
            "io_failed",
            format!("Cannot create {}: {err}", stdout_log.display()),
        )
    })?;
    let stderr_file = File::create(&stderr_log).map_err(|err| {
        GdxError::tool(
            "io_failed",
            format!("Cannot create {}: {err}", stderr_log.display()),
        )
    })?;

    let mut command = Command::new(binary);
    command
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout_file))
        .stderr(Stdio::from(stderr_file));
    let mut child = ManagedChild::spawn(&mut command)?;
    let child_pid = child.id();
    progress.child_started(
        &run_id,
        child_pid,
        Some("godot --version"),
        timeout_secs,
        &godot_path_string(&stdout_log),
        &godot_path_string(&stderr_log),
    );

    let started = Instant::now();
    let mut last_heartbeat = Instant::now();
    loop {
        if child
            .try_wait()
            .map_err(|err| GdxError::tool("wait_failed", format!("Cannot wait for Godot: {err}")))?
            .is_some()
        {
            let stdout = fs::read_to_string(&stdout_log).map_err(|err| {
                GdxError::tool(
                    "io_failed",
                    format!("Cannot read {}: {err}", stdout_log.display()),
                )
            })?;
            let stderr = fs::read_to_string(&stderr_log).unwrap_or_default();
            let status = child.wait().map_err(|err| {
                GdxError::tool("wait_failed", format!("Cannot collect Godot status: {err}"))
            })?;
            if !status.success() {
                let status_code = status.code().unwrap_or(-1);
                if is_native_crash(status_code, &stdout_log, &stderr_log) {
                    return Err(native_crash_error(
                        "Godot crashed while running --version",
                        status_code,
                        &stdout_log,
                        &stderr_log,
                    )
                    .with_stage("environment")
                    .with_subject("godot --version")
                    .with_child_pid(child_pid)
                    .with_elapsed_ms(started.elapsed().as_millis())
                    .with_timeout_secs(timeout_secs));
                }
                let message = if stderr.trim().is_empty() {
                    format!(
                        "Godot --version exited with status {}",
                        format_status_code(status_code)
                    )
                } else {
                    stderr.trim().to_string()
                };
                let error = GdxError::tool("godot_failed", message)
                    .with_stage("environment")
                    .with_subject("godot --version")
                    .with_child_pid(child_pid)
                    .with_elapsed_ms(started.elapsed().as_millis())
                    .with_timeout_secs(timeout_secs)
                    .with_artifact("stdout_log", godot_path_string(&stdout_log))
                    .with_artifact("stderr_log", godot_path_string(&stderr_log));
                return Err(attach_log_diagnostics(error, &stdout_log, &stderr_log));
            }
            let _ = fs::remove_file(&stdout_log);
            let _ = fs::remove_file(&stderr_log);
            return Ok(stdout.trim().to_string());
        }
        if cancellation_requested() {
            let _ = child.terminate_tree(Duration::from_secs(1));
            let error = GdxError::cancelled("Godot version check was cancelled")
                .with_stage("environment")
                .with_subject("godot --version")
                .with_child_pid(child_pid)
                .with_elapsed_ms(started.elapsed().as_millis())
                .with_timeout_secs(timeout_secs)
                .with_artifact("stdout_log", godot_path_string(&stdout_log))
                .with_artifact("stderr_log", godot_path_string(&stderr_log));
            return Err(attach_log_diagnostics(error, &stdout_log, &stderr_log));
        }
        if started.elapsed() >= Duration::from_secs(timeout_secs) {
            let _ = child.terminate_tree(Duration::from_secs(1));
            let error = GdxError::timeout("Timed out while running godot --version")
                .with_stage("environment")
                .with_subject("godot --version")
                .with_child_pid(child_pid)
                .with_elapsed_ms(started.elapsed().as_millis())
                .with_timeout_secs(timeout_secs)
                .with_artifact("stdout_log", godot_path_string(&stdout_log))
                .with_artifact("stderr_log", godot_path_string(&stderr_log));
            return Err(attach_log_diagnostics(error, &stdout_log, &stderr_log));
        }
        if last_heartbeat.elapsed() >= Duration::from_secs(5) {
            progress.heartbeat(&run_id, child_pid, Some("godot --version"));
            last_heartbeat = Instant::now();
        }
        thread::sleep(Duration::from_millis(50));
    }
}

pub fn run(
    command: GodotCommand,
    progress: &ProgressReporter,
    _warning_mode: WarningMode,
) -> GdxResult<GodotRunResult> {
    let mut run = RunHandle::begin(
        &command.project,
        &command.operation,
        command.subject.as_deref(),
    )?;
    let stdout_log = run.stdout_log.clone();
    let stderr_log = run.stderr_log.clone();
    let stdout_file = File::create(&stdout_log).map_err(|err| {
        GdxError::tool(
            "io_failed",
            format!("Cannot create {}: {err}", stdout_log.display()),
        )
    })?;
    let stderr_file = File::create(&stderr_log).map_err(|err| {
        GdxError::tool(
            "io_failed",
            format!("Cannot create {}: {err}", stderr_log.display()),
        )
    })?;

    let mut cmd = Command::new(&command.binary);
    cmd.args(&command.args);
    for (key, value) in &command.envs {
        cmd.env(key, value);
    }
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::from(stdout_file));
    cmd.stderr(Stdio::from(stderr_file));

    let mut child = match ManagedChild::spawn(&mut cmd) {
        Ok(child) => child,
        Err(error) => {
            run.finish("failed", None, Duration::ZERO);
            return Err(error
                .with_stage("spawn")
                .with_subject(command.subject.as_deref().unwrap_or(&command.operation))
                .with_elapsed_ms(0)
                .with_timeout_secs(command.timeout_secs)
                .with_artifact("stdout_log", godot_path_string(&stdout_log))
                .with_artifact("stderr_log", godot_path_string(&stderr_log)));
        }
    };
    let child_pid = child.id();
    run.set_child_pid(child_pid)?;
    progress.child_started(
        &run.run_id,
        child_pid,
        command.subject.as_deref(),
        command.timeout_secs,
        &godot_path_string(&stdout_log),
        &godot_path_string(&stderr_log),
    );

    let started = Instant::now();
    let mut last_heartbeat = Instant::now();
    loop {
        if child
            .try_wait()
            .map_err(|err| GdxError::tool("wait_failed", format!("Cannot wait for Godot: {err}")))?
            .is_some()
        {
            let status = child.wait().map_err(|err| {
                GdxError::tool("wait_failed", format!("Cannot collect Godot status: {err}"))
            })?;
            let status_code = status.code().unwrap_or(-1);
            let status_name = if status.success() {
                "succeeded"
            } else {
                "failed"
            };
            run.finish(status_name, Some(status_code), started.elapsed());
            return Ok(GodotRunResult {
                run_id: run.run_id,
                operation: command.operation,
                subject: command.subject,
                child_pid,
                status_code,
                duration_ms: started.elapsed().as_millis(),
                timeout_secs: command.timeout_secs,
                stdout_log,
                stderr_log,
                last_json: last_stdout_json(&run.stdout_log),
                log_summary: summarize_logs(&run.stdout_log, &run.stderr_log),
            });
        }

        if cancellation_requested() {
            let _ = child.terminate_tree(Duration::from_secs(2));
            run.finish("cancelled", None, started.elapsed());
            let error = GdxError::cancelled("Godot command was cancelled")
                .with_stage("running")
                .with_subject(command.subject.as_deref().unwrap_or(&command.operation))
                .with_child_pid(child_pid)
                .with_elapsed_ms(started.elapsed().as_millis())
                .with_timeout_secs(command.timeout_secs)
                .with_artifact("stdout_log", godot_path_string(&stdout_log))
                .with_artifact("stderr_log", godot_path_string(&stderr_log));
            return Err(attach_log_diagnostics(error, &stdout_log, &stderr_log));
        }

        let overall_timed_out = command
            .deadline
            .map(|deadline| Instant::now() >= deadline)
            .unwrap_or(false);
        if overall_timed_out || started.elapsed() >= Duration::from_secs(command.timeout_secs) {
            let _ = child.terminate_tree(Duration::from_secs(2));
            run.finish("timed_out", None, started.elapsed());
            let message = if overall_timed_out {
                "Global command timeout elapsed while running Godot"
            } else {
                "Timed out while running Godot"
            };
            let error = GdxError::timeout(message)
                .with_stage("running")
                .with_subject(command.subject.as_deref().unwrap_or(&command.operation))
                .with_child_pid(child_pid)
                .with_elapsed_ms(started.elapsed().as_millis())
                .with_timeout_secs(command.timeout_secs)
                .with_artifact("stdout_log", godot_path_string(&stdout_log))
                .with_artifact("stderr_log", godot_path_string(&stderr_log));
            return Err(attach_log_diagnostics(error, &stdout_log, &stderr_log));
        }

        if last_heartbeat.elapsed() >= Duration::from_secs(5) {
            progress.heartbeat(&run.run_id, child_pid, command.subject.as_deref());
            last_heartbeat = Instant::now();
        }

        thread::sleep(Duration::from_millis(100));
    }
}

pub fn godot_failed(result: &GodotRunResult) -> GdxError {
    let native_crash = is_native_crash(result.status_code, &result.stdout_log, &result.stderr_log);
    let mut error = if native_crash {
        GdxError::tool(
            "godot_native_crash",
            format!(
                "Godot crashed in native code with status {}",
                format_status_code(result.status_code)
            ),
        )
    } else if let Some(last_json) = &result.last_json {
        if last_json.get("ok").and_then(Value::as_bool) == Some(false) {
            let code = last_json
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("godot_failed");
            let message = last_json
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("Godot exited with non-zero status");
            let mut error = GdxError::tool(code, message);
            if let Some(details) = last_json.get("details") {
                error = error.with_details(details.clone());
            }
            error
        } else {
            GdxError::tool("godot_failed", "Godot exited with non-zero status")
        }
    } else {
        GdxError::tool("godot_failed", "Godot exited with non-zero status")
    };

    error = error
        .with_stage("godot")
        .with_subject(result.subject.as_deref().unwrap_or(&result.operation))
        .with_child_pid(result.child_pid)
        .with_elapsed_ms(result.duration_ms)
        .with_timeout_secs(result.timeout_secs)
        .with_artifact("stdout_log", godot_path_string(&result.stdout_log))
        .with_artifact("stderr_log", godot_path_string(&result.stderr_log));
    error = if native_crash {
        error.with_suggestion(
            "Godot crashed before gdx received runtime JSON. Inspect the stderr log and the local Godot/runtime environment, or pass an explicit --godot/GDX_GODOT path.",
        )
    } else {
        error.with_suggestion("Open the stderr log and fix the first Godot error.")
    };
    attach_log_diagnostics(error, &result.stdout_log, &result.stderr_log)
}

fn native_crash_error(
    context: &str,
    status_code: i32,
    stdout_log: &Path,
    stderr_log: &Path,
) -> GdxError {
    let error = GdxError::tool(
        "godot_native_crash",
        format!("{context} with status {}", format_status_code(status_code)),
    )
    .with_artifact("stdout_log", godot_path_string(stdout_log))
    .with_artifact("stderr_log", godot_path_string(stderr_log))
    .with_suggestion(
        "Godot crashed before gdx received runtime JSON. Inspect the stderr log and the local Godot/runtime environment, or pass an explicit --godot/GDX_GODOT path.",
    );
    attach_log_diagnostics(error, stdout_log, stderr_log)
}

fn is_native_crash(status_code: i32, stdout_log: &Path, stderr_log: &Path) -> bool {
    status_code == 0xC0000005u32 as i32
        || classify_logs(stdout_log, stderr_log) == Some("godot_native_crash")
}

fn format_status_code(status_code: i32) -> String {
    if status_code < 0 {
        format!("{status_code} ({:#010X})", status_code as u32)
    } else {
        status_code.to_string()
    }
}

pub fn automation_command(
    binary: PathBuf,
    project: PathBuf,
    action: &str,
    params: Value,
    timeout_secs: u64,
) -> GodotCommand {
    GodotCommand {
        binary,
        project: project.clone(),
        args: vec![
            "--headless".to_string(),
            "--path".to_string(),
            godot_path_string(&project),
            "-s".to_string(),
            GDX_TOOLS_AUTOMATION_RES.to_string(),
        ],
        envs: vec![
            ("GDX_TOOL_ACTION".to_string(), action.to_string()),
            ("GDX_TOOL_PARAMS".to_string(), params.to_string()),
        ],
        timeout_secs,
        deadline: None,
        operation: action.to_string(),
        subject: None,
    }
}

fn last_stdout_json(path: &Path) -> Option<Value> {
    let file = File::open(path).ok()?;
    BufReader::new(file)
        .lines()
        .map_while(Result::ok)
        .map(|line| line.trim().trim_start_matches('\u{feff}').to_string())
        .filter(|line| line.starts_with('{') && line.ends_with('}'))
        .filter_map(|line| serde_json::from_str(&line).ok())
        .last()
}

pub fn project_has_csharp(project: &Path) -> bool {
    walkdir::WalkDir::new(project)
        .into_iter()
        .filter_entry(|entry| !is_generated_path(project, entry.path()))
        .filter_map(Result::ok)
        .any(|entry| {
            entry.file_type().is_file()
                && entry
                    .path()
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .map(|extension| matches!(extension, "cs" | "csproj" | "sln"))
                    .unwrap_or(false)
        })
}

pub fn binary_is_dotnet(binary: &Path, version: &str) -> bool {
    version.contains(".mono.")
        || binary
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| {
                let lower = name.to_lowercase();
                lower.contains("mono") || lower.contains("dotnet")
            })
            .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn godot_failed_uses_automation_json_error() {
        let result = GodotRunResult {
            run_id: "r_test".to_string(),
            operation: "test".to_string(),
            subject: Some("res://test.gd".to_string()),
            child_pid: 1,
            status_code: 1,
            duration_ms: 1,
            timeout_secs: 60,
            stdout_log: PathBuf::from("stdout.log"),
            stderr_log: PathBuf::from("stderr.log"),
            last_json: Some(json!({
                "ok": false,
                "error": "save_failed",
                "message": "ResourceSaver.save failed: ERR_CANT_CREATE",
                "details": { "out": "res://scenes/main.tscn" }
            })),
            log_summary: LogSummary::default(),
        };

        let error = godot_failed(&result);

        assert_eq!(error.error, "save_failed");
        assert!(error.message.contains("ERR_CANT_CREATE"));
        assert_eq!(
            error.context.details.unwrap()["out"],
            "res://scenes/main.tscn"
        );
    }

    #[test]
    fn godot_failed_classifies_native_crash_status() {
        let result = GodotRunResult {
            run_id: "r_test".to_string(),
            operation: "test".to_string(),
            subject: None,
            child_pid: 1,
            status_code: 0xC0000005u32 as i32,
            duration_ms: 1,
            timeout_secs: 60,
            stdout_log: PathBuf::from("stdout.log"),
            stderr_log: PathBuf::from("stderr.log"),
            last_json: None,
            log_summary: LogSummary::default(),
        };

        let error = godot_failed(&result);

        assert_eq!(error.error, "godot_native_crash");
        assert!(error.message.contains("0xC0000005"));
    }

    #[test]
    fn explicit_and_project_typed_godot_selection_are_deterministic() {
        let root = std::env::temp_dir().join(format!(
            "gdx_godot_selection_{}_{}",
            std::process::id(),
            uuid::Uuid::new_v4().simple()
        ));
        fs::create_dir_all(&root).unwrap();
        let explicit = root.join("explicit.exe");
        let standard = root.join("standard.exe");
        let dotnet = root.join("dotnet.exe");
        for path in [&explicit, &standard, &dotnet] {
            fs::write(path, []).unwrap();
        }

        let selected = locate_godot(GodotSelection {
            explicit: Some(&explicit),
            standard: Some(&standard),
            dotnet: Some(&dotnet),
            project: Some(&root),
        })
        .unwrap();
        assert_eq!(selected, explicit);

        fs::write(root.join("game.cs"), "class Game {}\n").unwrap();
        let selected = locate_godot(GodotSelection {
            explicit: None,
            standard: Some(&standard),
            dotnet: Some(&dotnet),
            project: Some(&root),
        })
        .unwrap();
        assert_eq!(selected, dotnet);
        let _ = fs::remove_dir_all(root);
    }
}
