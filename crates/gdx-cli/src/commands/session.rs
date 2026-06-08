use std::path::PathBuf;

use clap::Args;
use serde_json::json;
use uuid::Uuid;

use crate::commands::Cli;
use crate::daemon::{self, SpawnDaemon};
use crate::error::{GdxError, GdxResult};
use crate::project::{assert_project, ensure_parent_dir, godot_path_string, read_main_scene};

#[derive(Debug, Args)]
pub struct ServeArgs {
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
pub struct KillArgs {
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

pub fn run_serve(cli: &Cli, args: &ServeArgs) -> GdxResult<serde_json::Value> {
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
                    "command": "serve",
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
        "command": "serve",
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
        .with_suggestion("Pass --scene res://... or create one with gdx scene new --set-main.")
    })
}

pub fn run_status(args: &StatusArgs) -> GdxResult<serde_json::Value> {
    let project = assert_project(&args.project)?;
    match daemon::read_session(&project.root) {
        Ok(session) => {
            let running = daemon::ping_session(&session);
            Ok(json!({
                "ok": true,
                "command": "status",
                "project": godot_path_string(&project.root),
                "running": running,
                "session": session
            }))
        }
        Err(_) => Ok(json!({
            "ok": true,
            "command": "status",
            "project": godot_path_string(&project.root),
            "running": false,
            "session": null
        })),
    }
}

pub fn run_kill(args: &KillArgs) -> GdxResult<serde_json::Value> {
    let project = assert_project(&args.project)?;
    let session = match daemon::read_session(&project.root) {
        Ok(session) => session,
        Err(_) => {
            return Ok(json!({
                "ok": true,
                "command": "kill",
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
        "command": "kill",
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
        "command": "capture",
        "project": godot_path_string(&project.root),
        "capture": godot_path_string(&out),
        "result": result
    }))
}
