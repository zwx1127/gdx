use std::fs;
use std::path::PathBuf;

use clap::Args;
use serde_json::json;

use crate::commands::Cli;
use crate::error::{GdxError, GdxResult};
use crate::godot::{self, GodotCommand};
use crate::project::{assert_project, ensure_parent_dir, godot_path_string, res_to_abs};

#[derive(Debug, Args)]
pub struct CreateArgs {
    #[arg(long)]
    pub project: PathBuf,

    #[arg(long)]
    pub path: String,

    #[arg(long)]
    pub class_name: Option<String>,

    #[arg(long, default_value = "Node")]
    pub extends: String,

    #[arg(long)]
    pub force: bool,
}

#[derive(Debug, Args)]
pub struct AttachArgs {
    #[arg(long)]
    pub project: PathBuf,

    #[arg(long)]
    pub scene: String,

    #[arg(long)]
    pub node: String,

    #[arg(long)]
    pub script: String,

    #[arg(long)]
    pub out: Option<String>,
}

#[derive(Debug, Args)]
pub struct CheckArgs {
    #[arg(long)]
    pub project: PathBuf,

    pub script_path: String,
}

#[derive(Debug, Args)]
pub struct CheckAllArgs {
    #[arg(long)]
    pub project: PathBuf,

    #[arg(long, default_value = "res://")]
    pub root: String,
}

pub fn run_create(args: &CreateArgs) -> GdxResult<serde_json::Value> {
    if !args.path.starts_with("res://") {
        return Err(GdxError::user(
            "invalid_path",
            "--path must be a res:// path",
        ));
    }
    if args.extends.trim().is_empty() {
        return Err(GdxError::user(
            "invalid_extends",
            "--extends must not be empty",
        ));
    }
    let project = assert_project(&args.project)?;
    let script_abs = res_to_abs(&project.root, &args.path)?;
    if script_abs.exists() && !args.force {
        return Err(GdxError::user(
            "script_exists",
            format!("Script already exists: {}", script_abs.display()),
        )
        .with_suggestion("Pass --force to overwrite the existing script."));
    }
    ensure_parent_dir(&script_abs)?;
    let mut text = format!("extends {}\n", args.extends);
    if let Some(class_name) = &args.class_name {
        if class_name.trim().is_empty() {
            return Err(GdxError::user(
                "invalid_class_name",
                "--class-name must not be empty",
            ));
        }
        text.push_str(&format!("class_name {}\n", class_name));
    }
    text.push('\n');
    fs::write(&script_abs, text).map_err(|err| {
        GdxError::tool(
            "io_failed",
            format!("Cannot write {}: {err}", script_abs.display()),
        )
    })?;
    Ok(json!({
        "ok": true,
        "command": "script.create",
        "project": godot_path_string(&project.root),
        "script": args.path,
        "script_abs": godot_path_string(&script_abs)
    }))
}

pub fn run_attach(cli: &Cli, args: &AttachArgs) -> GdxResult<serde_json::Value> {
    validate_res_path("--scene", &args.scene)?;
    validate_res_path("--script", &args.script)?;
    if let Some(out) = &args.out {
        validate_res_path("--out", out)?;
    }
    let project = assert_project(&args.project)?;
    let binary = godot::locate_godot(cli.godot.as_deref())?;
    let result = godot::run_automation(
        binary,
        project.root.clone(),
        "script_attach",
        json!({
            "scene": args.scene,
            "node": args.node,
            "script": args.script,
            "out": args.out
        }),
        60,
    )?;
    if result.status_code != 0 {
        return Err(godot::godot_failed(&result));
    }
    Ok(json!({
        "ok": true,
        "command": "script.attach",
        "project": godot_path_string(&project.root),
        "result": result.last_json
    }))
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
        "command": "script.check",
        "project": godot_path_string(&project.root),
        "script": args.script_path,
        "check": "parse_only",
        "artifacts": {
            "stdout_log": godot_path_string(&result.stdout_log),
            "stderr_log": godot_path_string(&result.stderr_log)
        }
    }))
}

pub fn run_check_all(cli: &Cli, args: &CheckAllArgs) -> GdxResult<serde_json::Value> {
    validate_res_path("--root", &args.root)?;
    let project = assert_project(&args.project)?;
    let binary = godot::locate_godot(cli.godot.as_deref())?;
    let result = godot::run_automation(
        binary,
        project.root.clone(),
        "script_check_all",
        json!({ "root": args.root }),
        120,
    )?;
    if result.status_code != 0 {
        return Err(godot::godot_failed(&result));
    }
    Ok(json!({
        "ok": true,
        "command": "script.check-all",
        "project": godot_path_string(&project.root),
        "result": result.last_json
    }))
}

fn validate_res_path(label: &str, value: &str) -> GdxResult<()> {
    if value.starts_with("res://") {
        Ok(())
    } else {
        Err(GdxError::user(
            "invalid_res_path",
            format!("{label} must be a res:// path"),
        ))
    }
}
