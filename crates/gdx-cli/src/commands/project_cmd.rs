use std::path::Path;

use clap::Args;
use serde_json::json;
use walkdir::WalkDir;

use crate::constants::{
    GDX_DAEMON_SERVER_GD, GDX_DAEMON_SERVER_TSCN, GDX_RUNTIME_CAPTURE_RUNNER_GD,
    GDX_RUNTIME_CAPTURE_RUNNER_TSCN, GDX_TOOLS_AUTOMATION_GD, GDX_TOOLS_CREATE_SCENE_GD,
};
use crate::context::{validate_non_empty, validate_res_path, AppContext};
use crate::error::{GdxError, GdxResult};
use crate::project::{
    godot_path_string, list_project_settings, read_main_scene, read_project_name,
    read_project_setting, remove_project_setting, res_to_abs, set_project_setting_quoted,
    set_project_setting_raw,
};

use super::init::{ensure_gdx_gitignore, install_gdx_addons};

#[derive(Debug, Args)]
pub struct InstallArgs {}

#[derive(Debug, Args)]
pub struct InspectArgs {
    #[arg(long, default_value_t = 500)]
    pub max_files: usize,
}

#[derive(Debug, Args)]
pub struct SettingGetArgs {
    #[arg(long)]
    pub section: String,

    #[arg(long)]
    pub key: String,
}

#[derive(Debug, Args)]
pub struct SettingSetArgs {
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
    pub section: String,
}

#[derive(Debug, Args)]
pub struct AutoloadAddArgs {
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
    pub name: String,
}

#[derive(Debug, Args)]
pub struct AutoloadListArgs {}

#[derive(Debug, Args)]
pub struct InputAddArgs {
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
    pub action: String,
}

#[derive(Debug, Args)]
pub struct InputListArgs {}

pub fn run_install(ctx: &AppContext, _args: &InstallArgs) -> GdxResult<serde_json::Value> {
    let project = ctx.project()?;
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

pub fn run_inspect(ctx: &AppContext, args: &InspectArgs) -> GdxResult<serde_json::Value> {
    let project = ctx.project()?;
    let mut scenes = Vec::new();
    let mut scripts = Vec::new();
    let mut assets = Vec::new();
    let mut others = Vec::new();
    let mut scene_count = 0usize;
    let mut script_count = 0usize;
    let mut asset_count = 0usize;
    let mut other_count = 0usize;

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
        match entry
            .path()
            .extension()
            .and_then(|extension| extension.to_str())
        {
            Some("tscn" | "scn") => {
                scene_count += 1;
                push_limited(&mut scenes, res_path, args.max_files);
            }
            Some("gd" | "cs") => {
                script_count += 1;
                push_limited(&mut scripts, res_path, args.max_files);
            }
            Some(
                "png" | "jpg" | "jpeg" | "webp" | "svg" | "glb" | "gltf" | "ogg" | "wav" | "mp3"
                | "tres" | "res" | "gdshader" | "material",
            ) => {
                asset_count += 1;
                push_limited(&mut assets, res_path, args.max_files);
            }
            _ => {
                other_count += 1;
                push_limited(&mut others, res_path, args.max_files);
            }
        }
    }

    scenes.sort();
    scripts.sort();
    assets.sort();
    others.sort();
    let main_scene = read_main_scene(&project.root)?;
    let main_scene_exists = main_scene
        .as_deref()
        .map(|path| res_to_abs(&project.root, path).map(|path| path.is_file()))
        .transpose()?;
    let autoloads = list_project_settings(&project.root, "autoload")?;
    let input_map = list_project_settings(&project.root, "input")?;
    let scenes_truncated = scene_count > scenes.len();
    let scripts_truncated = script_count > scripts.len();
    let assets_truncated = asset_count > assets.len();
    let other_truncated = other_count > others.len();
    let truncated = scenes_truncated || scripts_truncated || assets_truncated || other_truncated;

    Ok(json!({
        "ok": true,
        "command": "project.inspect",
        "project": godot_path_string(&project.root),
        "name": read_project_name(&project.root)?,
        "main_scene": main_scene,
        "main_scene_exists": main_scene_exists,
        "autoloads": autoloads,
        "input_map": input_map,
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
            "max_files": args.max_files,
            "truncated_by_kind": {
                "scenes": scenes_truncated,
                "scripts": scripts_truncated,
                "assets": assets_truncated,
                "other": other_truncated
            }
        },
        "summary": {
            "scenes": scene_count,
            "scripts": script_count,
            "assets": asset_count,
            "other": other_count,
            "files": scene_count + script_count + asset_count + other_count
        }
    }))
}

