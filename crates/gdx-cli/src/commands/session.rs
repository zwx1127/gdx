use std::path::PathBuf;

use clap::{ArgGroup, Args};
use serde_json::json;
use uuid::Uuid;

use crate::commands::Cli;
use crate::daemon::{self, SpawnDaemon};
use crate::error::{GdxError, GdxResult};
use crate::project::{assert_project, ensure_parent_dir, godot_path_string, read_main_scene};

#[derive(Debug, Args)]
pub struct StartArgs {
    #[arg(long)]
    pub project: PathBuf,

    #[arg(long)]
    pub scene: Option<String>,

    #[arg(long)]
    pub port: Option<u16>,

    #[arg(long, default_value_t = 1280)]
    pub width: u32,

    #[arg(long, default_value_t = 720)]
    pub height: u32,

    #[arg(long)]
    pub restart: bool,
}

#[derive(Debug, Args)]
pub struct StatusArgs {
    #[arg(long)]
    pub project: PathBuf,
}

#[derive(Debug, Args)]
pub struct StopArgs {
    #[arg(long)]
    pub project: PathBuf,

    #[arg(long)]
    pub force: bool,
}

#[derive(Debug, Args)]
pub struct CaptureArgs {
    #[arg(long)]
    pub project: PathBuf,

    #[arg(long)]
    pub out: PathBuf,

    #[arg(long, default_value_t = 10)]
    pub frames: u32,
}

#[derive(Debug, Args)]
#[command(group(
    ArgGroup::new("daemon_input_event")
        .required(true)
        .args(["keycode", "mouse_button", "mouse_motion"])
))]
pub struct InputArgs {
    #[arg(long)]
    pub project: PathBuf,

    #[arg(long)]
    pub keycode: Option<i64>,

    #[arg(long)]
    pub mouse_button: Option<i64>,

    #[arg(long)]
    pub mouse_motion: bool,

    #[arg(long, num_args = 2)]
    pub position: Option<Vec<f64>>,

    #[arg(long, default_value_t = true)]
    pub pressed: bool,
}

#[derive(Debug, Args)]
pub struct CallArgs {
    #[arg(long)]
    pub project: PathBuf,

    #[arg(long)]
    pub target: String,

    #[arg(long)]
    pub method: String,

    #[arg(long, default_value = "[]")]
    pub args_json: String,
}

#[derive(Debug, Args)]
pub struct StateArgs {
    #[arg(long)]
    pub project: PathBuf,

    #[arg(long)]
    pub target: String,

    #[arg(long)]
    pub method: Option<String>,

    #[arg(long)]
    pub property: Option<String>,
}

pub fn run_start(cli: &Cli, args: &StartArgs) -> GdxResult<serde_json::Value> {
    let project = assert_project(&args.project)?;
    let scene = resolve_scene(&project.root, args.scene.as_deref())?;
    if !scene.starts_with("res://") {
        return Err(GdxError::user(
            "invalid_scene",
            "--scene must be a res:// path",
        ));
    }
    if args.width == 0 || args.height == 0 {
        return Err(GdxError::user(
            "invalid_resolution",
            "Width and height must be greater than zero",
        ));
    }
    if let Ok(existing) = daemon::read_session(&project.root) {
        if daemon::ping_session(&existing) {
            if !args.restart {
                return Ok(json!({
                    "ok": true,
                    "command": "daemon.start",
                    "project": godot_path_string(&project.root),
                    "already_running": true,
                    "session": existing
                }));
            }
            let _ = daemon::rpc_session(&existing, "shutdown", json!({}), 3);
            let _ = daemon::kill_process(existing.pid, true);
            daemon::remove_session(&project.root);
        } else {
            daemon::remove_session(&project.root);
        }
    }

    let binary = daemon::locate_godot(cli.godot.as_deref())?;
    let port = match args.port {
        Some(port) => port,
        None => daemon::find_free_port()?,
    };
    let token = Uuid::new_v4().simple().to_string();
    let session = daemon::spawn_daemon(SpawnDaemon {
        binary,
        project: project.root.clone(),
        scene,
        port,
        token,
        width: args.width,
        height: args.height,
    })?;
    daemon::write_session(&project.root, &session)?;

    Ok(json!({
        "ok": true,
        "command": "daemon.start",
        "project": godot_path_string(&project.root),
        "already_running": false,
        "session": session
    }))
}

fn resolve_scene(project: &std::path::Path, explicit: Option<&str>) -> GdxResult<String> {
    if let Some(scene) = explicit {
        return Ok(scene.to_string());
    }
    read_main_scene(project)?.ok_or_else(|| {
        GdxError::user(
            "main_scene_missing",
            "No scene was provided and project has no main scene",
        )
        .with_suggestion("Pass --scene res://... or create one with gdx scene create --set-main.")
    })
}

