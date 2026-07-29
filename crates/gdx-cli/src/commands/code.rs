use clap::Args;
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use walkdir::WalkDir;

use crate::context::{validate_res_path, AppContext};
use crate::error::{GdxError, GdxResult};
use crate::godot::{self, GodotCommand};
use crate::project::{ensure_parent_dir, godot_path_string, res_to_abs};
use crate::traversal::{is_explicitly_excluded, is_generated_path};

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

    #[arg(long)]
    pub jobs: Option<usize>,

    #[arg(long, default_value_t = 60)]
    pub script_timeout_secs: u64,

    #[arg(long, default_value_t = 300)]
    pub timeout_secs: u64,

    #[arg(long = "exclude")]
    pub excludes: Vec<String>,
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
    run_strict_script_check(ctx, &project.root, &args.script_path, 60)?;
    Ok(json!({
        "ok": true,
        "command": "script.check",
        "project": godot_path_string(&project.root),
        "script": args.script_path,
        "check": "strict_parse"
    }))
}

pub fn run_check_all(ctx: &AppContext, args: &CheckAllArgs) -> GdxResult<serde_json::Value> {
    validate_res_path("--root", &args.root)?;
    if args.jobs == Some(0) {
        return Err(GdxError::user(
            "invalid_jobs",
            "--jobs must be greater than zero",
        ));
    }
    if args.script_timeout_secs == 0 || args.timeout_secs == 0 {
        return Err(GdxError::user(
            "invalid_timeout",
            "Timeouts must be greater than zero",
        ));
    }
    if args.timeout_secs < args.script_timeout_secs {
        return Err(GdxError::user(
            "invalid_timeout",
            "--timeout-secs must be greater than or equal to --script-timeout-secs",
        ));
    }
    for excluded in &args.excludes {
        validate_res_path("--exclude", excluded)?;
    }
    let project = ctx.project()?;
    let scripts = collect_scripts(&project.root, &args.root, &args.excludes)?;
    let total = scripts.len();
    let auto_jobs = std::thread::available_parallelism()
        .map(|value| value.get())
        .unwrap_or(1)
        .min(4);
    let jobs = args.jobs.unwrap_or(auto_jobs).min(total.max(1));
    let binary = ctx.locate_godot()?;
    let deadline = Instant::now() + Duration::from_secs(args.timeout_secs);
    let project_root = Arc::new(project.root.clone());
    let scripts = Arc::new(scripts);
    let next = Arc::new(AtomicUsize::new(0));
    let completed = Arc::new(AtomicUsize::new(0));
    let results: Arc<Mutex<Vec<Option<GdxResult<godot::GodotRunResult>>>>> =
        Arc::new(Mutex::new((0..total).map(|_| None).collect()));

    std::thread::scope(|scope| {
        for _ in 0..jobs {
            let scripts = Arc::clone(&scripts);
            let next = Arc::clone(&next);
            let completed = Arc::clone(&completed);
            let results = Arc::clone(&results);
            let ctx = ctx.clone();
            let binary = binary.clone();
            let project_root = Arc::clone(&project_root);
            scope.spawn(move || loop {
                if Instant::now() >= deadline || crate::process::cancellation_requested() {
                    break;
                }
                let index = next.fetch_add(1, Ordering::SeqCst);
                if index >= scripts.len() {
                    break;
                }
                let script = &scripts[index];
                let started = Instant::now();
                let result = run_strict_script_check_with_binary(
                    &ctx,
                    binary.clone(),
                    &project_root,
                    script,
                    args.script_timeout_secs,
                    Some(deadline),
                );
                let ok = result.is_ok();
                results.lock().unwrap_or_else(|err| err.into_inner())[index] = Some(result);
                let done = completed.fetch_add(1, Ordering::SeqCst) + 1;
                ctx.progress
                    .item_finished(done, total, script, started.elapsed().as_millis(), ok);
            });
        }
    });

    let mut results = Arc::try_unwrap(results)
        .map_err(|_| GdxError::tool("internal_error", "Script results are still shared"))?
        .into_inner()
        .unwrap_or_else(|err| err.into_inner());
    let completed_count = results.iter().filter(|value| value.is_some()).count();
    if crate::process::cancellation_requested() && completed_count < total {
        return Err(GdxError::cancelled("Script checks were cancelled")
            .with_stage("script_check")
            .with_subject(args.root.clone())
            .with_details(json!({
                "counts": {
                    "discovered": total,
                    "checked": completed_count,
                    "remaining": total - completed_count,
                }
            })));
    }
    if completed_count < total {
        let next_subject = results
            .iter()
            .position(Option::is_none)
            .map(|index| scripts[index].clone());
        return Err(GdxError::timeout("Global script check timeout elapsed")
            .with_stage("script_check")
            .with_subject(next_subject.unwrap_or_else(|| args.root.clone()))
            .with_elapsed_ms(args.timeout_secs as u128 * 1_000)
            .with_timeout_secs(args.timeout_secs)
            .with_details(json!({
                "counts": {
                    "discovered": total,
                    "checked": completed_count,
                    "remaining": total - completed_count,
                }
            })));
    }

    let mut checked_count = 0usize;
    let mut failures = Vec::new();
    let mut first_error = None;
    for (index, result) in results.iter_mut().enumerate() {
        let script = &scripts[index];
        match result.take().expect("every script result must be present") {
            Ok(_) => checked_count += 1,
            Err(error) => {
                failures.push(error_json(script, &error));
                if first_error.is_none() {
                    first_error = Some(error);
                }
            }
        }
    }
    if let Some(mut error) = first_error {
        error.insert_detail("failures", json!(failures));
        error.insert_detail(
            "counts",
            json!({
                "discovered": total,
                "checked": total,
                "failed": failures.len(),
            }),
        );
        return Err(error);
    }
    Ok(json!({
        "ok": true,
        "command": "script.check-all",
        "project": godot_path_string(&project.root),
        "check": "strict_parse",
        "root": args.root,
        "jobs": jobs,
        "counts": {
            "discovered": total,
            "checked": checked_count,
            "failed": 0,
        }
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
        "root": args.root,
        "counts": {
            "checked": result.get("count").and_then(serde_json::Value::as_u64).unwrap_or(0),
            "failed": 0
        }
    }))
}

