use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde_json::Value;

use crate::cli::Cli;
use crate::diagnostics::{LogSummary, WarningMode};
use crate::error::{GdxError, GdxResult};
use crate::godot;
use crate::progress::ProgressReporter;
use crate::project::{assert_project, Project};
use crate::run_store;

#[derive(Debug, Clone)]
pub struct AppContext {
    pub godot: Option<PathBuf>,
    pub godot_standard: Option<PathBuf>,
    pub godot_dotnet: Option<PathBuf>,
    pub project: Option<PathBuf>,
    pub cwd: PathBuf,
    pub progress: ProgressReporter,
    pub warnings: WarningMode,
    state: Arc<Mutex<ExecutionState>>,
}

#[derive(Debug, Default)]
struct ExecutionState {
    projects_with_runs: BTreeSet<PathBuf>,
    log_summary: LogSummary,
}

impl AppContext {
    pub fn new(godot: Option<PathBuf>, project: Option<PathBuf>) -> GdxResult<Self> {
        let cwd = std::env::current_dir().map_err(|err| {
            GdxError::tool("io_failed", format!("Cannot read current directory: {err}"))
        })?;
        Ok(Self {
            godot,
            godot_standard: None,
            godot_dotnet: None,
            project,
            cwd,
            progress: ProgressReporter::off(),
            warnings: WarningMode::Summary,
            state: Arc::new(Mutex::new(ExecutionState::default())),
        })
    }

    pub fn from_cli(cli: &Cli, progress: ProgressReporter) -> GdxResult<Self> {
        crate::process::initialize_cancellation();
        let mut value = Self::new(cli.godot.clone(), cli.project.clone())?;
        value.godot_standard = cli.godot_standard.clone();
        value.godot_dotnet = cli.godot_dotnet.clone();
        value.progress = progress;
        value.warnings = cli.warnings;
        Ok(value)
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
        let project = self.project.as_deref();
        godot::locate_godot(godot::GodotSelection {
            explicit: self.godot.as_deref(),
            standard: self.godot_standard.as_deref(),
            dotnet: self.godot_dotnet.as_deref(),
            project,
        })
    }

    pub fn run_godot(&self, command: godot::GodotCommand) -> GdxResult<godot::GodotRunResult> {
        let project = command.project.clone();
        let result = godot::run(command, &self.progress, self.warnings);
        let mut state = self.state.lock().unwrap_or_else(|err| err.into_inner());
        state.projects_with_runs.insert(project);
        if let Ok(value) = &result {
            state.log_summary.merge(&value.log_summary);
        } else if let Err(error) = &result {
            let summary = error
                .context
                .details
                .as_ref()
                .and_then(|details| details.get("diagnostics"))
                .and_then(|diagnostics| diagnostics.get("log_summary"))
                .cloned()
                .and_then(|value| serde_json::from_value::<LogSummary>(value).ok());
            if let Some(summary) = summary {
                state.log_summary.merge(&summary);
            }
        }
        result
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
        let result = self.run_godot(godot::automation_command(
            binary,
            project,
            action,
            params,
            timeout_secs,
        ))?;
        if result.status_code != 0 {
            return Err(godot::godot_failed(&result));
        }
        Ok(result.last_json.unwrap_or(Value::Null))
    }

    pub fn finalize_runs(&self) -> Value {
        let projects = self
            .state
            .lock()
            .unwrap_or_else(|err| err.into_inner())
            .projects_with_runs
            .clone();
        if projects.is_empty() {
            return serde_json::json!({
                "pruned_runs": 0,
                "freed_bytes": 0,
                "warnings": [],
            });
        }
        let mut pruned = 0u64;
        let mut freed = 0u64;
        let mut warnings = Vec::new();
        for project in projects {
            let value = run_store::auto_prune(&project);
            pruned += value["pruned_runs"].as_u64().unwrap_or(0);
            freed += value["freed_bytes"].as_u64().unwrap_or(0);
            if let Some(warning) = value.get("warning").and_then(Value::as_str) {
                warnings.push(serde_json::json!({ "message": warning }));
            }
            if let Some(items) = value.get("warnings").and_then(Value::as_array) {
                warnings.extend(items.iter().cloned());
            }
        }
        serde_json::json!({
            "pruned_runs": pruned,
            "freed_bytes": freed,
            "warnings": warnings,
        })
    }

    pub fn log_summary(&self) -> Value {
        serde_json::to_value(
            self.state
                .lock()
                .unwrap_or_else(|err| err.into_inner())
                .log_summary
                .filtered(self.warnings),
        )
        .unwrap_or(Value::Null)
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
    serde_json::from_str(text.trim_start_matches('\u{feff}')).map_err(|err| {
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

    #[test]
    fn reads_utf8_bom_json_files() {
        let ctx = ctx();
        let path = std::env::temp_dir().join(format!(
            "gdx_context_bom_{}_{}.json",
            std::process::id(),
            uuid::Uuid::new_v4().simple()
        ));
        std::fs::write(&path, "\u{feff}{\"text\":\"中文\"}").unwrap();
        let value = read_json_file(&ctx, &path).unwrap();
        assert_eq!(value["text"], "中文");
        let _ = std::fs::remove_file(path);
    }
}
