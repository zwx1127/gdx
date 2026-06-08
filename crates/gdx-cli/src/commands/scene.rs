use std::path::PathBuf;

use clap::{ArgGroup, Args};
use serde_json::json;

use crate::commands::Cli;
use crate::constants::{GDX_TOOLS_CREATE_SCENE_GD, GDX_TOOLS_CREATE_SCENE_RES};
use crate::daemon;
use crate::error::{GdxError, GdxResult};
use crate::godot::{self, GodotCommand};
use crate::project::{
    abs_to_res, assert_project, ensure_parent_dir, godot_path_string, res_to_abs,
    set_project_setting,
};

#[derive(Debug, Args)]
pub struct NewArgs {
    #[arg(long)]
    pub project: PathBuf,

    #[arg(long)]
    pub out: String,

    #[arg(long)]
    pub root_type: String,

    #[arg(long)]
    pub name: String,

    #[arg(long)]
    pub set_main: bool,

    #[arg(long)]
    pub force: bool,
}

#[derive(Debug, Args)]
pub struct TreeArgs {
    #[arg(long)]
    pub project: PathBuf,
}

#[derive(Debug, Args)]
pub struct AddNodeArgs {
    #[arg(long)]
    pub project: PathBuf,

    #[arg(long)]
    pub parent: String,

    #[arg(long = "type")]
    pub type_name: String,

    #[arg(long)]
    pub name: String,
}

#[derive(Debug, Args)]
#[command(group(
    ArgGroup::new("scene_set_value")
        .required(true)
        .args(["value", "number", "bool_value", "vec2", "vec3", "color", "resource", "node_path"])
))]
pub struct SetArgs {
    #[arg(long)]
    pub project: PathBuf,

    #[arg(long)]
    pub node: String,

    #[arg(long)]
    pub property: String,

    #[arg(long)]
    pub value: Option<String>,

    #[arg(long)]
    pub number: Option<f64>,

    #[arg(long = "bool")]
    pub bool_value: Option<bool>,

    #[arg(long, num_args = 2)]
    pub vec2: Option<Vec<f64>>,

    #[arg(long, num_args = 3)]
    pub vec3: Option<Vec<f64>>,

    #[arg(long, num_args = 3..=4)]
    pub color: Option<Vec<f64>>,

    #[arg(long)]
    pub resource: Option<String>,

    #[arg(long)]
    pub node_path: Option<String>,
}

#[derive(Debug, Args)]
pub struct SaveArgs {
    #[arg(long)]
    pub project: PathBuf,

    #[arg(long)]
    pub out: Option<String>,
}

pub fn run_new(cli: &Cli, args: &NewArgs) -> GdxResult<serde_json::Value> {
    if !args.out.starts_with("res://") {
        return Err(GdxError::user("invalid_out", "--out must be a res:// path"));
    }
    if args.root_type.trim().is_empty() {
        return Err(GdxError::user(
            "invalid_root_type",
            "--root-type must not be empty",
        ));
    }
    if args.name.trim().is_empty() {
        return Err(GdxError::user("invalid_name", "--name must not be empty"));
    }
    let project = assert_project(&args.project)?;
    if !project.root.join(GDX_TOOLS_CREATE_SCENE_GD).is_file() {
        return Err(
            GdxError::user("gdx_addons_missing", "gdx project addon files are missing")
                .with_suggestion("Run gdx project setup --project <dir> before creating scenes."),
        );
    }

    let scene_abs = res_to_abs(&project.root, &args.out)?;
    if scene_abs.exists() && !args.force {
        return Err(GdxError::user(
            "scene_exists",
            format!("Scene already exists: {}", scene_abs.display()),
        )
        .with_suggestion("Pass --force to overwrite the existing scene."));
    }
    ensure_parent_dir(&scene_abs)?;
    let binary = godot::locate_godot(cli.godot.as_deref())?;

    let result = godot::run(GodotCommand {
        binary,
        project: project.root.clone(),
        args: vec![
            "--headless".to_string(),
            "--path".to_string(),
            godot_path_string(&project.root),
            "-s".to_string(),
            GDX_TOOLS_CREATE_SCENE_RES.to_string(),
        ],
        envs: vec![
            ("GDX_SCENE_OUT".to_string(), args.out.clone()),
            ("GDX_SCENE_ROOT_TYPE".to_string(), args.root_type.clone()),
            ("GDX_SCENE_ROOT_NAME".to_string(), args.name.clone()),
            ("GDX_SCENE_FORCE".to_string(), args.force.to_string()),
        ],
        timeout_secs: 60,
    })?;

    if result.status_code != 0 {
        return Err(godot::godot_failed(&result));
    }
    if !scene_abs.is_file() {
        return Err(GdxError::tool(
            "scene_not_created",
            format!(
                "Godot completed but scene was not created: {}",
                scene_abs.display()
            ),
        )
        .with_artifact("stdout_log", godot_path_string(&result.stdout_log))
        .with_artifact("stderr_log", godot_path_string(&result.stderr_log)));
    }
    let scene_res = abs_to_res(&project.root, &scene_abs)?;
    if args.set_main {
        set_project_setting(&project.root, "application", "run/main_scene", &scene_res)?;
    }

    Ok(json!({
        "ok": true,
        "command": "scene.new",
        "project": godot_path_string(&project.root),
        "scene": scene_res,
        "scene_abs": godot_path_string(&scene_abs),
        "root": {
            "type": args.root_type,
            "name": args.name
        },
        "set_main": args.set_main,
        "godot": {
            "last_json": result.last_json
        },
        "artifacts": {
            "stdout_log": godot_path_string(&result.stdout_log),
            "stderr_log": godot_path_string(&result.stderr_log)
        }
    }))
}