fn run_strict_script_check(
    ctx: &AppContext,
    project_root: &std::path::Path,
    script_path: &str,
    timeout_secs: u64,
) -> GdxResult<godot::GodotRunResult> {
    let binary = ctx.locate_godot()?;
    run_strict_script_check_with_binary(ctx, binary, project_root, script_path, timeout_secs, None)
}

fn run_strict_script_check_with_binary(
    ctx: &AppContext,
    binary: PathBuf,
    project_root: &Path,
    script_path: &str,
    timeout_secs: u64,
    deadline: Option<Instant>,
) -> GdxResult<godot::GodotRunResult> {
    let result = ctx.run_godot(GodotCommand {
        binary,
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
        deadline,
        operation: "script.check".to_string(),
        subject: Some(script_path.to_string()),
    })?;
    if result.status_code != 0 {
        let mut error = godot::godot_failed(&result);
        let details = match error.context.details.take() {
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

fn collect_scripts(project_root: &Path, root: &str, excludes: &[String]) -> GdxResult<Vec<String>> {
    let root_abs = res_to_abs(project_root, root)?;
    let excluded_abs = excludes
        .iter()
        .map(|value| res_to_abs(project_root, value))
        .collect::<GdxResult<Vec<_>>>()?;
    let mut scripts = Vec::new();
    if root_abs.is_file() {
        if !is_generated_path(project_root, &root_abs)
            && !is_explicitly_excluded(project_root, &root_abs, &excluded_abs)
            && root_abs.extension().and_then(|ext| ext.to_str()) == Some("gd")
        {
            scripts.push(crate::project::abs_to_res(project_root, &root_abs)?);
        }
        return Ok(scripts);
    }
    for entry in WalkDir::new(&root_abs).into_iter().filter_entry(|entry| {
        !is_generated_path(project_root, entry.path())
            && !is_explicitly_excluded(project_root, entry.path(), &excluded_abs)
    }) {
        let entry = entry.map_err(|err| {
            GdxError::tool(
                "script_discovery_failed",
                format!("Cannot walk scripts: {err}"),
            )
        })?;
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("gd") {
            scripts.push(crate::project::abs_to_res(project_root, path)?);
        }
    }
    scripts.sort();
    Ok(scripts)
}

fn error_json(script: &str, error: &GdxError) -> serde_json::Value {
    json!({
        "script": script,
        "error": error.error,
        "message": error.message,
        "stage": error.context.stage,
        "child_pid": error.context.child_pid,
        "details": error.context.details,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_discovery_prunes_generated_dirs_but_keeps_gdx_addons() {
        let root = std::env::temp_dir().join(format!(
            "gdx_script_discovery_{}_{}",
            std::process::id(),
            uuid::Uuid::new_v4().simple()
        ));
        for path in [
            "scripts/main.gd",
            "addons/gdx_tools/automation.gd",
            ".gdx/runs/old.gd",
            ".godot/generated.gd",
            "export/copied.gd",
            "ignored/skip.gd",
        ] {
            let path = root.join(path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, "extends Node\n").unwrap();
        }

        let scripts = collect_scripts(&root, "res://", &["res://ignored".to_string()]).unwrap();

        assert_eq!(
            scripts,
            vec![
                "res://addons/gdx_tools/automation.gd",
                "res://scripts/main.gd"
            ]
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn script_discovery_does_not_walk_ten_thousand_run_directories() {
        let root = std::env::temp_dir().join(format!(
            "gdx_script_history_{}_{}",
            std::process::id(),
            uuid::Uuid::new_v4().simple()
        ));
        let history = root.join(".gdx").join("runs");
        fs::create_dir_all(&history).unwrap();
        for index in 0..10_000 {
            fs::create_dir(history.join(format!("r_{index:05}"))).unwrap();
        }
        fs::create_dir_all(root.join("scripts")).unwrap();
        fs::write(root.join("scripts/main.gd"), "extends Node\n").unwrap();

        let started = Instant::now();
        let scripts = collect_scripts(&root, "res://", &[]).unwrap();

        assert_eq!(scripts, vec!["res://scripts/main.gd"]);
        assert!(started.elapsed() < Duration::from_secs(2));
        let _ = fs::remove_dir_all(root);
    }
}
