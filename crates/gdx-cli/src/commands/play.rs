use std::fs;
use std::path::PathBuf;

use clap::Args;
use serde_json::json;

use crate::constants::GDX_RUNTIME_CAPTURE_RUNNER_RES;
use crate::context::{validate_res_path, AppContext};
use crate::error::{GdxError, GdxResult};
use crate::godot::{self, GodotCommand};
use crate::project::{ensure_parent_dir, godot_path_string, read_main_scene};

#[derive(Debug, Args)]
pub struct CaptureArgs {
    #[arg(long)]
    pub scene: Option<String>,

    #[arg(long)]
    pub out: PathBuf,

    #[arg(long, default_value_t = 10)]
    pub frames: u32,

    #[arg(long, default_value_t = 1280)]
    pub width: u32,

    #[arg(long, default_value_t = 720)]
    pub height: u32,
}

pub fn run_capture(ctx: &AppContext, args: &CaptureArgs) -> GdxResult<serde_json::Value> {
    let project = ctx.project()?;
    let scene = resolve_scene(&project.root, args.scene.as_deref())?;
    validate_res_path("--scene", &scene)?;
    if args.width == 0 || args.height == 0 {
        return Err(GdxError::user(
            "invalid_resolution",
            "Width and height must be greater than zero",
        ));
    }
    let capture = ctx.abs_path(&args.out);
    ensure_parent_dir(&capture)?;
    let binary = ctx.locate_godot()?;
    let result = godot::run(GodotCommand {
        binary,
        project: project.root.clone(),
        args: vec![
            "--path".to_string(),
            godot_path_string(&project.root),
            "--resolution".to_string(),
            format!("{}x{}", args.width, args.height),
            "--single-window".to_string(),
            GDX_RUNTIME_CAPTURE_RUNNER_RES.to_string(),
        ],
        envs: vec![
            ("GDX_TARGET_SCENE".to_string(), scene.clone()),
            ("GDX_CAPTURE_OUT".to_string(), godot_path_string(&capture)),
            ("GDX_CAPTURE_FRAMES".to_string(), args.frames.to_string()),
        ],
        timeout_secs: 120,
    })?;
    if result.status_code != 0 {
        return Err(godot::godot_failed(&result).with_suggestion(runtime_capture_suggestion()));
    }

    let size = fs::metadata(&capture)
        .map(|metadata| metadata.len())
        .unwrap_or(0);
    if size == 0 {
        return Err(GdxError::tool(
            "capture_not_created",
            format!(
                "Godot completed but capture is missing or empty: {}",
                capture.display()
            ),
        )
        .with_artifact("stdout_log", godot_path_string(&result.stdout_log))
        .with_artifact("stderr_log", godot_path_string(&result.stderr_log)));
    }

    Ok(json!({
        "ok": true,
        "command": "capture.run",
        "project": godot_path_string(&project.root),
        "scene": scene,
        "capture": godot_path_string(&capture),
        "frames": args.frames,
        "resolution": [args.width, args.height],
        "artifacts": {
            "capture": godot_path_string(&capture),
            "stdout_log": godot_path_string(&result.stdout_log),
            "stderr_log": godot_path_string(&result.stderr_log)
        }
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

fn runtime_capture_suggestion() -> &'static str {
    if std::env::consts::OS == "linux" {
        "Ensure DISPLAY or WAYLAND_DISPLAY is set. In headless environments, run Godot with xvfb-run -a."
    } else {
        "Ensure Godot can open a normal runtime window on this machine."
    }
}
