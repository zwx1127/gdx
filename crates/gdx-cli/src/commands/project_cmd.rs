use std::path::{Path, PathBuf};

use clap::Args;
use serde_json::json;
use walkdir::WalkDir;

use crate::constants::{
    GDX_DAEMON_SERVER_GD, GDX_DAEMON_SERVER_TSCN, GDX_RUNTIME_CAPTURE_RUNNER_GD,
    GDX_RUNTIME_CAPTURE_RUNNER_TSCN, GDX_TOOLS_CREATE_SCENE_GD,
};
use crate::error::GdxResult;
use crate::project::{assert_project, godot_path_string, read_main_scene, read_project_name};

use super::init::{ensure_gdx_gitignore, install_gdx_addons};

#[derive(Debug, Args)]
pub struct SetupArgs {
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

pub fn run_setup(args: &SetupArgs) -> GdxResult<serde_json::Value> {
    let project = assert_project(&args.project)?;
    let mut files = install_gdx_addons(&project.root)?;
    if ensure_gdx_gitignore(&project.root)? {
        files.push(".gitignore".to_string());
    }
    files.sort();

    Ok(json!({
        "ok": true,
        "command": "project.setup",
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
                "png" | "jpg" | "jpeg" | "webp" | "svg" | "glb" | "gltf" | "ogg" | "wav" | "mp3",
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

fn gdx_addons_installed(project: &Path) -> bool {
    [
        GDX_TOOLS_CREATE_SCENE_GD,
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
