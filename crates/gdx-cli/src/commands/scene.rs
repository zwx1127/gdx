use std::fs;
use std::path::PathBuf;

use clap::Args;
use gdx_schema::SceneSpec;
use serde_json::json;

use crate::commands::Cli;
use crate::daemon;
use crate::error::{GdxError, GdxResult};
use crate::godot::{self, GodotCommand};
use crate::project::{
    abs_to_res, assert_project, ensure_parent_dir, godot_path_string, res_to_abs,
};

#[derive(Debug, Args)]
pub struct BuildArgs {
    #[arg(long)]
    pub project: PathBuf,

    #[arg(long)]
    pub spec: PathBuf,

    #[arg(long)]
    pub out: String,
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
pub struct SetArgs {
    #[arg(long)]
    pub project: PathBuf,

    #[arg(long)]
    pub node: String,

    #[arg(long)]
    pub property: String,

    #[arg(long)]
    pub value_json: String,
}

#[derive(Debug, Args)]
pub struct SaveArgs {
    #[arg(long)]
    pub project: PathBuf,

    #[arg(long)]
    pub out: Option<String>,
}

pub fn run_build(cli: &Cli, args: &BuildArgs) -> GdxResult<serde_json::Value> {
    if !args.out.starts_with("res://") {
        return Err(GdxError::user("invalid_out", "--out must be a res:// path"));
    }
    let project = assert_project(&args.project)?;
    let spec_path = args.spec.canonicalize().map_err(|err| {
        GdxError::user("spec_not_found", format!("Cannot open scene spec: {err}"))
    })?;
    let text = fs::read_to_string(&spec_path).map_err(|err| {
        GdxError::user("read_spec_failed", format!("Cannot read scene spec: {err}"))
    })?;
    let spec: SceneSpec = serde_json::from_str(&text).map_err(|err| {
        GdxError::validation(
            "invalid_scene_spec",
            format!("Invalid scene spec JSON: {err}"),
        )
    })?;
    spec.validate_minimal()
        .map_err(|err| GdxError::validation("invalid_scene_spec", err))?;

    let scene_abs = res_to_abs(&project.root, &args.out)?;
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
            "res://addons/gdx_tools/build_scene.gd".to_string(),
        ],
        envs: vec![
            ("GDX_SCENE_SPEC".to_string(), godot_path_string(&spec_path)),
            ("GDX_SCENE_OUT".to_string(), args.out.clone()),
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

    Ok(json!({
        "ok": true,
        "command": "scene.build",
        "project": godot_path_string(&project.root),
        "scene": scene_res,
        "scene_abs": godot_path_string(&scene_abs),
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
    let value: serde_json::Value = serde_json::from_str(&args.value_json)
        .unwrap_or_else(|_| serde_json::Value::String(args.value_json.clone()));
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
