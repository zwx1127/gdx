use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::constants::GDX_DAEMON_SERVER_RES;
use crate::diagnostics::attach_log_diagnostics;
use crate::error::{GdxError, GdxResult};
use crate::project::{ensure_dir, godot_path_string};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonSession {
    pub pid: u32,
    pub port: u16,
    pub token: String,
    pub scene: String,
    pub stdout_log: String,
    pub stderr_log: String,
    pub started_at: u64,
}

pub fn session_path(project: &Path) -> PathBuf {
    project.join(".gdx").join("daemon").join("session.json")
}

pub fn read_session(project: &Path) -> GdxResult<DaemonSession> {
    let path = session_path(project);
    let text = fs::read_to_string(&path).map_err(|err| {
        GdxError::not_found(
            "daemon_not_running",
            format!("Cannot read daemon session {}: {err}", path.display()),
        )
    })?;
    serde_json::from_str(&text)
        .map_err(|err| GdxError::tool("invalid_session", format!("Invalid session file: {err}")))
}

pub fn write_session(project: &Path, session: &DaemonSession) -> GdxResult<()> {
    let path = session_path(project);
    if let Some(parent) = path.parent() {
        ensure_dir(parent)?;
    }
    let text = serde_json::to_string_pretty(session)
        .map_err(|err| GdxError::tool("json_failed", format!("Cannot serialize session: {err}")))?;
    fs::write(&path, text).map_err(|err| {
        GdxError::tool(
            "io_failed",
            format!("Cannot write {}: {err}", path.display()),
        )
    })
}

pub fn remove_session(project: &Path) {
    let _ = fs::remove_file(session_path(project));
}

pub fn rpc(project: &Path, method: &str, params: Value, timeout_secs: u64) -> GdxResult<Value> {
    let session = read_session(project)?;
    rpc_session(&session, method, params, timeout_secs)
        .map_err(|err| enrich_rpc_error(err, project, method))
}

pub fn rpc_session(
    session: &DaemonSession,
    method: &str,
    params: Value,
    timeout_secs: u64,
) -> GdxResult<Value> {
    let mut stream = TcpStream::connect(("127.0.0.1", session.port)).map_err(|err| {
        GdxError::tool(
            "daemon_connect_failed",
            format!("Cannot connect to daemon on port {}: {err}", session.port),
        )
    })?;
    stream
        .set_read_timeout(Some(Duration::from_secs(timeout_secs)))
        .map_err(|err| GdxError::tool("io_failed", format!("Cannot set read timeout: {err}")))?;
    stream
        .set_write_timeout(Some(Duration::from_secs(timeout_secs)))
        .map_err(|err| GdxError::tool("io_failed", format!("Cannot set write timeout: {err}")))?;

    let id = Uuid::new_v4().simple().to_string();
    let request = json!({
        "id": id,
        "token": session.token,
        "method": method,
        "params": params,
    });
    writeln!(stream, "{request}").map_err(|err| {
        GdxError::tool(
            "daemon_write_failed",
            format!("Cannot write RPC request: {err}"),
        )
    })?;

    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line).map_err(|err| {
        GdxError::tool(
            "daemon_read_failed",
            format!("Cannot read RPC response: {err}"),
        )
    })?;
    if line.trim().is_empty() {
        return Err(GdxError::tool(
            "daemon_empty_response",
            "Daemon returned an empty response",
        ));
    }
    let response: Value = serde_json::from_str(line.trim()).map_err(|err| {
        GdxError::tool(
            "daemon_invalid_response",
            format!("Daemon returned invalid JSON: {err}"),
        )
    })?;
    if response.get("ok").and_then(Value::as_bool) == Some(true) {
        Ok(response.get("result").cloned().unwrap_or(Value::Null))
    } else {
        let mut error = GdxError::tool(
            response
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("daemon_error"),
            response
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("Daemon RPC failed"),
        );
        if let Some(details) = response.get("details") {
            error = error.with_details(details.clone());
        }
        Err(error)
    }
}

