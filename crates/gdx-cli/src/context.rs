use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::error::{GdxError, GdxResult};
use crate::godot;
use crate::project::{assert_project, Project};

#[derive(Debug, Clone)]
pub struct AppContext {
    pub godot: Option<PathBuf>,
    pub project: Option<PathBuf>,
    pub cwd: PathBuf,
}

impl AppContext {
    pub fn new(godot: Option<PathBuf>, project: Option<PathBuf>) -> GdxResult<Self> {
        let cwd = std::env::current_dir().map_err(|err| {
            GdxError::tool("io_failed", format!("Cannot read current directory: {err}"))
        })?;
        Ok(Self {
            godot,
            project,
            cwd,
        })
    }

    pub fn project(&self) -> GdxResult<Project> {
        let path = self.project.as_deref().ok_or_else(|| {
            GdxError::user(
                "missing_project",
                "Pass --project <dir> for commands that operate on a Godot project",
            )
        })?;
        assert_project(path)
    }

    pub fn locate_godot(&self) -> GdxResult<PathBuf> {
        godot::locate_godot(self.godot.as_deref())
    }

    pub fn abs_path(&self, path: &Path) -> PathBuf {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.cwd.join(path)
        }
    }

    pub fn run_automation(
        &self,
        project: PathBuf,
        action: &str,
        params: Value,
        timeout_secs: u64,
    ) -> GdxResult<Value> {
        let binary = self.locate_godot()?;
        let result = godot::run_automation(binary, project, action, params, timeout_secs)?;
        if result.status_code != 0 {
            return Err(godot::godot_failed(&result));
        }
        Ok(result.last_json.unwrap_or(Value::Null))
    }
}

pub fn validate_non_empty(name: &str, value: &str) -> GdxResult<()> {
    if value.trim().is_empty() {
        Err(GdxError::user(
            format!("invalid_{name}"),
            format!("{name} must not be empty"),
        ))
    } else {
        Ok(())
    }
}

pub fn validate_res_path(label: &str, value: &str) -> GdxResult<()> {
    if value.starts_with("res://") {
        Ok(())
    } else {
        Err(GdxError::user(
            "invalid_res_path",
            format!("{label} must be a res:// path"),
        ))
    }
}

pub fn read_json_file(ctx: &AppContext, path: &Path) -> GdxResult<Value> {
    let path = ctx.abs_path(path);
    let text = std::fs::read_to_string(&path).map_err(|err| {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> AppContext {
        AppContext::new(None, None).unwrap()
    }

    #[test]
    fn reports_missing_project_context() {
        let err = ctx().project().unwrap_err();

        assert_eq!(err.error, "missing_project");
    }

    #[test]
    fn resolves_relative_paths_against_cwd() {
        let ctx = ctx();
        let path = ctx.abs_path(Path::new("scene.json"));

        assert_eq!(path, ctx.cwd.join("scene.json"));
    }

    #[test]
    fn validates_res_paths() {
        assert!(validate_res_path("--path", "res://main.tscn").is_ok());

        let err = validate_res_path("--path", "main.tscn").unwrap_err();
        assert_eq!(err.error, "invalid_res_path");
    }

    #[test]
    fn reads_json_files() {
        let ctx = ctx();
        let path = std::env::temp_dir().join(format!(
            "gdx_context_json_{}_{}.json",
            std::process::id(),
            uuid::Uuid::new_v4().simple()
        ));
        std::fs::write(&path, r#"{"ok":true}"#).unwrap();

        let value = read_json_file(&ctx, &path).unwrap();

        assert_eq!(value["ok"], true);
        let _ = std::fs::remove_file(path);
    }
}
