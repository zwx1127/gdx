use std::path::PathBuf;

use clap::Args;
use serde_json::json;

use crate::commands::Cli;
use crate::error::{GdxError, GdxResult};
use crate::godot;
use crate::project::{assert_project, godot_path_string};

#[derive(Debug, Args)]
pub struct RunArgs {
    #[arg(long)]
    pub project: PathBuf,

    #[arg(long)]
    pub path: String,

    #[arg(long, default_value = "run_tests")]
    pub method: String,
}

pub fn run(cli: &Cli, args: &RunArgs) -> GdxResult<serde_json::Value> {
    if !args.path.starts_with("res://") {
        return Err(GdxError::user(
            "invalid_path",
            "--path must be a res:// path",
        ));
    }
    if args.method.trim().is_empty() {
        return Err(GdxError::user(
            "invalid_method",
            "--method must not be empty",
        ));
    }
    let project = assert_project(&args.project)?;
    let binary = godot::locate_godot(cli.godot.as_deref())?;
    let result = godot::run_automation(
        binary,
        project.root.clone(),
        "test_run",
        json!({
            "path": args.path,
            "method": args.method
        }),
        120,
    )?;
    if result.status_code != 0 {
        return Err(godot::godot_failed(&result));
    }
    Ok(json!({
        "ok": true,
        "command": "test.run",
        "project": godot_path_string(&project.root),
        "result": result.last_json
    }))
}