fn enrich_rpc_error(mut error: GdxError, project: &Path, method: &str) -> GdxError {
    if error.error != "unknown_method" {
        return error;
    }

    let details = match error.details.take() {
        Some(mut existing) if existing.is_object() => {
            existing["original_error"] = json!("unknown_method");
            existing["requested_rpc_method"] = json!(method);
            existing["project"] = json!(godot_path_string(project));
            existing
        }
        Some(existing) => json!({
            "context": existing,
            "original_error": "unknown_method",
            "requested_rpc_method": method,
            "project": godot_path_string(project)
        }),
        None => json!({
            "original_error": "unknown_method",
            "requested_rpc_method": method,
            "project": godot_path_string(project)
        }),
    };

    GdxError::tool(
        "daemon_runtime_outdated",
        format!("Daemon runtime does not support RPC method: {method}"),
    )
    .with_details(details)
    .with_suggestion(
        "Run `gdx --project <project> project install`, then restart the daemon with `gdx --project <project> daemon start --restart`.",
    )
}

pub fn ping_session(session: &DaemonSession) -> bool {
    rpc_session(session, "ping", json!({}), 2).is_ok()
}

pub fn find_free_port() -> GdxResult<u16> {
    let listener = TcpListener::bind(("127.0.0.1", 0)).map_err(|err| {
        GdxError::tool("port_bind_failed", format!("Cannot find free port: {err}"))
    })?;
    listener
        .local_addr()
        .map(|addr| addr.port())
        .map_err(|err| GdxError::tool("port_failed", format!("Cannot read bound port: {err}")))
}

pub struct SpawnDaemon {
    pub binary: PathBuf,
    pub project: PathBuf,
    pub scene: String,
    pub port: u16,
    pub token: String,
    pub width: u32,
    pub height: u32,
}

pub fn spawn_daemon(args: SpawnDaemon) -> GdxResult<DaemonSession> {
    let daemon_dir = args.project.join(".gdx").join("daemon");
    ensure_dir(&daemon_dir)?;
    let stdout_log = daemon_dir.join("godot.daemon.stdout.log");
    let stderr_log = daemon_dir.join("godot.daemon.stderr.log");
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

    let mut command = Command::new(&args.binary);
    command
        .arg("--path")
        .arg(godot_path_string(&args.project))
        .arg("--resolution")
        .arg(format!("{}x{}", args.width, args.height))
        .arg("--single-window")
        .arg(GDX_DAEMON_SERVER_RES)
        .env("GDX_DAEMON_PORT", args.port.to_string())
        .env("GDX_DAEMON_TOKEN", &args.token)
        .env("GDX_DAEMON_SCENE", &args.scene)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout_file))
        .stderr(Stdio::from(stderr_file));

    #[cfg(windows)]
    {
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        command.creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);
    }

    let mut child = spawn_background_command(&mut command).map_err(|err| {
        GdxError::tool("spawn_failed", format!("Cannot start Godot daemon: {err}"))
    })?;

    let session = DaemonSession {
        pid: child.id(),
        port: args.port,
        token: args.token,
        scene: args.scene,
        stdout_log: godot_path_string(&stdout_log),
        stderr_log: godot_path_string(&stderr_log),
        started_at: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or(0),
    };

    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(30) {
        if let Some(status) = child.try_wait().map_err(|err| {
            GdxError::tool("wait_failed", format!("Cannot check daemon status: {err}"))
        })? {
            let error = GdxError::tool(
                "daemon_exited",
                format!("Godot daemon exited early with status {status}"),
            )
            .with_artifact("stdout_log", session.stdout_log.clone())
            .with_artifact("stderr_log", session.stderr_log.clone());
            return Err(attach_log_diagnostics(error, &stdout_log, &stderr_log));
        }
        if ping_session(&session) {
            return Ok(session);
        }
        thread::sleep(Duration::from_millis(250));
    }

    let _ = child.kill();
    let _ = child.wait();
    let error = GdxError::timeout("Timed out waiting for daemon to start")
        .with_artifact("stdout_log", session.stdout_log)
        .with_artifact("stderr_log", session.stderr_log);
    Err(attach_log_diagnostics(error, &stdout_log, &stderr_log))
}

