use std::path::{Path, PathBuf};

use clap::Args;
use serde_json::json;
use walkdir::WalkDir;

use crate::commands::Cli;
use crate::constants::{
    GDX_DAEMON_SERVER_GD, GDX_DAEMON_SERVER_TSCN, GDX_RUNTIME_CAPTURE_RUNNER_GD,
    GDX_RUNTIME_CAPTURE_RUNNER_TSCN, GDX_TOOLS_AUTOMATION_GD, GDX_TOOLS_CREATE_SCENE_GD,
};
use crate::error::{GdxError, GdxResult};
use crate::godot;
use crate::project::{
    assert_project, godot_path_string, list_project_settings, read_main_scene, read_project_name,
    read_project_setting, remove_project_setting, set_project_setting_quoted,
    set_project_setting_raw,
};

use super::init::{ensure_gdx_gitignore, install_gdx_addons};

#[derive(Debug, Args)]
pub struct InstallArgs {
    #[arg(long)]
    pub project: PathBuf,
}

#[derive(Debug, Args)]
pub struct InspectArgs {
    #[arg(long)]
    pub project: PathBuf,

    #[arg(long, default_value_t = 500)]
    pub max_files: usize,
}

#[derive(Debug, Args)]
pub struct SettingGetArgs {
    #[arg(long)]
    pub project: PathBuf,

    #[arg(long)]
    pub section: String,

    #[arg(long)]
    pub key: String,
}

#[derive(Debug, Args)]
pub struct SettingSetArgs {
    #[arg(long)]
    pub project: PathBuf,

    #[arg(long)]
    pub section: String,

    #[arg(long)]
    pub key: String,

    #[arg(long)]
    pub value: String,

    #[arg(long)]
    pub raw: bool,
}

#[derive(Debug, Args)]
pub struct SettingListArgs {
    #[arg(long)]
    pub project: PathBuf,

    #[arg(long)]
    pub section: String,
}

#[derive(Debug, Args)]
pub struct AutoloadAddArgs {
    #[arg(long)]
    pub project: PathBuf,

    #[arg(long)]
    pub name: String,

    #[arg(long)]
    pub path: String,

    #[arg(long)]
    pub global: bool,
}

#[derive(Debug, Args)]
pub struct AutoloadRemoveArgs {
    #[arg(long)]
    pub project: PathBuf,

    #[arg(long)]
    pub name: String,
}

#[derive(Debug, Args)]
pub struct AutoloadListArgs {
    #[arg(long)]
    pub project: PathBuf,
}

#[derive(Debug, Args)]
pub struct InputAddArgs {
    #[arg(long)]
    pub project: PathBuf,

    #[arg(long)]
    pub action: String,

    #[arg(long)]
    pub keycode: Option<i64>,

    #[arg(long)]
    pub mouse_button: Option<i64>,

    #[arg(long, default_value_t = 0.5)]
    pub deadzone: f64,
}

#[derive(Debug, Args)]
pub struct InputRemoveArgs {
    #[arg(long)]
    pub project: PathBuf,

    #[arg(long)]
    pub action: String,
}

#[derive(Debug, Args)]
pub struct InputListArgs {
    #[arg(long)]
    pub project: PathBuf,
}

pub fn run_install(args: &InstallArgs) -> GdxResult<serde_json::Value> {
    let project = assert_project(&args.project)?;
    let mut files = install_gdx_addons(&project.root)?;
    if ensure_gdx_gitignore(&project.root)? {
        files.push(".gitignore".to_string());
    }
    files.sort();

    Ok(json!({
        "ok": true,
        "command": "project.install",
        "project": godot_path_string(&project.root),
        "files": files
    }))
}