pub fn run_setting_get(ctx: &AppContext, args: &SettingGetArgs) -> GdxResult<serde_json::Value> {
    let project = ctx.project()?;
    let value = read_project_setting(&project.root, &args.section, &args.key)?;
    Ok(json!({
        "ok": true,
        "command": "setting.get",
        "project": godot_path_string(&project.root),
        "section": args.section,
        "key": args.key,
        "value": value
    }))
}

pub fn run_setting_set(ctx: &AppContext, args: &SettingSetArgs) -> GdxResult<serde_json::Value> {
    let project = ctx.project()?;
    validate_non_empty("section", &args.section)?;
    validate_non_empty("key", &args.key)?;
    if args.raw {
        set_project_setting_raw(&project.root, &args.section, &args.key, &args.value)?;
    } else {
        set_project_setting_quoted(&project.root, &args.section, &args.key, &args.value)?;
    }
    Ok(json!({
        "ok": true,
        "command": "setting.set",
        "project": godot_path_string(&project.root),
        "section": args.section,
        "key": args.key,
        "raw": args.raw
    }))
}

pub fn run_setting_list(ctx: &AppContext, args: &SettingListArgs) -> GdxResult<serde_json::Value> {
    let project = ctx.project()?;
    let settings = list_project_settings(&project.root, &args.section)?;
    Ok(json!({
        "ok": true,
        "command": "setting.list",
        "project": godot_path_string(&project.root),
        "section": args.section,
        "settings": settings
    }))
}

pub fn run_autoload_add(ctx: &AppContext, args: &AutoloadAddArgs) -> GdxResult<serde_json::Value> {
    let project = ctx.project()?;
    validate_non_empty("name", &args.name)?;
    validate_res_path("--path", &args.path)?;
    let value = if args.global {
        format!("*{}", args.path)
    } else {
        args.path.clone()
    };
    set_project_setting_quoted(&project.root, "autoload", &args.name, &value)?;
    Ok(json!({
        "ok": true,
        "command": "autoload.add",
        "project": godot_path_string(&project.root),
        "name": args.name,
        "path": args.path,
        "global": args.global
    }))
}

pub fn run_autoload_remove(
    ctx: &AppContext,
    args: &AutoloadRemoveArgs,
) -> GdxResult<serde_json::Value> {
    let project = ctx.project()?;
    validate_non_empty("name", &args.name)?;
    let removed = remove_project_setting(&project.root, "autoload", &args.name)?;
    Ok(json!({
        "ok": true,
        "command": "autoload.remove",
        "project": godot_path_string(&project.root),
        "name": args.name,
        "removed": removed
    }))
}

pub fn run_autoload_list(
    ctx: &AppContext,
    _args: &AutoloadListArgs,
) -> GdxResult<serde_json::Value> {
    let project = ctx.project()?;
    let autoloads = list_project_settings(&project.root, "autoload")?;
    Ok(json!({
        "ok": true,
        "command": "autoload.list",
        "project": godot_path_string(&project.root),
        "autoloads": autoloads
    }))
}

pub fn run_input_add(ctx: &AppContext, args: &InputAddArgs) -> GdxResult<serde_json::Value> {
    let project = ctx.project()?;
    validate_non_empty("action", &args.action)?;
    if args.keycode.is_none() && args.mouse_button.is_none() {
        return Err(GdxError::user(
            "missing_event",
            "Pass --keycode <code> or --mouse-button <code>",
        ));
    }
    let result = ctx.run_automation(
        project.root.clone(),
        "project_input_add",
        json!({
            "action": args.action,
            "keycode": args.keycode,
            "mouse_button": args.mouse_button,
            "deadzone": args.deadzone
        }),
        30,
    )?;
    Ok(json!({
        "ok": true,
        "command": "input-map.add",
        "project": godot_path_string(&project.root),
        "result": result
    }))
}

pub fn run_input_remove(ctx: &AppContext, args: &InputRemoveArgs) -> GdxResult<serde_json::Value> {
    let project = ctx.project()?;
    validate_non_empty("action", &args.action)?;
    let result = ctx.run_automation(
        project.root.clone(),
        "project_input_remove",
        json!({ "action": args.action }),
        30,
    )?;
    Ok(json!({
        "ok": true,
        "command": "input-map.remove",
        "project": godot_path_string(&project.root),
        "result": result
    }))
}

