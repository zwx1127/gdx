use std::fs;
use std::path::PathBuf;

use clap::Args;
use serde_json::json;

use crate::commands::Cli;
use crate::error::{GdxError, GdxResult};
use crate::godot;
use crate::project::{assert_project, godot_path_string};

#[derive(Debug, Args)]
pub struct CreateArgs {
    #[arg(long)]
    pub project: PathBuf,

    #[arg(long = "type")]
    pub type_name: String,

    #[arg(long)]
    pub out: String,

    #[arg(long)]
    pub properties: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct InspectArgs {
    #[arg(long)]
    pub project: PathBuf,

    #[arg(long)]
    pub path: String,
}

pub fn run_create(cli: &Cli, args: &CreateArgs) -> GdxResult<serde_json::Value> {
    validate_res_path("--out", &args.out)?;
    if args.type_name.trim().is_empty() {
        return Err(GdxError::user("invalid_type", "--type must not be empty"));
    }
    let project = assert_project(&args.project)?;
    let properties = match &args.properties {
        Some(path) => read_json_file(path)?,
        None => json!({}),
    };
    if !properties.is_object() {
        return Err(GdxError::user(
            "invalid_properties",
            "--properties must point to a JSON object",
        ));
    }
    let binary = godot::locate_godot(cli.godot.as_deref())?;
    let result = godot::run_automation(
        binary,
        project.root.clone(),
        "resource_create",
        json!({
            "type": args.type_name,
            "out": args.out,
            "properties": properties
        }),
        60,
    )?;
    if result.status_code != 0 {
        return Err(godot::godot_failed(&result));
    }
    Ok(json!({
        "ok": true,
        "command": "resource.create",
        "project": godot_path_string(&project.root),
        "result": result.last_json
    }))
}

pub fn run_inspect(cli: &Cli, args: &InspectArgs) -> GdxResult<serde_json::Value> {
    validate_res_path("--path", &args.path)?;
    let project = assert_project(&args.project)?;
    let binary = godot::locate_godot(cli.godot.as_deref())?;
    let result = godot::run_automation(
        binary,
        project.root.clone(),
        "resource_inspect",
        json!({ "path": args.path }),
        30,
    )?;
    if result.status_code != 0 {
        return Err(godot::godot_failed(&result));
    }
    Ok(json!({
        "ok": true,
        "command": "resource.inspect",
        "project": godot_path_string(&project.root),
        "resource": result.last_json
    }))
}

fn read_json_file(path: &PathBuf) -> GdxResult<serde_json::Value> {
    let path = if path.is_absolute() {
        path.clone()
    } else {
        std::env::current_dir()
            .map_err(|err| {
                GdxError::tool("io_failed", format!("Cannot read current directory: {err}"))
            })?
            .join(path)
    };
    let text = fs::read_to_string(&path).map_err(|err| {
        GdxError::tool(
            "io_failed",
            format!("Cannot read {}: {err}", path.display()),
        )
    })?;
    serde_json::from_str(&text).map_err(|err| {
        GdxError::user(
            "invalid_json",
            format!("{} must contain valid JSON: {err}", path.display()),
        )
    })
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