pub fn run_inspect(args: &InspectArgs) -> GdxResult<serde_json::Value> {
    let project = assert_project(&args.project)?;
    let mut scenes = Vec::new();
    let mut scripts = Vec::new();
    let mut assets = Vec::new();
    let mut others = Vec::new();
    let mut visited = 0usize;
    let mut truncated = false;

    for entry in WalkDir::new(&project.root)
        .into_iter()
        .filter_entry(|entry| !is_ignored_path(&project.root, entry.path()))
    {
        let entry = entry.map_err(|err| {
            crate::error::GdxError::tool("walk_failed", format!("Cannot inspect project: {err}"))
        })?;
        if !entry.file_type().is_file() {
            continue;
        }
        let Some(res_path) = to_res_path(&project.root, entry.path()) else {
            continue;
        };
        visited += 1;
        if visited > args.max_files {
            truncated = true;
            break;
        }
        match entry
            .path()
            .extension()
            .and_then(|extension| extension.to_str())
        {
            Some("tscn" | "scn") => scenes.push(res_path),
            Some("gd" | "cs") => scripts.push(res_path),
            Some(
                "png" | "jpg" | "jpeg" | "webp" | "svg" | "glb" | "gltf" | "ogg" | "wav" | "mp3"
                | "tres" | "res" | "gdshader" | "material",
            ) => assets.push(res_path),
            _ => others.push(res_path),
        }
    }

    scenes.sort();
    scripts.sort();
    assets.sort();
    others.sort();

    Ok(json!({
        "ok": true,
        "command": "project.inspect",
        "project": godot_path_string(&project.root),
        "name": read_project_name(&project.root)?,
        "main_scene": read_main_scene(&project.root)?,
        "gdx": {
            "installed": gdx_addons_installed(&project.root),
            "files": {
                "create_scene": project.root.join(GDX_TOOLS_CREATE_SCENE_GD).is_file(),
                "automation": project.root.join(GDX_TOOLS_AUTOMATION_GD).is_file(),
                "capture_runner": project.root.join(GDX_RUNTIME_CAPTURE_RUNNER_TSCN).is_file(),
                "daemon": project.root.join(GDX_DAEMON_SERVER_TSCN).is_file()
            }
        },
        "files": {
            "scenes": scenes,
            "scripts": scripts,
            "assets": assets,
            "other": others,
            "truncated": truncated,
            "max_files": args.max_files
        }
    }))
}

pub fn run_setting_get(args: &SettingGetArgs) -> GdxResult<serde_json::Value> {
    let project = assert_project(&args.project)?;
    let value = read_project_setting(&project.root, &args.section, &args.key)?;
    Ok(json!({
        "ok": true,
        "command": "project.setting.get",
        "project": godot_path_string(&project.root),
        "section": args.section,
        "key": args.key,
        "value": value
    }))
}

pub fn run_setting_set(args: &SettingSetArgs) -> GdxResult<serde_json::Value> {
    let project = assert_project(&args.project)?;
    validate_non_empty("section", &args.section)?;
    validate_non_empty("key", &args.key)?;
    if args.raw {
        set_project_setting_raw(&project.root, &args.section, &args.key, &args.value)?;
    } else {
        set_project_setting_quoted(&project.root, &args.section, &args.key, &args.value)?;
    }
    Ok(json!({
        "ok": true,
        "command": "project.setting.set",
        "project": godot_path_string(&project.root),
        "section": args.section,
        "key": args.key,
        "raw": args.raw
    }))
}

pub fn run_setting_list(args: &SettingListArgs) -> GdxResult<serde_json::Value> {
    let project = assert_project(&args.project)?;
    let settings = list_project_settings(&project.root, &args.section)?;
    Ok(json!({
        "ok": true,
        "command": "project.setting.list",
        "project": godot_path_string(&project.root),
        "section": args.section,
        "settings": settings
    }))
}

pub fn run_autoload_add(args: &AutoloadAddArgs) -> GdxResult<serde_json::Value> {
    let project = assert_project(&args.project)?;
    validate_non_empty("name", &args.name)?;
    if !args.path.starts_with("res://") {
        return Err(GdxError::user(
            "invalid_path",
            "--path must be a res:// path",
        ));
    }
    let value = if args.global {
        format!("*{}", args.path)
    } else {
        args.path.clone()
    };
    set_project_setting_quoted(&project.root, "autoload", &args.name, &value)?;
    Ok(json!({
        "ok": true,
        "command": "project.autoload.add",
        "project": godot_path_string(&project.root),
        "name": args.name,
        "path": args.path,
        "global": args.global
    }))
}

pub fn run_autoload_remove(args: &AutoloadRemoveArgs) -> GdxResult<serde_json::Value> {
    let project = assert_project(&args.project)?;
    validate_non_empty("name", &args.name)?;
    let removed = remove_project_setting(&project.root, "autoload", &args.name)?;
    Ok(json!({
        "ok": true,
        "command": "project.autoload.remove",
        "project": godot_path_string(&project.root),
        "name": args.name,
        "removed": removed
    }))
}

