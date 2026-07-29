use std::io::{self, Read};
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::cli::Cli;
use crate::error::{GdxError, GdxResult};
use crate::godot;
use crate::progress::ProgressReporter;
use crate::project::{assert_project, godot_path_string, Project};

#[derive(Debug, Clone)]
pub struct AppContext {
    pub godot: Option<PathBuf>,
    pub godot_standard: Option<PathBuf>,
    pub godot_dotnet: Option<PathBuf>,
    pub project: Option<PathBuf>,
    pub cwd: PathBuf,
    pub progress: ProgressReporter,
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
        })
    }

    pub fn from_cli(cli: &Cli, progress: ProgressReporter) -> GdxResult<Self> {
        crate::process::initialize_cancellation();
        let mut value = Self::new(cli.godot.clone(), cli.project.clone())?;
        value.godot_standard = cli.godot_standard.clone();
        value.godot_dotnet = cli.godot_dotnet.clone();
        value.progress = progress;
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
        godot::run(command, &self.progress)
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
        let value = result.last_json.unwrap_or(Value::Null);
        if value.get("ok").and_then(Value::as_bool) == Some(true) {
            Ok(value.get("result").cloned().unwrap_or(Value::Null))
        } else {
            Ok(value)
        }
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
    let bytes = std::fs::read(&path).map_err(|err| {
        GdxError::tool(
            "io_failed",
            format!("Cannot read {}: {err}", path.display()),
        )
    })?;
    parse_json_bytes(&bytes, &godot_path_string(&path), "invalid_json")
}

pub fn read_json_stdin(error_code: &str) -> GdxResult<Value> {
    let mut bytes = Vec::new();
    io::stdin()
        .read_to_end(&mut bytes)
        .map_err(|err| GdxError::tool("io_failed", format!("Cannot read stdin: {err}")))?;
    parse_json_bytes(&bytes, "stdin", error_code)
}

pub fn parse_json_text(text: &str, origin: &str, error_code: &str) -> GdxResult<Value> {
    parse_json_utf8(text.trim_start_matches('\u{feff}'), origin, error_code)
}

pub fn parse_json_bytes(bytes: &[u8], origin: &str, error_code: &str) -> GdxResult<Value> {
    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    let text = std::str::from_utf8(bytes).map_err(|err| {
        GdxError::user(
            "invalid_utf8",
            format!("{origin} must be UTF-8 with an optional UTF-8 BOM"),
        )
        .with_details(serde_json::json!({
            "input_origin": origin,
            "byte_offset": err.valid_up_to(),
        }))
        .with_suggestion("Write JSON as UTF-8 without BOM, or with a UTF-8 BOM.")
    })?;
    parse_json_utf8(text, origin, error_code)
}

fn parse_json_utf8(text: &str, origin: &str, error_code: &str) -> GdxResult<Value> {
    serde_json::from_str(text).map_err(|err| {
        let byte_offset = byte_offset_for_line_column(text, err.line(), err.column());
        GdxError::user(
            error_code,
            format!("{origin} must contain valid JSON: {err}"),
        )
        .with_details(serde_json::json!({
            "input_origin": origin,
            "line": err.line(),
            "column": err.column(),
            "byte_offset": byte_offset,
        }))
    })
}

fn byte_offset_for_line_column(text: &str, line: usize, column: usize) -> usize {
    let line_start = text
        .split_inclusive('\n')
        .take(line.saturating_sub(1))
        .map(str::len)
        .sum::<usize>();
    line_start
        + text[line_start..]
            .char_indices()
            .nth(column.saturating_sub(1))
            .map(|(index, _)| index)
            .unwrap_or_else(|| text[line_start..].len())
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

    #[test]
    fn reports_json_origin_and_position() {
        let error = parse_json_text("[\"ok\",]", "--args-json", "invalid_args_json").unwrap_err();

        assert_eq!(error.error, "invalid_args_json");
        let details = error.context.details.unwrap();
        assert_eq!(details["input_origin"], "--args-json");
        assert!(details["line"].as_u64().unwrap() >= 1);
        assert!(details["column"].as_u64().unwrap() >= 1);
        assert!(details.get("preview").is_none());
    }

    #[test]
    fn rejects_non_utf8_json_with_dedicated_error() {
        let error =
            parse_json_bytes(&[b'{', 0xFF, b'}'], "fixture.json", "invalid_json").unwrap_err();

        assert_eq!(error.error, "invalid_utf8");
        assert_eq!(
            error.context.details.unwrap()["input_origin"],
            "fixture.json"
        );
    }
}
