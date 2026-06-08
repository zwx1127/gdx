use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::Value;
use uuid::Uuid;

use crate::constants::GDX_TOOLS_AUTOMATION_RES;
use crate::error::{GdxError, GdxResult};
use crate::project::{ensure_dir, godot_path_string};

#[derive(Debug, Clone)]
pub struct GodotCommand {
    pub binary: PathBuf,
    pub project: PathBuf,
    pub args: Vec<String>,
    pub envs: Vec<(String, String)>,
    pub timeout_secs: u64,
}

#[derive(Debug, Clone)]
pub struct GodotRunResult {
    pub status_code: i32,
    pub stdout_log: PathBuf,
    pub stderr_log: PathBuf,
    pub last_json: Option<Value>,
}

pub fn locate_godot(explicit: Option<&Path>) -> GdxResult<PathBuf> {
    if let Some(path) = explicit {
        return validate_binary(path);
    }

    if let Ok(path) = std::env::var("GDX_GODOT") {
        if !path.trim().is_empty() {
            return validate_binary(Path::new(&path));
        }
    }

    let candidates = [
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

    for candidate in candidates {
        if let Ok(path) = which::which(candidate) {
            return Ok(path);
        }
    }

    Err(GdxError::not_found(
        "godot_not_found",
        "Could not find Godot. Pass --godot <path> or set GDX_GODOT.",
    ))
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

pub fn run_version(binary: &Path, timeout_secs: u64) -> GdxResult<String> {
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

    let mut child = Command::new(binary)
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout_file))
        .stderr(Stdio::from(stderr_file))
        .spawn()
        .map_err(|err| GdxError::tool("spawn_failed", format!("Cannot start Godot: {err}")))?;

    let started = Instant::now();
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
            let _ = fs::remove_file(&stdout_log);
            let _ = fs::remove_file(&stderr_log);
            if !status.success() {
                return Err(GdxError::tool("godot_failed", stderr.trim().to_string()));
            }
            return Ok(stdout.trim().to_string());
        }
        if started.elapsed() >= Duration::from_secs(timeout_secs) {
            let _ = child.kill();
            let _ = child.wait();
            let _ = fs::remove_file(&stdout_log);
            let _ = fs::remove_file(&stderr_log);
            return Err(GdxError::timeout("Timed out while running godot --version"));
        }
        thread::sleep(Duration::from_millis(50));
    }
}

pub fn run(command: GodotCommand) -> GdxResult<GodotRunResult> {
    let run_id = format!("r_{}", Uuid::new_v4().simple());
    let run_dir = command.project.join(".gdx").join("runs").join(run_id);
    ensure_dir(&run_dir)?;
    let stdout_log = run_dir.join("godot.stdout.log");
    let stderr_log = run_dir.join("godot.stderr.log");
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

    let mut child = cmd
        .spawn()
        .map_err(|err| GdxError::tool("spawn_failed", format!("Cannot start Godot: {err}")))?;

    let started = Instant::now();
    loop {
        if child
            .try_wait()
            .map_err(|err| GdxError::tool("wait_failed", format!("Cannot wait for Godot: {err}")))?
            .is_some()
        {
            let status = child.wait().map_err(|err| {
                GdxError::tool("wait_failed", format!("Cannot collect Godot status: {err}"))
            })?;
            let stdout = fs::read_to_string(&stdout_log).map_err(|err| {
                GdxError::tool(
                    "io_failed",
                    format!("Cannot read {}: {err}", stdout_log.display()),
                )
            })?;
            return Ok(GodotRunResult {
                status_code: status.code().unwrap_or(-1),
                stdout_log,
                stderr_log,
                last_json: last_stdout_json(&stdout),
            });
        }

        if started.elapsed() >= Duration::from_secs(command.timeout_secs) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(GdxError::timeout("Timed out while running Godot")
                .with_artifact("stdout_log", godot_path_string(&stdout_log))
                .with_artifact("stderr_log", godot_path_string(&stderr_log)));
        }

        thread::sleep(Duration::from_millis(100));
    }
}

pub fn godot_failed(result: &GodotRunResult) -> GdxError {
    GdxError::tool("godot_failed", "Godot exited with non-zero status")
        .with_artifact("stdout_log", godot_path_string(&result.stdout_log))
        .with_artifact("stderr_log", godot_path_string(&result.stderr_log))
        .with_suggestion("Open the stderr log and fix the first Godot error.")
}

pub fn run_automation(
    binary: PathBuf,
    project: PathBuf,
    action: &str,
    params: Value,
    timeout_secs: u64,
) -> GdxResult<GodotRunResult> {
    run(GodotCommand {
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
    })
}

fn last_stdout_json(stdout: &str) -> Option<Value> {
    stdout
        .lines()
        .rev()
        .map(str::trim)
        .filter(|line| line.starts_with('{') && line.ends_with('}'))
        .find_map(|line| serde_json::from_str(line).ok())
}