pub fn run_tree(args: &TreeArgs) -> GdxResult<serde_json::Value> {
    let project = assert_project(&args.project)?;
    let result = daemon::rpc(&project.root, "scene_tree", json!({}), 10)?;
    Ok(json!({
        "ok": true,
        "command": "scene.tree",
        "project": godot_path_string(&project.root),
        "tree": result
    }))
}

pub fn run_add_node(args: &AddNodeArgs) -> GdxResult<serde_json::Value> {
    if args.parent.trim().is_empty() {
        return Err(GdxError::user(
            "invalid_parent",
            "--parent must not be empty",
        ));
    }
    if args.type_name.trim().is_empty() {
        return Err(GdxError::user("invalid_type", "--type must not be empty"));
    }
    if args.name.trim().is_empty() {
        return Err(GdxError::user("invalid_name", "--name must not be empty"));
    }
    let project = assert_project(&args.project)?;
    let result = daemon::rpc(
        &project.root,
        "add_node",
        json!({
            "parent": args.parent,
            "type": args.type_name,
            "name": args.name
        }),
        10,
    )?;
    Ok(json!({
        "ok": true,
        "command": "scene.add-node",
        "project": godot_path_string(&project.root),
        "node": result
    }))
}

pub fn run_set(args: &SetArgs) -> GdxResult<serde_json::Value> {
    if args.node.trim().is_empty() {
        return Err(GdxError::user("invalid_node", "--node must not be empty"));
    }
    if args.property.trim().is_empty() {
        return Err(GdxError::user(
            "invalid_property",
            "--property must not be empty",
        ));
    }
    let value = scene_set_value(args)?;
    let project = assert_project(&args.project)?;
    let result = daemon::rpc(
        &project.root,
        "set_property",
        json!({
            "node": args.node,
            "property": args.property,
            "value": value
        }),
        10,
    )?;
    Ok(json!({
        "ok": true,
        "command": "scene.set",
        "project": godot_path_string(&project.root),
        "result": result
    }))
}

fn scene_set_value(args: &SetArgs) -> GdxResult<serde_json::Value> {
    if let Some(value) = &args.value {
        return Ok(serde_json::Value::String(value.clone()));
    }
    if let Some(number) = args.number {
        return serde_json::Number::from_f64(number)
            .map(serde_json::Value::Number)
            .ok_or_else(|| GdxError::user("invalid_number", "--number must be finite"));
    }
    if let Some(value) = args.bool_value {
        return Ok(serde_json::Value::Bool(value));
    }
    if let Some(values) = &args.vec2 {
        return Ok(json!({ "vec2": values }));
    }
    if let Some(values) = &args.vec3 {
        return Ok(json!({ "vec3": values }));
    }
    if let Some(values) = &args.color {
        let mut color = values.clone();
        if color.len() == 3 {
            color.push(1.0);
        }
        return Ok(json!({ "color": color }));
    }
    if let Some(resource) = &args.resource {
        if !resource.starts_with("res://") {
            return Err(GdxError::user(
                "invalid_resource",
                "--resource must be a res:// path",
            ));
        }
        return Ok(json!({ "resource": resource }));
    }
    if let Some(node_path) = &args.node_path {
        return Ok(json!({ "node_path": node_path }));
    }
    Err(GdxError::user(
        "missing_value",
        "scene set requires one value flag",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_set_args() -> SetArgs {
        SetArgs {
            project: PathBuf::from("demo"),
            node: "/Title".to_string(),
            property: "text".to_string(),
            value: None,
            number: None,
            bool_value: None,
            vec2: None,
            vec3: None,
            color: None,
            resource: None,
            node_path: None,
        }
    }

    #[test]
    fn set_value_encodes_resource_wrapper_internally() {
        let mut args = base_set_args();
        args.resource = Some("res://assets/icon.svg".to_string());

        let value = scene_set_value(&args).unwrap();

        assert_eq!(value, json!({ "resource": "res://assets/icon.svg" }));
    }

    #[test]
    fn set_value_adds_default_alpha_to_color() {
        let mut args = base_set_args();
        args.color = Some(vec![0.1, 0.2, 0.3]);

        let value = scene_set_value(&args).unwrap();

        assert_eq!(value, json!({ "color": [0.1, 0.2, 0.3, 1.0] }));
    }
}

pub fn run_save(args: &SaveArgs) -> GdxResult<serde_json::Value> {
    if let Some(out) = &args.out {
        if !out.starts_with("res://") {
            return Err(GdxError::user("invalid_out", "--out must be a res:// path"));
        }
    }
    let project = assert_project(&args.project)?;
    let result = daemon::rpc(
        &project.root,
        "save_scene",
        json!({
            "out": args.out
        }),
        10,
    )?;
    Ok(json!({
        "ok": true,
        "command": "scene.save",
        "project": godot_path_string(&project.root),
        "result": result
    }))
}
