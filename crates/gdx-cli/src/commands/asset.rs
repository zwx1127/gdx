use std::fs;
use std::path::{Path, PathBuf};

use clap::Args;
use serde_json::json;
use walkdir::WalkDir;

use crate::commands::Cli;
use crate::error::GdxResult;
use crate::godot::{self, GodotCommand};
use crate::project::{assert_project, godot_path_string};

#[derive(Debug, Args)]
pub struct ImportArgs {
    #[arg(long)]
    pub project: PathBuf,
}

pub fn run_import(cli: &Cli, args: &ImportArgs) -> GdxResult<serde_json::Value> {
    let project = assert_project(&args.project)?;
    let binary = godot::locate_godot(cli.godot.as_deref())?;
    let result = godot::run(GodotCommand {
        binary,
        project: project.root.clone(),
        args: vec![
            "--headless".to_string(),
            "--path".to_string(),
            godot_path_string(&project.root),
            "--import".to_string(),
        ],
        envs: Vec::new(),
        timeout_secs: 120,
    })?;
    if result.status_code != 0 {
        let stderr = fs::read_to_string(&result.stderr_log).unwrap_or_default();
        if is_mono_dotnet_unavailable(&stderr) && !project_has_csharp(&project.root) {
            return Ok(json!({
                "ok": true,
                "command": "asset.import",
                "project": godot_path_string(&project.root),
                "imported": false,
                "skipped": true,
                "reason": "mono_dotnet_unavailable",
                "warning": "Godot mono crashed during editor import because .NET/hostfxr is unavailable. The project has no C# files, so MVP-0 continues without editor import.",
                "artifacts": {
                    "stdout_log": godot_path_string(&result.stdout_log),
                    "stderr_log": godot_path_string(&result.stderr_log)
                }
            }));
        }
        return Err(godot::godot_failed(&result));
    }

    Ok(json!({
        "ok": true,
        "command": "asset.import",
        "project": godot_path_string(&project.root),
        "imported": true,
        "artifacts": {
            "stdout_log": godot_path_string(&result.stdout_log),
            "stderr_log": godot_path_string(&result.stderr_log)
        }
    }))
}

fn is_mono_dotnet_unavailable(stderr: &str) -> bool {
    stderr.contains("Failed to load hostfxr")
        || stderr.contains(".NET failed to get list of installed SDKs")
        || stderr.contains("Could not create child process: dotnet")
}

fn project_has_csharp(project: &Path) -> bool {
    WalkDir::new(project)
        .into_iter()
        .filter_map(Result::ok)
        .any(|entry| {
            entry.file_type().is_file()
                && entry
                    .path()
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .map(|extension| matches!(extension, "cs" | "csproj" | "sln"))
                    .unwrap_or(false)
        })
}