pub fn run_autoload_list(args: &AutoloadListArgs) -> GdxResult<serde_json::Value> {
    let project = assert_project(&args.project)?;
    let autoloads = list_project_settings(&project.root, "autoload")?;
    Ok(json!({
        "ok": true,
        "command": "project.autoload.list",
        "project": godot_path_string(&project.root),
        "autoloads": autoloads
    }))
}

pub fn run_input_add(cli: &Cli, args: &InputAddArgs) -> GdxResult<serde_json::Value> {
    let project = assert_project(&args.project)?;
    validate_non_empty("action", &args.action)?;
    if args.keycode.is_none() && args.mouse_button.is_none() {
        return Err(GdxError::user(
            "missing_event",
            "Pass --keycode <code> or --mouse-button <code>",
        ));
    }
    let result = run_project_input(
        cli,
        project.root,
        "project_input_add",
        json!({
            "action": args.action,
            "keycode": args.keycode,
            "mouse_button": args.mouse_button,
            "deadzone": args.deadzone
        }),
    )?;
    Ok(json!({
        "ok": true,
        "command": "project.input.add",
        "result": result
    }))
}

pub fn run_input_remove(cli: &Cli, args: &InputRemoveArgs) -> GdxResult<serde_json::Value> {
    let project = assert_project(&args.project)?;
    validate_non_empty("action", &args.action)?;
    let result = run_project_input(
        cli,
        project.root,
        "project_input_remove",
        json!({ "action": args.action }),
    )?;
    Ok(json!({
        "ok": true,
        "command": "project.input.remove",
        "result": result
    }))
}

pub fn run_input_list(cli: &Cli, args: &InputListArgs) -> GdxResult<serde_json::Value> {
    let project = assert_project(&args.project)?;
    let result = run_project_input(cli, project.root, "project_input_list", json!({}))?;
    Ok(json!({
        "ok": true,
        "command": "project.input.list",
        "result": result
    }))
}

fn run_project_input(
    cli: &Cli,
    project: PathBuf,
    action: &str,
    params: serde_json::Value,
) -> GdxResult<serde_json::Value> {
    let binary = godot::locate_godot(cli.godot.as_deref())?;
    let result = godot::run_automation(binary, project, action, params, 30)?;
    if result.status_code != 0 {
        return Err(godot::godot_failed(&result));
    }
    Ok(result.last_json.unwrap_or(serde_json::Value::Null))
}

fn validate_non_empty(name: &str, value: &str) -> GdxResult<()> {
    if value.trim().is_empty() {
        Err(GdxError::user(
            format!("invalid_{name}"),
            format!("{name} must not be empty"),
        ))
    } else {
        Ok(())
    }
}

fn gdx_addons_installed(project: &Path) -> bool {
    [
        GDX_TOOLS_CREATE_SCENE_GD,
        GDX_TOOLS_AUTOMATION_GD,
        GDX_RUNTIME_CAPTURE_RUNNER_GD,
        GDX_RUNTIME_CAPTURE_RUNNER_TSCN,
        GDX_DAEMON_SERVER_GD,
        GDX_DAEMON_SERVER_TSCN,
    ]
    .iter()
    .all(|path| project.join(path).is_file())
}

fn is_ignored_path(project: &Path, path: &Path) -> bool {
    if path == project {
        return true;
    }
    let Ok(rel) = path.strip_prefix(project) else {
        return true;
    };
    let mut components = rel.components();
    match components
        .next()
        .and_then(|component| component.as_os_str().to_str())
    {
        Some(".godot" | ".gdx") => true,
        Some("addons") => matches!(
            components
                .next()
                .and_then(|component| component.as_os_str().to_str()),
            Some("gdx_tools" | "gdx_runtime" | "gdx_daemon")
        ),
        _ => false,
    }
}

fn to_res_path(project: &Path, path: &Path) -> Option<String> {
    let rel = path.strip_prefix(project).ok()?;
    Some(format!(
        "res://{}",
        rel.to_string_lossy().replace('\\', "/")
    ))
}
