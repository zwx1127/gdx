use std::collections::BTreeMap;
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
use crate::diagnostics::attach_log_diagnostics_with_context;
use crate::error::{GdxError, GdxResult};
use crate::project::{ensure_dir, godot_path_string};
use crate::traversal::is_generated_path;

pub const RUNTIME_UPDATE_SUGGESTION: &str = "Run `gdx --project <project> project update --check` to inspect managed runtime file changes, then run `gdx --project <project> project update` and restart the daemon with `gdx --project <project> daemon start --restart`.";
const MAX_SCRIPT_CHANGE_PATHS: usize = 50;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScriptSnapshot {
    pub files: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScriptSyncStatus {
    pub status: &'static str,
    pub snapshot_present: bool,
    pub total_changes: usize,
    pub added: Vec<String>,
    pub modified: Vec<String>,
    pub removed: Vec<String>,
    pub paths_truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonSession {
    pub pid: u32,
    pub port: u16,
    pub token: String,
    pub scene: String,
    pub stdout_log: String,
    pub stderr_log: String,
    pub started_at: u64,
    #[serde(default)]
    pub script_snapshot: Option<ScriptSnapshot>,
}

pub fn runtime_dir(project: &Path) -> PathBuf {
    let canonical = fs::canonicalize(project).unwrap_or_else(|_| project.to_path_buf());
    let mut identity = godot_path_string(&canonical);
    if cfg!(windows) {
        identity.make_ascii_lowercase();
    }
    let mut hash = 0xcbf29ce484222325u64;
    for byte in identity.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    std::env::temp_dir()
        .join("gdx")
        .join("runtime")
        .join(format!("{hash:016x}"))
}

pub fn session_path(project: &Path) -> PathBuf {
    runtime_dir(project).join("session.json")
}

pub fn read_session(project: &Path) -> GdxResult<DaemonSession> {
    let path = session_path(project);
    let text = fs::read_to_string(&path).map_err(|_| {
        GdxError::not_found(
            "daemon_not_running",
            "No active daemon session for this project",
        )
    })?;
    serde_json::from_str(text.trim_start_matches('\u{feff}'))
        .map_err(|err| GdxError::tool("invalid_session", format!("Invalid session file: {err}")))
}

pub fn write_session(project: &Path, session: &DaemonSession) -> GdxResult<()> {
    let path = session_path(project);
    if let Some(parent) = path.parent() {
        ensure_dir(parent)?;
    }
    let text = serde_json::to_string(session)
        .map_err(|err| GdxError::tool("json_failed", format!("Cannot serialize session: {err}")))?;
    fs::write(&path, text).map_err(|err| {
        GdxError::tool(
            "io_failed",
            format!("Cannot write {}: {err}", path.display()),
        )
    })
}

pub fn remove_session(project: &Path) {
    let _ = fs::remove_dir_all(runtime_dir(project));
}

pub fn rpc(project: &Path, method: &str, params: Value, timeout_secs: u64) -> GdxResult<Value> {
    let session = read_session(project)?;
    ensure_session_scripts_fresh(project, &session)?;
    rpc_session(&session, method, params, timeout_secs)
        .map_err(|err| enrich_rpc_error(err, project, method))
}

pub fn ensure_session_scripts_fresh(project: &Path, session: &DaemonSession) -> GdxResult<()> {
    let sync = script_sync_status(project, session)?;
    if sync.status == "fresh" {
        return Ok(());
    }

    let message = if sync.status == "unknown" {
        "The running daemon has no GDScript source snapshot"
    } else {
        "GDScript sources changed after the daemon started"
    };
    Err(GdxError::tool("daemon_scripts_stale", message)
        .with_stage("daemon_script_sync")
        .with_details(json!(sync))
        .with_suggestion(format!(
            "Restart the daemon with `gdx --project {} daemon start --restart`.",
            godot_path_string(project)
        )))
}

pub fn script_sync_status(project: &Path, session: &DaemonSession) -> GdxResult<ScriptSyncStatus> {
    let Some(previous) = session.script_snapshot.as_ref() else {
        return Ok(ScriptSyncStatus {
            status: "unknown",
            snapshot_present: false,
            total_changes: 0,
            added: Vec::new(),
            modified: Vec::new(),
            removed: Vec::new(),
            paths_truncated: false,
        });
    };
    let current = capture_script_snapshot(project)?;
    Ok(compare_script_snapshots(previous, &current))
}

fn capture_script_snapshot(project: &Path) -> GdxResult<ScriptSnapshot> {
    let mut files = BTreeMap::new();
    let walker = walkdir::WalkDir::new(project)
        .into_iter()
        .filter_entry(|entry| {
            !is_generated_path(project, entry.path())
                && (!entry.file_type().is_dir()
                    || entry.path() == project
                    || !entry.path().join(".gdignore").is_file())
        });
    for entry in walker {
        let entry = entry.map_err(|err| {
            GdxError::tool(
                "daemon_script_sync_failed",
                format!("Cannot inspect GDScript sources: {err}"),
            )
            .with_stage("daemon_script_sync")
        })?;
        if !entry.file_type().is_file()
            || entry
                .path()
                .extension()
                .and_then(|extension| extension.to_str())
                .map(|extension| !extension.eq_ignore_ascii_case("gd"))
                .unwrap_or(true)
        {
            continue;
        }
        let contents = fs::read(entry.path()).map_err(|err| {
            GdxError::tool(
                "daemon_script_sync_failed",
                format!("Cannot read {}: {err}", entry.path().display()),
            )
            .with_stage("daemon_script_sync")
            .with_subject(godot_path_string(entry.path()))
        })?;
        let relative = entry.path().strip_prefix(project).map_err(|err| {
            GdxError::tool(
                "daemon_script_sync_failed",
                format!(
                    "Cannot resolve {} relative to project: {err}",
                    entry.path().display()
                ),
            )
            .with_stage("daemon_script_sync")
        })?;
        let path = format!("res://{}", godot_path_string(relative));
        files.insert(path, content_hash(&contents));
    }
    Ok(ScriptSnapshot { files })
}

fn compare_script_snapshots(
    previous: &ScriptSnapshot,
    current: &ScriptSnapshot,
) -> ScriptSyncStatus {
    let mut changes: Vec<(&'static str, String)> = Vec::new();
    for (path, hash) in &current.files {
        match previous.files.get(path) {
            None => changes.push(("added", path.clone())),
            Some(previous_hash) if previous_hash != hash => {
                changes.push(("modified", path.clone()))
            }
            Some(_) => {}
        }
    }
    for path in previous.files.keys() {
        if !current.files.contains_key(path) {
            changes.push(("removed", path.clone()));
        }
    }
    changes.sort_by(|left, right| left.1.cmp(&right.1).then(left.0.cmp(right.0)));

    let total_changes = changes.len();
    let mut added = Vec::new();
    let mut modified = Vec::new();
    let mut removed = Vec::new();
    for (kind, path) in changes.into_iter().take(MAX_SCRIPT_CHANGE_PATHS) {
        match kind {
            "added" => added.push(path),
            "modified" => modified.push(path),
            "removed" => removed.push(path),
            _ => unreachable!(),
        }
    }

    ScriptSyncStatus {
        status: if total_changes == 0 { "fresh" } else { "stale" },
        snapshot_present: true,
        total_changes,
        added,
        modified,
        removed,
        paths_truncated: total_changes > MAX_SCRIPT_CHANGE_PATHS,
    }
}

fn content_hash(bytes: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("fnv1a64:{hash:016x}")
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
    let response: Value = serde_json::from_str(line.trim().trim_start_matches('\u{feff}'))
        .map_err(|err| {
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
    if error.error == "daemon_invalid_response" {
        error.insert_detail("requested_rpc_method", json!(method));
        error.insert_detail("project", json!(godot_path_string(project)));
        error.context.suggestion = Some(RUNTIME_UPDATE_SUGGESTION.to_string());
        return error;
    }
    if error.error != "unknown_method" {
        return error;
    }

    let details = match error.context.details.take() {
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
    .with_suggestion(RUNTIME_UPDATE_SUGGESTION)
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
    let script_snapshot = capture_script_snapshot(&args.project)?;
    let project_has_csharp = crate::godot::project_has_csharp(&args.project);
    let daemon_dir = runtime_dir(&args.project);
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
        script_snapshot: Some(script_snapshot),
    };

    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(30) {
        if crate::process::cancellation_requested() {
            let _ = kill_process(child.id(), true);
            let _ = child.wait();
            let error = GdxError::cancelled("Daemon startup was cancelled")
                .with_stage("daemon_start")
                .with_child_pid(session.pid)
                .with_elapsed_ms(started.elapsed().as_millis());
            let error = attach_log_diagnostics_with_context(
                error,
                &stdout_log,
                &stderr_log,
                Some(project_has_csharp),
            );
            remove_session(&args.project);
            return Err(error);
        }
        if let Some(status) = child.try_wait().map_err(|err| {
            GdxError::tool("wait_failed", format!("Cannot check daemon status: {err}"))
        })? {
            let error = GdxError::tool(
                "daemon_exited",
                format!("Godot daemon exited early with status {status}"),
            );
            let error = attach_log_diagnostics_with_context(
                error,
                &stdout_log,
                &stderr_log,
                Some(project_has_csharp),
            );
            remove_session(&args.project);
            return Err(error);
        }
        if ping_session(&session) {
            return Ok(session);
        }
        thread::sleep(Duration::from_millis(250));
    }

    let _ = kill_process(child.id(), true);
    let _ = child.wait();
    let error = attach_log_diagnostics_with_context(
        GdxError::timeout("Timed out waiting for daemon to start"),
        &stdout_log,
        &stderr_log,
        Some(project_has_csharp),
    );
    remove_session(&args.project);
    Err(error)
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
        command.arg("/T");
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
            .context
            .suggestion
            .as_deref()
            .unwrap()
            .contains("project update --check"));
        assert_eq!(
            mapped.context.details.unwrap()["requested_rpc_method"],
            "activate_node"
        );
    }

    #[test]
    fn reads_bom_prefixed_session_json() {
        let project = std::env::temp_dir().join(format!(
            "gdx_daemon_session_{}_{}",
            std::process::id(),
            uuid::Uuid::new_v4().simple()
        ));
        let path = session_path(&project);
        assert!(!path.starts_with(&project));
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            &path,
            "\u{feff}{\"pid\":1,\"port\":2,\"token\":\"t\",\"scene\":\"res://main.tscn\",\"stdout_log\":\"out\",\"stderr_log\":\"err\",\"started_at\":3}",
        )
        .unwrap();

        let session = read_session(&project).unwrap();

        assert_eq!(session.port, 2);
        assert!(session.script_snapshot.is_none());
        remove_session(&project);
    }

    #[test]
    fn script_snapshot_reports_added_modified_removed_and_ignores_gdignore() {
        let project = std::env::temp_dir().join(format!(
            "gdx_daemon_scripts_{}_{}",
            std::process::id(),
            uuid::Uuid::new_v4().simple()
        ));
        fs::create_dir_all(project.join("scripts")).unwrap();
        fs::create_dir_all(project.join("design")).unwrap();
        fs::write(project.join("scripts/main.gd"), "extends Node\n").unwrap();
        fs::write(project.join("scripts/removed.gd"), "extends Node\n").unwrap();
        fs::write(project.join("design/.gdignore"), "").unwrap();
        fs::write(project.join("design/reference.gd"), "extends Node\n").unwrap();

        let previous = capture_script_snapshot(&project).unwrap();
        assert_eq!(previous.files.len(), 2);
        assert!(!previous.files.contains_key("res://design/reference.gd"));

        fs::write(
            project.join("scripts/main.gd"),
            "extends Node\nfunc changed():\n    pass\n",
        )
        .unwrap();
        fs::write(project.join("scripts/added.gd"), "extends Node\n").unwrap();
        fs::remove_file(project.join("scripts/removed.gd")).unwrap();
        fs::write(
            project.join("design/reference.gd"),
            "extends Node\nfunc ignored():\n    pass\n",
        )
        .unwrap();

        let current = capture_script_snapshot(&project).unwrap();
        let sync = compare_script_snapshots(&previous, &current);
        assert_eq!(sync.status, "stale");
        assert_eq!(sync.total_changes, 3);
        assert_eq!(sync.added, vec!["res://scripts/added.gd".to_string()]);
        assert_eq!(sync.modified, vec!["res://scripts/main.gd".to_string()]);
        assert_eq!(sync.removed, vec!["res://scripts/removed.gd".to_string()]);
        assert!(!sync.paths_truncated);

        let _ = fs::remove_dir_all(project);
    }

    #[test]
    fn script_snapshot_change_details_are_bounded() {
        let previous = ScriptSnapshot::default();
        let mut current = ScriptSnapshot::default();
        for index in 0..55 {
            current.files.insert(
                format!("res://scripts/{index:02}.gd"),
                format!("hash-{index}"),
            );
        }

        let sync = compare_script_snapshots(&previous, &current);

        assert_eq!(sync.total_changes, 55);
        assert_eq!(sync.added.len(), MAX_SCRIPT_CHANGE_PATHS);
        assert!(sync.paths_truncated);
    }

    #[test]
    fn invalid_rpc_response_keeps_code_and_adds_update_context() {
        let error = GdxError::tool(
            "daemon_invalid_response",
            "Daemon returned invalid JSON: number out of range",
        );

        let enriched = enrich_rpc_error(error, Path::new("D:/Game"), "call_method");

        assert_eq!(enriched.error, "daemon_invalid_response");
        assert_eq!(
            enriched.context.details.as_ref().unwrap()["requested_rpc_method"],
            "call_method"
        );
        assert!(enriched
            .context
            .suggestion
            .as_deref()
            .unwrap()
            .contains("project update --check"));
    }

    #[test]
    fn legacy_session_without_snapshot_requires_restart() {
        let session = DaemonSession {
            pid: 1,
            port: 2,
            token: "token".into(),
            scene: "res://main.tscn".into(),
            stdout_log: "stdout.log".into(),
            stderr_log: "stderr.log".into(),
            started_at: 3,
            script_snapshot: None,
        };

        let error = ensure_session_scripts_fresh(Path::new("D:/Game"), &session).unwrap_err();

        assert_eq!(error.error, "daemon_scripts_stale");
        assert_eq!(error.context.details.unwrap()["status"], "unknown");
    }
}
