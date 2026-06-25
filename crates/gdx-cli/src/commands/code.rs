use clap::Args;
use serde_json::json;
use std::fs;
use walkdir::WalkDir;

use crate::context::{validate_res_path, AppContext};
use crate::error::{GdxError, GdxResult};
use crate::godot::{self, GodotCommand};
use crate::project::{ensure_parent_dir, godot_path_string, res_to_abs};

#[derive(Debug, Args)]
pub struct CreateArgs {
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
    pub script_path: String,
}

#[derive(Debug, Args)]
pub struct CheckAllArgs {
    #[arg(long, default_value = "res://")]
    pub root: String,
}

#[derive(Debug, Args)]
pub struct LoadCheckArgs {
    #[arg(long, default_value = "res://")]
    pub root: String,
}

pub fn run_create(ctx: &AppContext, args: &CreateArgs) -> GdxResult<serde_json::Value> {
    validate_res_path("--path", &args.path)?;
    if args.extends.trim().is_empty() {
        return Err(GdxError::user(
            "invalid_extends",
            "--extends must not be empty",
        ));
    }
    let project = ctx.project()?;
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

pub fn run_attach(ctx: &AppContext, args: &AttachArgs) -> GdxResult<serde_json::Value> {
    validate_res_path("--scene", &args.scene)?;
    validate_res_path("--script", &args.script)?;
    if let Some(out) = &args.out {
        validate_res_path("--out", out)?;
    }
    let project = ctx.project()?;
    let out = args.out.as_deref().unwrap_or(&args.scene);
    ensure_parent_dir(&res_to_abs(&project.root, out)?)?;
    let result = ctx.run_automation(
        project.root.clone(),
        "script_attach",
        json!({
            "scene": args.scene,
            "node": args.node,
            "script": args.script,
            "out": out
        }),
        60,
    )?;
    Ok(json!({
        "ok": true,
        "command": "script.attach",
        "project": godot_path_string(&project.root),
        "result": result
    }))
}

pub fn run_check(ctx: &AppContext, args: &CheckArgs) -> GdxResult<serde_json::Value> {
    let project = ctx.project()?;
    validate_res_path("script_path", &args.script_path)?;
    let result = run_strict_script_check(ctx, &project.root, &args.script_path, 60)?;
    Ok(json!({
        "ok": true,
        "command": "script.check",
        "project": godot_path_string(&project.root),
        "script": args.script_path,
        "check": "strict_parse",
        "artifacts": {
            "stdout_log": godot_path_string(&result.stdout_log),
            "stderr_log": godot_path_string(&result.stderr_log)
        }
    }))
}

pub fn run_check_all(ctx: &AppContext, args: &CheckAllArgs) -> GdxResult<serde_json::Value> {
    validate_res_path("--root", &args.root)?;
    let project = ctx.project()?;
    let scripts = collect_scripts(&project.root, &args.root)?;
    let mut checked = Vec::new();
    for script in &scripts {
        let result = run_strict_script_check(ctx, &project.root, script, 60)?;
        checked.push(json!({
            "script": script,
            "artifacts": {
                "stdout_log": godot_path_string(&result.stdout_log),
                "stderr_log": godot_path_string(&result.stderr_log)
            }
        }));
    }
    Ok(json!({
        "ok": true,
        "command": "script.check-all",
        "project": godot_path_string(&project.root),
        "check": "strict_parse",
        "root": args.root,
        "count": checked.len(),
        "checked": checked
    }))
}

pub fn run_load_check(ctx: &AppContext, args: &LoadCheckArgs) -> GdxResult<serde_json::Value> {
    validate_res_path("--root", &args.root)?;
    let project = ctx.project()?;
    let result = ctx.run_automation(
        project.root.clone(),
        "script_check_all",
        json!({ "root": args.root }),
        120,
    )?;
    Ok(json!({
        "ok": true,
        "command": "script.load-check",
        "project": godot_path_string(&project.root),
        "check": "load_only",
        "result": result
    }))
}

fn run_strict_script_check(
    ctx: &AppContext,
    project_root: &std::path::Path,
    script_path: &str,
    timeout_secs: u64,
) -> GdxResult<godot::GodotRunResult> {
    let binary = ctx.locate_godot()?;
    let result = godot::run(GodotCommand {
        binary,
        project: project_root.to_path_buf(),
        args: vec![
            "--headless".to_string(),
            "--path".to_string(),
            godot_path_string(project_root),
            "--check-only".to_string(),
            "--script".to_string(),
            script_path.to_string(),
        ],
        envs: Vec::new(),
        timeout_secs,
    })?;
    if result.status_code != 0 {
        let mut error = godot::godot_failed(&result);
        let details = match error.details.take() {
            Some(mut existing) if existing.is_object() => {
                existing["script"] = json!(script_path);
                existing
            }
            Some(existing) => json!({
                "context": existing,
                "script": script_path
            }),
            None => json!({
                "script": script_path
            }),
        };
        return Err(error.with_details(details));
    }
    Ok(result)
}

fn collect_scripts(project_root: &std::path::Path, root: &str) -> GdxResult<Vec<String>> {
    let root_abs = res_to_abs(project_root, root)?;
    let mut scripts = Vec::new();
    if root_abs.is_file() {
        if root_abs.extension().and_then(|ext| ext.to_str()) == Some("gd") {
            scripts.push(crate::project::abs_to_res(project_root, &root_abs)?);
        }
        return Ok(scripts);
    }
    for entry in WalkDir::new(&root_abs).into_iter().filter_map(Result::ok) {
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("gd") {
            scripts.push(crate::project::abs_to_res(project_root, path)?);
        }
    }
    scripts.sort();
    Ok(scripts)
}
