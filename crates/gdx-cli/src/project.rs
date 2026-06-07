use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{GdxError, GdxResult};

#[derive(Debug, Clone)]
pub struct Project {
    pub root: PathBuf,
}

pub fn assert_project(path: &Path) -> GdxResult<Project> {
    let root = path.canonicalize().map_err(|err| {
        GdxError::user(
            "project_not_found",
            format!("Cannot open project directory: {err}"),
        )
    })?;
    if !root.join("project.godot").is_file() {
        return Err(GdxError::user(
            "invalid_project",
            format!("Missing project.godot in {}", root.display()),
        ));
    }
    Ok(Project { root })
}

pub fn res_to_abs(project: &Path, path: &str) -> GdxResult<PathBuf> {
    if let Some(rest) = path.strip_prefix("res://") {
        Ok(project.join(rest.replace('/', std::path::MAIN_SEPARATOR_STR)))
    } else {
        let path = PathBuf::from(path);
        if path.is_absolute() {
            Ok(path)
        } else {
            std::env::current_dir()
                .map(|cwd| cwd.join(path))
                .map_err(|err| {
                    GdxError::tool("io_failed", format!("Cannot read current directory: {err}"))
                })
        }
    }
}

pub fn abs_to_res(project: &Path, path: &Path) -> GdxResult<String> {
    let project = project.canonicalize().map_err(|err| {
        GdxError::tool(
            "io_failed",
            format!("Cannot canonicalize project path: {err}"),
        )
    })?;
    let path = path
        .canonicalize()
        .map_err(|err| GdxError::tool("io_failed", format!("Cannot canonicalize path: {err}")))?;
    let rel = path.strip_prefix(&project).map_err(|_| {
        GdxError::user(
            "path_outside_project",
            format!("{} is outside {}", path.display(), project.display()),
        )
    })?;
    Ok(format!(
        "res://{}",
        rel.to_string_lossy().replace('\\', "/")
    ))
}

pub fn ensure_dir(path: &Path) -> GdxResult<()> {
    fs::create_dir_all(path).map_err(|err| {
        GdxError::tool(
            "io_failed",
            format!("Cannot create {}: {err}", path.display()),
        )
    })
}

pub fn ensure_parent_dir(path: &Path) -> GdxResult<()> {
    if let Some(parent) = path.parent() {
        ensure_dir(parent)?;
    }
    Ok(())
}

pub fn godot_path_string(path: &Path) -> String {
    let mut value = path.to_string_lossy().into_owned();
    if cfg!(windows) {
        if let Some(rest) = value.strip_prefix(r"\\?\UNC\") {
            value = format!(r"\\{rest}");
        } else if let Some(rest) = value.strip_prefix(r"\\?\") {
            value = rest.to_string();
        }
    }
    value.replace('\\', "/")
}
