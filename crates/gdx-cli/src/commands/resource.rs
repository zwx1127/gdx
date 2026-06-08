use std::path::PathBuf;

use clap::Args;
use serde_json::json;

use crate::context::{read_json_file, validate_res_path, AppContext};
use crate::error::{GdxError, GdxResult};
use crate::project::godot_path_string;

#[derive(Debug, Args)]
pub struct CreateArgs {
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
    pub path: String,
}

pub fn run_create(ctx: &AppContext, args: &CreateArgs) -> GdxResult<serde_json::Value> {
    validate_res_path("--out", &args.out)?;
    if args.type_name.trim().is_empty() {
        return Err(GdxError::user("invalid_type", "--type must not be empty"));
    }
    let project = ctx.project()?;
    let properties = match &args.properties {
        Some(path) => read_json_file(ctx, path)?,
        None => json!({}),
    };
    if !properties.is_object() {
        return Err(GdxError::user(
            "invalid_properties",
            "--properties must point to a JSON object",
        ));
    }
    let result = ctx.run_automation(
        project.root.clone(),
        "resource_create",
        json!({
            "type": args.type_name,
            "out": args.out,
            "properties": properties
        }),
        60,
    )?;
    Ok(json!({
        "ok": true,
        "command": "resource.create",
        "project": godot_path_string(&project.root),
        "result": result
    }))
}

pub fn run_inspect(ctx: &AppContext, args: &InspectArgs) -> GdxResult<serde_json::Value> {
    validate_res_path("--path", &args.path)?;
    let project = ctx.project()?;
    let result = ctx.run_automation(
        project.root.clone(),
        "resource_inspect",
        json!({ "path": args.path }),
        30,
    )?;
    Ok(json!({
        "ok": true,
        "command": "resource.inspect",
        "project": godot_path_string(&project.root),
        "resource": result
    }))
}
