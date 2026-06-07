use std::fs;
use std::path::{Path, PathBuf};

use clap::Args;
use serde_json::json;
use walkdir::WalkDir;

use crate::error::{GdxError, GdxResult};
use crate::project::{ensure_dir, godot_path_string};

#[derive(Debug, Args)]
pub struct BasicArgs {
    #[arg(long)]
    pub path: PathBuf,

    #[arg(long)]
    pub name: String,

    #[arg(long)]
    pub force: bool,
}

pub fn run_basic(args: &BasicArgs) -> GdxResult<serde_json::Value> {
    if args.name.trim().is_empty() {
        return Err(GdxError::user(
            "invalid_name",
            "Project name must not be empty",
        ));
    }

    let target = if args.path.is_absolute() {
        args.path.clone()
    } else {
        std::env::current_dir()
            .map_err(|err| {
                GdxError::tool("io_failed", format!("Cannot read current directory: {err}"))
            })?
            .join(&args.path)
    };

    if target.exists()
        && target
            .read_dir()
            .map(|mut it| it.next().is_some())
            .unwrap_or(true)
        && !args.force
    {
        return Err(GdxError::user(
            "target_not_empty",
            format!("Target directory is not empty: {}", target.display()),
        )
        .with_suggestion("Pass --force to copy template files into the existing directory."));
    }

    ensure_dir(&target)?;
    let template = template_basic_path();
    let mut files = Vec::new();

    for entry in WalkDir::new(&template) {
        let entry = entry
            .map_err(|err| GdxError::tool("walk_failed", format!("Cannot walk template: {err}")))?;
        let source = entry.path();
        let rel = source.strip_prefix(&template).map_err(|err| {
            GdxError::tool(
                "path_failed",
                format!("Cannot compute template relative path: {err}"),
            )
        })?;
        if rel.as_os_str().is_empty() {
            continue;
        }
        let dest = target.join(rel);
        if entry.file_type().is_dir() {
            ensure_dir(&dest)?;
            continue;
        }
        if let Some(parent) = dest.parent() {
            ensure_dir(parent)?;
        }
        if rel == Path::new("project.godot") {
            let text = fs::read_to_string(source).map_err(|err| {
                GdxError::tool(
                    "io_failed",
                    format!("Cannot read {}: {err}", source.display()),
                )
            })?;
            fs::write(&dest, text.replace("{{name}}", &args.name)).map_err(|err| {
                GdxError::tool(
                    "io_failed",
                    format!("Cannot write {}: {err}", dest.display()),
                )
            })?;
        } else {
            fs::copy(source, &dest).map_err(|err| {
                GdxError::tool(
                    "io_failed",
                    format!(
                        "Cannot copy {} to {}: {err}",
                        source.display(),
                        dest.display()
                    ),
                )
            })?;
        }
        files.push(rel.to_string_lossy().replace('\\', "/"));
    }

    let gitignore = ".godot/\n.gdx/runs/\nexport/\n";
    fs::write(target.join(".gitignore"), gitignore).map_err(|err| {
        GdxError::tool(
            "io_failed",
            format!(
                "Cannot write {}: {err}",
                target.join(".gitignore").display()
            ),
        )
    })?;
    files.push(".gitignore".to_string());
    files.sort();

    Ok(json!({
        "ok": true,
        "command": "init.basic",
        "project": godot_path_string(&target),
        "files": files
    }))
}

fn template_basic_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("templates")
        .join("basic")
}
