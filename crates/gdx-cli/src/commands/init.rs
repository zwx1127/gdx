use std::fs;
use std::path::{Path, PathBuf};

use clap::Args;
use serde_json::json;

use crate::constants::{
    GDX_DAEMON_SERVER_GD, GDX_DAEMON_SERVER_TSCN, GDX_GITIGNORE_ENTRIES,
    GDX_RUNTIME_CAPTURE_RUNNER_GD, GDX_RUNTIME_CAPTURE_RUNNER_TSCN, GDX_TOOLS_AUTOMATION_GD,
    GDX_TOOLS_CREATE_SCENE_GD,
};
use crate::context::AppContext;
use crate::error::{GdxError, GdxResult};
use crate::project::{ensure_dir, godot_path_string};

#[derive(Debug, Args)]
pub struct CreateArgs {
    #[arg(long)]
    pub path: PathBuf,

    #[arg(long)]
    pub name: String,

    #[arg(long)]
    pub force: bool,
}

struct ResourceFile {
    path: &'static str,
    contents: &'static str,
}

const ADDON_FILES: &[ResourceFile] = &[
    ResourceFile {
        path: GDX_TOOLS_CREATE_SCENE_GD,
        contents: include_str!("../../resources/addons/gdx_tools/create_scene.gd"),
    },
    ResourceFile {
        path: GDX_TOOLS_AUTOMATION_GD,
        contents: include_str!("../../resources/addons/gdx_tools/automation.gd"),
    },
    ResourceFile {
        path: GDX_RUNTIME_CAPTURE_RUNNER_GD,
        contents: include_str!("../../resources/addons/gdx_runtime/capture_runner.gd"),
    },
    ResourceFile {
        path: GDX_RUNTIME_CAPTURE_RUNNER_TSCN,
        contents: include_str!("../../resources/addons/gdx_runtime/capture_runner.tscn"),
    },
    ResourceFile {
        path: GDX_DAEMON_SERVER_GD,
        contents: include_str!("../../resources/addons/gdx_daemon/daemon_server.gd"),
    },
    ResourceFile {
        path: GDX_DAEMON_SERVER_TSCN,
        contents: include_str!("../../resources/addons/gdx_daemon/daemon_server.tscn"),
    },
];

pub fn run_create(ctx: &AppContext, args: &CreateArgs) -> GdxResult<serde_json::Value> {
    if args.name.trim().is_empty() {
        return Err(GdxError::user(
            "invalid_name",
            "Project name must not be empty",
        ));
    }

    let target = ctx.abs_path(&args.path);

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
        .with_suggestion("Pass --force to write gdx scaffold files into the existing directory."));
    }

    ensure_dir(&target)?;
    let mut files = Vec::new();

    let project_text = format!(
        "config_version=5\n\n[application]\nconfig/name=\"{}\"\n",
        escape_project_name(&args.name)
    );
    write_text(&target.join("project.godot"), &project_text)?;
    files.push("project.godot".to_string());

    files.extend(install_gdx_addons(&target)?);
    if ensure_gdx_gitignore(&target)? {
        files.push(".gitignore".to_string());
    }
    files.sort();

    Ok(json!({
        "ok": true,
        "command": "project.create",
        "project": godot_path_string(&target),
        "files": files
    }))
}

pub(crate) fn install_gdx_addons(project: &Path) -> GdxResult<Vec<String>> {
    let mut files = Vec::new();
    for file in ADDON_FILES {
        write_text(&project.join(Path::new(file.path)), file.contents)?;
        files.push(file.path.to_string());
    }
    Ok(files)
}

pub(crate) fn ensure_gdx_gitignore(project: &Path) -> GdxResult<bool> {
    let path = project.join(".gitignore");
    let original = fs::read_to_string(&path).unwrap_or_default();
    let mut text = original.clone();
    for entry in GDX_GITIGNORE_ENTRIES {
        if !original.lines().any(|line| line.trim() == *entry) {
            if !text.is_empty() && !text.ends_with('\n') {
                text.push('\n');
            }
            text.push_str(entry);
            text.push('\n');
        }
    }
    if text != original {
        write_text(&path, &text)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

fn write_text(path: &Path, contents: &str) -> GdxResult<()> {
    if let Some(parent) = path.parent() {
        ensure_dir(parent)?;
    }
    fs::write(path, contents).map_err(|err| {
        GdxError::tool(
            "io_failed",
            format!("Cannot write {}: {err}", path.display()),
        )
    })
}

fn escape_project_name(name: &str) -> String {
    name.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_project(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("gdx_init_test_{}_{}", name, std::process::id()))
    }

    #[test]
    fn init_creates_minimal_project_with_gdx_addons() {
        let path = temp_project("minimal");
        let _ = fs::remove_dir_all(&path);
        let ctx = AppContext::new(None, None).unwrap();
        let args = CreateArgs {
            path: path.clone(),
            name: "hello".to_string(),
            force: false,
        };

        run_create(&ctx, &args).unwrap();

        let project = fs::read_to_string(path.join("project.godot")).unwrap();
        assert!(project.contains("config/name=\"hello\""));
        assert!(!project.contains("config/features"));
        assert!(!project.contains("run/main_scene"));
        assert!(!project.contains("renderer/rendering_method"));
        assert!(path.join("addons/gdx_tools/create_scene.gd").is_file());
        assert!(path.join("addons/gdx_tools/automation.gd").is_file());
        assert!(path.join("addons/gdx_runtime/capture_runner.gd").is_file());
        assert!(path.join("addons/gdx_daemon/daemon_server.gd").is_file());
        assert!(!path.join("scripts/main.gd").exists());
        assert!(!path.join("assets/icon.svg").exists());
        assert!(!path.join("scenes/.gitkeep").exists());

        let _ = fs::remove_dir_all(&path);
    }

    #[test]
    fn init_rejects_non_empty_target_without_force() {
        let path = temp_project("non_empty");
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        fs::write(path.join("user.txt"), "keep").unwrap();
        let ctx = AppContext::new(None, None).unwrap();
        let args = CreateArgs {
            path: path.clone(),
            name: "hello".to_string(),
            force: false,
        };

        let err = run_create(&ctx, &args).unwrap_err();

        assert_eq!(err.error, "target_not_empty");
        assert_eq!(fs::read_to_string(path.join("user.txt")).unwrap(), "keep");

        let _ = fs::remove_dir_all(&path);
    }
}
