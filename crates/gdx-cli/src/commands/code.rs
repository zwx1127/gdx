use std::path::PathBuf;

use clap::Args;
use serde_json::json;

use crate::commands::Cli;
use crate::error::GdxResult;
use crate::godot::{self, GodotCommand};
use crate::project::{assert_project, godot_path_string};

#[derive(Debug, Args)]
pub struct CheckArgs {
    #[arg(long)]
    pub project: PathBuf,

    pub script_path: String,
}

pub fn run_check(cli: &Cli, args: &CheckArgs) -> GdxResult<serde_json::Value> {
    let project = assert_project(&args.project)?;
    let binary = godot::locate_godot(cli.godot.as_deref())?;
    let result = godot::run(GodotCommand {
        binary,
        project: project.root.clone(),
        args: vec![
            "--headless".to_string(),
            "--path".to_string(),
            godot_path_string(&project.root),
            "--check-only".to_string(),
            "--script".to_string(),
            args.script_path.clone(),
        ],
        envs: Vec::new(),
        timeout_secs: 60,
    })?;
    if result.status_code != 0 {
        return Err(godot::godot_failed(&result));
    }

    Ok(json!({
        "ok": true,
        "command": "code.check",
        "project": godot_path_string(&project.root),
        "script": args.script_path,
        "check": "parse_only",
        "artifacts": {
            "stdout_log": godot_path_string(&result.stdout_log),
            "stderr_log": godot_path_string(&result.stderr_log)
        }
    }))
}
