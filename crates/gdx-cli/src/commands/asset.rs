use std::fs;
use std::path::{Path, PathBuf};

use clap::Args;
use serde_json::json;
use walkdir::WalkDir;

use crate::commands::Cli;
use crate::error::{GdxError, GdxResult};
use crate::godot::{self, GodotCommand};
use crate::project::{assert_project, ensure_parent_dir, godot_path_string, res_to_abs};

#[derive(Debug, Args)]
pub struct CopyArgs {
    #[arg(long)]
    pub project: PathBuf,

    #[arg(long)]
    pub from: PathBuf,

    #[arg(long)]
    pub to: String,

    #[arg(long)]
    pub force: bool,
}

#[derive(Debug, Args)]
pub struct ImportArgs {
    #[arg(long)]
    pub project: PathBuf,
}

#[derive(Debug, Args)]
pub struct InspectArgs {
    #[arg(long)]
    pub project: PathBuf,

    #[arg(long)]
    pub path: String,
}

pub fn run_copy(args: &CopyArgs) -> GdxResult<serde_json::Value> {
    if !args.to.starts_with("res://") {
        return Err(GdxError::user("invalid_to", "--to must be a res:// path"));
    }
    let project = assert_project(&args.project)?;
    let from = if args.from.is_absolute() {
        args.from.clone()
    } else {
        std::env::current_dir()
            .map_err(|err| {
                GdxError::tool("io_failed", format!("Cannot read current directory: {err}"))
            })?
            .join(&args.from)
    };
    if !from.is_file() {
        return Err(GdxError::not_found(
            "source_not_found",
            format!("Source file does not exist: {}", from.display()),
        ));
    }
    let to_abs = res_to_abs(&project.root, &args.to)?;
    if to_abs.exists() && !args.force {
        return Err(GdxError::user(
            "asset_exists",
            format!("Target asset already exists: {}", to_abs.display()),
        )
        .with_suggestion("Pass --force to overwrite the target asset."));
    }
    ensure_parent_dir(&to_abs)?;
    fs::copy(&from, &to_abs).map_err(|err| {
        GdxError::tool(
            "io_failed",
            format!(
                "Cannot copy {} to {}: {err}",
                from.display(),
                to_abs.display()
            ),
        )
    })?;
    Ok(json!({
        "ok": true,
        "command": "asset.copy",
        "project": godot_path_string(&project.root),
        "from": godot_path_string(&from),
        "to": args.to,
        "to_abs": godot_path_string(&to_abs)
    }))
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

pub fn run_inspect(cli: &Cli, args: &InspectArgs) -> GdxResult<serde_json::Value> {
    if !args.path.starts_with("res://") {
        return Err(GdxError::user(
            "invalid_path",
            "--path must be a res:// path",
        ));
    }
    let project = assert_project(&args.project)?;
    let binary = godot::locate_godot(cli.godot.as_deref())?;
    let result = godot::run_automation(
        binary,
        project.root.clone(),
        "asset_inspect",
        json!({ "path": args.path }),
        30,
    )?;
    if result.status_code != 0 {
        return Err(godot::godot_failed(&result));
    }
    Ok(json!({
        "ok": true,
        "command": "asset.inspect",
        "project": godot_path_string(&project.root),
        "asset": result.last_json
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
