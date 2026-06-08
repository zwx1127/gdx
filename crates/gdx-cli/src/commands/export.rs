use std::path::PathBuf;

use clap::Args;
use serde_json::json;

use crate::context::{validate_non_empty, AppContext};
use crate::error::{GdxError, GdxResult};
use crate::godot::{self, GodotCommand};
use crate::project::godot_path_string;

#[derive(Debug, Args)]
pub struct BuildArgs {
    #[arg(long)]
    pub preset: String,

    #[arg(long)]
    pub out: PathBuf,
}

pub fn run_build(ctx: &AppContext, args: &BuildArgs) -> GdxResult<serde_json::Value> {
    validate_non_empty("preset", &args.preset)?;
    let project = ctx.project()?;
    let presets = project.root.join("export_presets.cfg");
    if !presets.is_file() {
        return Err(GdxError::user(
            "export_presets_not_found",
            format!("Missing {}", presets.display()),
        )
        .with_suggestion("Create export_presets.cfg in Godot before running export build."));
    }
    let out = ctx.abs_path(&args.out);
    if let Some(parent) = out.parent() {
        if !parent.as_os_str().is_empty() && !parent.is_dir() {
            return Err(GdxError::user(
                "export_dir_not_found",
                format!("Output directory does not exist: {}", parent.display()),
            ));
        }
    }

    let binary = ctx.locate_godot()?;
    let result = godot::run(GodotCommand {
        binary,
        project: project.root.clone(),
        args: vec![
            "--headless".to_string(),
            "--path".to_string(),
            godot_path_string(&project.root),
            "--export-release".to_string(),
            args.preset.clone(),
            godot_path_string(&out),
        ],
        envs: Vec::new(),
        timeout_secs: 600,
    })?;
    if result.status_code != 0 {
        return Err(godot::godot_failed(&result).with_suggestion(
            "Check export preset name and ensure Godot export templates are installed.",
        ));
    }

    Ok(json!({
        "ok": true,
        "command": "export.build",
        "project": godot_path_string(&project.root),
        "preset": args.preset,
        "out": godot_path_string(&out),
        "artifacts": {
            "stdout_log": godot_path_string(&result.stdout_log),
            "stderr_log": godot_path_string(&result.stderr_log)
        }
    }))
}
