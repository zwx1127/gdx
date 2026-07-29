use clap::Args;
use serde_json::json;

use crate::context::{validate_non_empty, validate_res_path, AppContext};
use crate::error::{GdxError, GdxResult};
use crate::godot;
use crate::project::{
    godot_path_string, list_project_settings, read_main_scene, read_project_name,
    read_project_setting, remove_project_setting, res_to_abs, set_project_setting_quoted,
    set_project_setting_raw,
};

use super::addons::{self, AddonUpdateOptions};

#[derive(Debug, Args)]
pub struct UpdateArgs {
    #[arg(long)]
    pub check: bool,

    #[arg(long)]
    pub force: bool,
}

#[derive(Debug, Args)]
pub struct InspectArgs {}

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

pub fn run_update(ctx: &AppContext, args: &UpdateArgs) -> GdxResult<serde_json::Value> {
    let project = ctx.project()?;
    let report = addons::update_gdx_addons(
        &project.root,
        AddonUpdateOptions {
            check: args.check,
            force: args.force,
        },
    )?;

    let changes: Vec<_> = report
        .files
        .into_iter()
        .filter(|file| file.action != "unchanged")
        .collect();
    Ok(json!({
        "ok": true,
        "command": "project.update",
        "project": godot_path_string(&project.root),
        "up_to_date": report.up_to_date,
        "changed": report.changed,
        "check": report.check,
        "restart_daemon_required": report.restart_daemon_required,
        "changes": changes
    }))
}

