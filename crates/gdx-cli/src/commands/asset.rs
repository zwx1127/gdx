use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;

use clap::Args;
use serde_json::json;

use crate::context::{validate_res_path, AppContext};
use crate::error::GdxResult;
use crate::godot::{self, GodotCommand};
use crate::project::godot_path_string;

#[derive(Debug, Args)]
pub struct ImportArgs {}

#[derive(Debug, Args)]
pub struct InspectArgs {
    #[arg(long)]
    pub path: String,
}

pub fn run_import(ctx: &AppContext, _args: &ImportArgs) -> GdxResult<serde_json::Value> {
    let project = ctx.project()?;
    let binary = ctx.locate_godot()?;
    let result = ctx.run_godot(GodotCommand {
        binary,
        args: vec![
            "--headless".to_string(),
            "--path".to_string(),
            godot_path_string(&project.root),
            "--import".to_string(),
        ],
        envs: Vec::new(),
        timeout_secs: 120,
        deadline: None,
        operation: "asset.import".to_string(),
        subject: None,
    })?;
    if result.status_code != 0 {
        if log_has_mono_dotnet_unavailable(&result.stderr_log)
            && !godot::project_has_csharp(&project.root)
        {
            return Ok(json!({
                "ok": true,
                "command": "asset.import",
                "project": godot_path_string(&project.root),
                "imported": false,
                "skipped": true,
                "reason": "mono_dotnet_unavailable",
                "warning": "Godot mono crashed during editor import because .NET/hostfxr is unavailable. The project has no C# files, so the diagnostic is non-blocking."
            }));
        }
        return Err(godot::godot_failed(&result));
    }

    Ok(json!({
        "ok": true,
        "command": "asset.import",
        "project": godot_path_string(&project.root),
        "imported": true
    }))
}

pub fn run_inspect(ctx: &AppContext, args: &InspectArgs) -> GdxResult<serde_json::Value> {
    validate_res_path("--path", &args.path)?;
    let project = ctx.project()?;
    let result = ctx.run_automation(
        project.root.clone(),
        "asset_inspect",
        json!({ "path": args.path }),
        30,
    )?;
    Ok(json!({
        "ok": true,
        "command": "asset.inspect",
        "project": godot_path_string(&project.root),
        "asset": result
    }))
}

fn is_mono_dotnet_unavailable(stderr: &str) -> bool {
    stderr.contains("Failed to load hostfxr")
        || stderr.contains(".NET failed to get list of installed SDKs")
        || stderr.contains("Could not create child process: dotnet")
}

fn log_has_mono_dotnet_unavailable(path: &Path) -> bool {
    let Ok(file) = fs::File::open(path) else {
        return false;
    };
    BufReader::new(file)
        .lines()
        .map_while(Result::ok)
        .any(|line| is_mono_dotnet_unavailable(&line))
}