pub fn run_status(args: &StatusArgs) -> GdxResult<serde_json::Value> {
    let project = assert_project(&args.project)?;
    match daemon::read_session(&project.root) {
        Ok(session) => {
            let running = daemon::ping_session(&session);
            Ok(json!({
                "ok": true,
                "command": "daemon.status",
                "project": godot_path_string(&project.root),
                "running": running,
                "session": session
            }))
        }
        Err(_) => Ok(json!({
            "ok": true,
            "command": "daemon.status",
            "project": godot_path_string(&project.root),
            "running": false,
            "session": null
        })),
    }
}

pub fn run_stop(args: &StopArgs) -> GdxResult<serde_json::Value> {
    let project = assert_project(&args.project)?;
    let session = match daemon::read_session(&project.root) {
        Ok(session) => session,
        Err(_) => {
            return Ok(json!({
                "ok": true,
                "command": "daemon.stop",
                "project": godot_path_string(&project.root),
                "killed": false,
                "reason": "not_running"
            }));
        }
    };
    let shutdown_ok = daemon::rpc_session(&session, "shutdown", json!({}), 3).is_ok();
    if !shutdown_ok || args.force {
        let _ = daemon::kill_process(session.pid, args.force);
    }
    daemon::remove_session(&project.root);
    Ok(json!({
        "ok": true,
        "command": "daemon.stop",
        "project": godot_path_string(&project.root),
        "killed": true,
        "shutdown_rpc": shutdown_ok
    }))
}

pub fn run_capture(args: &CaptureArgs) -> GdxResult<serde_json::Value> {
    let project = assert_project(&args.project)?;
    let out = if args.out.is_absolute() {
        args.out.clone()
    } else {
        std::env::current_dir()
            .map_err(|err| {
                GdxError::tool("io_failed", format!("Cannot read current directory: {err}"))
            })?
            .join(&args.out)
    };
    ensure_parent_dir(&out)?;
    let result = daemon::rpc(
        &project.root,
        "capture",
        json!({
            "out": godot_path_string(&out),
            "frames": args.frames
        }),
        30,
    )?;
    Ok(json!({
        "ok": true,
        "command": "daemon.capture",
        "project": godot_path_string(&project.root),
        "capture": godot_path_string(&out),
        "result": result
    }))
}

pub fn run_input(args: &InputArgs) -> GdxResult<serde_json::Value> {
    let project = assert_project(&args.project)?;
    let position = args.position.clone().unwrap_or_else(|| vec![0.0, 0.0]);
    let params = if let Some(keycode) = args.keycode {
        json!({ "kind": "key", "keycode": keycode, "pressed": args.pressed })
    } else if let Some(button) = args.mouse_button {
        json!({
            "kind": "mouse_button",
            "button": button,
            "position": position,
            "pressed": args.pressed
        })
    } else {
        json!({ "kind": "mouse_motion", "position": position })
    };
    let result = daemon::rpc(&project.root, "input_event", params, 10)?;
    Ok(json!({
        "ok": true,
        "command": "daemon.input",
        "project": godot_path_string(&project.root),
        "result": result
    }))
}

pub fn run_call(args: &CallArgs) -> GdxResult<serde_json::Value> {
    if args.target.trim().is_empty() {
        return Err(GdxError::user(
            "invalid_target",
            "--target must not be empty",
        ));
    }
    if args.method.trim().is_empty() {
        return Err(GdxError::user(
            "invalid_method",
            "--method must not be empty",
        ));
    }
    let call_args: serde_json::Value = serde_json::from_str(&args.args_json).map_err(|err| {
        GdxError::user(
            "invalid_args_json",
            format!("--args-json must be valid JSON array: {err}"),
        )
    })?;
    if !call_args.is_array() {
        return Err(GdxError::user(
            "invalid_args_json",
            "--args-json must be a JSON array",
        ));
    }
    let project = assert_project(&args.project)?;
    let result = daemon::rpc(
        &project.root,
        "call_method",
        json!({
            "target": args.target,
            "method": args.method,
            "args": call_args
        }),
        30,
    )?;
    Ok(json!({
        "ok": true,
        "command": "daemon.call",
        "project": godot_path_string(&project.root),
        "result": result
    }))
}

pub fn run_state(args: &StateArgs) -> GdxResult<serde_json::Value> {
    if args.target.trim().is_empty() {
        return Err(GdxError::user(
            "invalid_target",
            "--target must not be empty",
        ));
    }
    let project = assert_project(&args.project)?;
    let result = daemon::rpc(
        &project.root,
        "get_state",
        json!({
            "target": args.target,
            "method": args.method,
            "property": args.property
        }),
        10,
    )?;
    Ok(json!({
        "ok": true,
        "command": "daemon.state",
        "project": godot_path_string(&project.root),
        "result": result
    }))
}