pub fn run_inspect(ctx: &AppContext, _args: &InspectArgs) -> GdxResult<serde_json::Value> {
    let project = ctx.project()?;
    let main_scene = read_main_scene(&project.root)?;
    let main_scene_exists = main_scene
        .as_deref()
        .map(|path| res_to_abs(&project.root, path).map(|path| path.is_file()))
        .transpose()?;
    let gdx_addons = addons::inspect_gdx_addons(&project.root)?;

    Ok(json!({
        "ok": true,
        "command": "project.inspect",
        "project": godot_path_string(&project.root),
        "name": read_project_name(&project.root)?,
        "main_scene": main_scene,
        "main_scene_exists": main_scene_exists,
        "language": godot::project_language_mode(&project.root).as_str(),
        "gdx": {
            "installed": gdx_addons.installed,
            "up_to_date": gdx_addons.up_to_date
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    const AUTOMATION_GD: &str = "addons/gdx_tools/automation.gd";

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

    fn temp_minimal_project(name: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "gdx_project_{}_{}_{}",
            name,
            std::process::id(),
            uuid::Uuid::new_v4().simple()
        ));
        fs::create_dir_all(&path).unwrap();
        fs::write(
            path.join("project.godot"),
            "config_version=5\n\n[application]\nconfig/name=\"Update Demo\"\n",
        )
        .unwrap();
        path
    }

    fn context_for(project: &std::path::Path) -> AppContext {
        AppContext::new(None, Some(project.to_path_buf())).unwrap()
    }

    fn update_args(check: bool, force: bool) -> UpdateArgs {
        UpdateArgs { check, force }
    }

    fn file_report<'a>(value: &'a serde_json::Value, path: &str) -> &'a serde_json::Value {
        value["changes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|file| file["path"] == path)
            .unwrap()
    }

    #[test]
    fn update_installs_missing_gdx_addons() {
        let project = temp_minimal_project("missing");
        fs::write(project.join(".gitignore"), "artifacts/\n").unwrap();
        let ctx = context_for(&project);

        let value = run_update(&ctx, &update_args(false, false)).unwrap();

        assert_eq!(value["command"], "project.update");
        assert_eq!(value["changed"], true);
        assert_eq!(value["up_to_date"], true);
        assert_eq!(value["restart_daemon_required"], true);
        let automation = file_report(&value, AUTOMATION_GD);
        assert_eq!(automation["status_before"], "missing");
        assert_eq!(automation["action"], "created");
        assert!(project.join(AUTOMATION_GD).is_file());
        assert_eq!(
            fs::read_to_string(project.join(".gitignore")).unwrap(),
            "artifacts/\n"
        );

        let _ = fs::remove_dir_all(project);
    }

    #[test]
    fn update_refreshes_outdated_gdx_addons() {
        let project = temp_minimal_project("outdated");
        let old_path = project.join(AUTOMATION_GD);
        fs::create_dir_all(old_path.parent().unwrap()).unwrap();
        fs::write(&old_path, "extends SceneTree\n# old runtime\n").unwrap();
        let ctx = context_for(&project);

        let value = run_update(&ctx, &update_args(false, false)).unwrap();

        let automation = file_report(&value, AUTOMATION_GD);
        assert_eq!(automation["status_before"], "outdated");
        assert_eq!(automation["action"], "updated");
        assert_eq!(
            fs::read_to_string(project.join(AUTOMATION_GD)).unwrap(),
            include_str!("../../resources/addons/gdx_tools/automation.gd")
        );

        let _ = fs::remove_dir_all(project);
    }

    #[test]
    fn update_check_reports_without_writing() {
        let project = temp_minimal_project("check");
        let old_path = project.join(AUTOMATION_GD);
        fs::create_dir_all(old_path.parent().unwrap()).unwrap();
        fs::write(&old_path, "extends SceneTree\n# old runtime\n").unwrap();
        let ctx = context_for(&project);

        let value = run_update(&ctx, &update_args(true, false)).unwrap();

        assert_eq!(value["changed"], false);
        assert_eq!(value["up_to_date"], false);
        assert_eq!(value["restart_daemon_required"], false);
        let automation = file_report(&value, AUTOMATION_GD);
        assert_eq!(automation["status_before"], "outdated");
        assert_eq!(automation["action"], "would_update");
        assert_eq!(
            fs::read_to_string(project.join(AUTOMATION_GD)).unwrap(),
            "extends SceneTree\n# old runtime\n"
        );

        let _ = fs::remove_dir_all(project);
    }

    #[test]
    fn update_current_gdx_addons_is_noop() {
        let project = temp_minimal_project("current");
        let ctx = context_for(&project);
        run_update(&ctx, &update_args(false, false)).unwrap();

        let value = run_update(&ctx, &update_args(false, false)).unwrap();

        assert_eq!(value["changed"], false);
        assert_eq!(value["up_to_date"], true);
        assert_eq!(value["restart_daemon_required"], false);
        assert!(value["changes"].as_array().unwrap().is_empty());

        let _ = fs::remove_dir_all(project);
    }

    #[test]
    fn inspect_reports_missing_outdated_and_current_gdx_addons() {
        let project = temp_minimal_project("inspect_gdx");
        let ctx = context_for(&project);

        let missing = run_inspect(&ctx, &InspectArgs {}).unwrap();
        assert_eq!(missing["gdx"]["installed"], false);
        assert_eq!(missing["gdx"]["up_to_date"], false);

        run_update(&ctx, &update_args(false, false)).unwrap();
        let current = run_inspect(&ctx, &InspectArgs {}).unwrap();
        assert_eq!(current["gdx"]["installed"], true);
        assert_eq!(current["gdx"]["up_to_date"], true);

        fs::write(
            project.join(AUTOMATION_GD),
            "extends SceneTree\n# old runtime\n",
        )
        .unwrap();
        let outdated = run_inspect(&ctx, &InspectArgs {}).unwrap();
        assert_eq!(outdated["gdx"]["installed"], true);
        assert_eq!(outdated["gdx"]["up_to_date"], false);

        let _ = fs::remove_dir_all(project);
    }

    #[test]
    fn inspect_reports_only_project_identity_and_tool_status() {
        let project = temp_project();
        let ctx = context_for(&project);

        let value = run_inspect(&ctx, &InspectArgs {}).unwrap();

        assert_eq!(value["main_scene"], "res://scenes/main.tscn");
        assert_eq!(value["main_scene_exists"], true);
        assert_eq!(value["name"], "Inspect Demo");
        assert_eq!(value["language"], "gdscript");
        assert!(value.get("files").is_none());
        assert!(value.get("summary").is_none());

        let _ = fs::remove_dir_all(project);
    }
}
