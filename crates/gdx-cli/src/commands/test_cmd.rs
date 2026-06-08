use clap::Args;
use serde_json::json;

use crate::context::{validate_non_empty, validate_res_path, AppContext};
use crate::error::GdxResult;
use crate::project::godot_path_string;

#[derive(Debug, Args)]
pub struct RunArgs {
    #[arg(long)]
    pub path: String,

    #[arg(long, default_value = "run_tests")]
    pub method: String,
}

pub fn run(ctx: &AppContext, args: &RunArgs) -> GdxResult<serde_json::Value> {
    validate_res_path("--path", &args.path)?;
    validate_non_empty("method", &args.method)?;
    let project = ctx.project()?;
    let result = ctx.run_automation(
        project.root.clone(),
        "test_run",
        json!({
            "path": args.path,
            "method": args.method
        }),
        120,
    )?;
    Ok(json!({
        "ok": true,
        "command": "test.run",
        "project": godot_path_string(&project.root),
        "result": result
    }))
}