pub fn run_input_list(ctx: &AppContext, _args: &InputListArgs) -> GdxResult<serde_json::Value> {
    let project = ctx.project()?;
    let result = ctx.run_automation(project.root.clone(), "project_input_list", json!({}), 30)?;
    Ok(json!({
        "ok": true,
        "command": "input-map.list",
        "project": godot_path_string(&project.root),
        "result": result
    }))
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
        return false;
    }
    let Ok(rel) = path.strip_prefix(project) else {
        return true;
    };
    if path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| matches!(extension, "import" | "uid"))
        .unwrap_or(false)
    {
        return true;
    }
    let mut components = rel.components();
    match components
        .next()
        .and_then(|component| component.as_os_str().to_str())
    {
        Some(".godot" | ".gdx" | ".git" | ".agent-relay") => true,
        Some("addons") => matches!(
            components
                .next()
                .and_then(|component| component.as_os_str().to_str()),
            Some("gdx_tools" | "gdx_runtime" | "gdx_daemon")
        ),
        _ => false,
    }
}

fn push_limited(values: &mut Vec<String>, value: String, max: usize) {
    if values.len() < max {
        values.push(value);
    }
}

fn to_res_path(project: &Path, path: &Path) -> Option<String> {
    let rel = path.strip_prefix(project).ok()?;
    Some(format!(
        "res://{}",
        rel.to_string_lossy().replace('\\', "/")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_project() -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "gdx_inspect_{}_{}",
            std::process::id(),
            uuid::Uuid::new_v4().simple()
        ));
        fs::create_dir_all(path.join("scenes")).unwrap();
        fs::create_dir_all(path.join("scripts")).unwrap();
        fs::create_dir_all(path.join("assets")).unwrap();
        fs::create_dir_all(path.join(".godot")).unwrap();
        fs::create_dir_all(path.join(".gdx")).unwrap();
        fs::create_dir_all(path.join("addons").join("gdx_tools")).unwrap();
        fs::write(
            path.join("project.godot"),
            "config_version=5\n\n[application]\nconfig/name=\"Inspect Demo\"\nrun/main_scene=\"res://scenes/main.tscn\"\n\n[autoload]\nGame=\"*res://scripts/main.gd\"\n\n[input]\nui_accept={}\n",
        )
        .unwrap();
        fs::write(
            path.join("scenes").join("main.tscn"),
            "[gd_scene format=3]\n",
        )
        .unwrap();
        fs::write(path.join("scripts").join("main.gd"), "extends Node\n").unwrap();
        fs::write(path.join("assets").join("icon.png"), [0u8, 1, 2]).unwrap();
        fs::write(path.join("assets").join("icon.png.import"), "metadata").unwrap();
        fs::write(path.join(".godot").join("ignored.gd"), "extends Node\n").unwrap();
        fs::write(
            path.join("addons").join("gdx_tools").join("automation.gd"),
            "extends SceneTree\n",
        )
        .unwrap();
        path
    }

    #[test]
    fn inspect_walks_project_root_and_ignores_generated_files() {
        let project = temp_project();
        let ctx = AppContext {
            godot: None,
            project: Some(project.clone()),
            cwd: project.clone(),
        };

        let value = run_inspect(&ctx, &InspectArgs { max_files: 10 }).unwrap();

        assert_eq!(value["main_scene"], "res://scenes/main.tscn");
        assert_eq!(value["main_scene_exists"], true);
        assert_eq!(value["summary"]["scenes"], 1);
        assert_eq!(value["summary"]["scripts"], 1);
        assert_eq!(value["summary"]["assets"], 1);
        assert!(value["files"]["scenes"]
            .as_array()
            .unwrap()
            .contains(&json!("res://scenes/main.tscn")));
        assert!(value["files"]["scripts"]
            .as_array()
            .unwrap()
            .contains(&json!("res://scripts/main.gd")));
        assert!(value["files"]["assets"]
            .as_array()
            .unwrap()
            .contains(&json!("res://assets/icon.png")));
        assert!(!value["files"]["scripts"]
            .as_array()
            .unwrap()
            .contains(&json!("res://.godot/ignored.gd")));

        let _ = fs::remove_dir_all(project);
    }

    #[test]
    fn inspect_truncates_each_kind_independently() {
        let project = temp_project();
        fs::write(project.join("scripts").join("second.gd"), "extends Node\n").unwrap();
        let ctx = AppContext {
            godot: None,
            project: Some(project.clone()),
            cwd: project.clone(),
        };

        let value = run_inspect(&ctx, &InspectArgs { max_files: 1 }).unwrap();

        assert_eq!(value["summary"]["scripts"], 2);
        assert_eq!(value["files"]["scripts"].as_array().unwrap().len(), 1);
        assert_eq!(value["files"]["assets"].as_array().unwrap().len(), 1);
        assert_eq!(value["files"]["truncated_by_kind"]["scripts"], true);
        assert_eq!(value["files"]["truncated_by_kind"]["assets"], false);

        let _ = fs::remove_dir_all(project);
    }
}