#[cfg(not(windows))]
fn spawn_background_command(command: &mut Command) -> std::io::Result<Child> {
    command.spawn()
}

#[cfg(windows)]
fn spawn_background_command(command: &mut Command) -> std::io::Result<Child> {
    without_inherited_std_handles(|| command.spawn())
}

#[cfg(windows)]
fn without_inherited_std_handles<T>(f: impl FnOnce() -> T) -> T {
    use std::ffi::c_void;

    type Handle = *mut c_void;

    const STD_INPUT_HANDLE: u32 = -10i32 as u32;
    const STD_OUTPUT_HANDLE: u32 = -11i32 as u32;
    const STD_ERROR_HANDLE: u32 = -12i32 as u32;
    const HANDLE_FLAG_INHERIT: u32 = 0x0000_0001;
    const INVALID_HANDLE_VALUE: Handle = -1isize as Handle;

    #[link(name = "kernel32")]
    extern "system" {
        fn GetStdHandle(n_std_handle: u32) -> Handle;
        fn GetHandleInformation(h_object: Handle, lpdw_flags: *mut u32) -> i32;
        fn SetHandleInformation(h_object: Handle, dw_mask: u32, dw_flags: u32) -> i32;
    }

    let mut restore: Vec<(Handle, u32)> = Vec::new();
    for std_handle in [STD_INPUT_HANDLE, STD_OUTPUT_HANDLE, STD_ERROR_HANDLE] {
        unsafe {
            let handle = GetStdHandle(std_handle);
            if handle.is_null() || handle == INVALID_HANDLE_VALUE {
                continue;
            }
            let mut flags = 0;
            if GetHandleInformation(handle, &mut flags) == 0 {
                continue;
            }
            restore.push((handle, flags));
            let inherit_flags = flags & !HANDLE_FLAG_INHERIT;
            let _ = SetHandleInformation(handle, HANDLE_FLAG_INHERIT, inherit_flags);
        }
    }

    let result = f();

    for (handle, flags) in restore {
        unsafe {
            let _ = SetHandleInformation(handle, HANDLE_FLAG_INHERIT, flags & HANDLE_FLAG_INHERIT);
        }
    }

    result
}

pub fn kill_process(pid: u32, force: bool) -> GdxResult<()> {
    if cfg!(windows) {
        let mut command = Command::new("taskkill");
        command.arg("/PID").arg(pid.to_string());
        if force {
            command.arg("/F");
        }
        let status = command
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(|err| GdxError::tool("kill_failed", format!("Cannot run taskkill: {err}")))?;
        if status.success() {
            Ok(())
        } else {
            Err(GdxError::tool(
                "kill_failed",
                format!("taskkill failed for pid {pid}"),
            ))
        }
    } else {
        let signal = if force { "-KILL" } else { "-TERM" };
        let status = Command::new("kill")
            .arg(signal)
            .arg(pid.to_string())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(|err| GdxError::tool("kill_failed", format!("Cannot run kill: {err}")))?;
        if status.success() {
            Ok(())
        } else {
            Err(GdxError::tool(
                "kill_failed",
                format!("kill failed for pid {pid}"),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn unknown_rpc_method_maps_to_runtime_outdated() {
        let error = GdxError::tool("unknown_method", "Unknown RPC method: activate_node")
            .with_details(json!({ "methods": ["ping"] }));
        let mapped = enrich_rpc_error(error, Path::new("D:/Game"), "activate_node");

        assert_eq!(mapped.error, "daemon_runtime_outdated");
        assert!(mapped.message.contains("activate_node"));
        assert!(mapped
            .suggestion
            .as_deref()
            .unwrap()
            .contains("project install"));
        assert_eq!(
            mapped.details.unwrap()["requested_rpc_method"],
            "activate_node"
        );
    }
}
