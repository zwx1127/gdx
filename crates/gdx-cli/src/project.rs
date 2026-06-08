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

pub fn read_project_name(project: &Path) -> GdxResult<Option<String>> {
    read_project_setting(project, "application", "config/name")
}

pub fn read_main_scene(project: &Path) -> GdxResult<Option<String>> {
    read_project_setting(project, "application", "run/main_scene")
}

pub fn read_project_setting(project: &Path, section: &str, key: &str) -> GdxResult<Option<String>> {
    let path = project.join("project.godot");
    let text = fs::read_to_string(&path).map_err(|err| {
        GdxError::tool(
            "io_failed",
            format!("Cannot read {}: {err}", path.display()),
        )
    })?;
    Ok(read_setting_from_text(&text, section, key))
}

pub fn set_project_setting(project: &Path, section: &str, key: &str, value: &str) -> GdxResult<()> {
    let path = project.join("project.godot");
    let text = fs::read_to_string(&path).map_err(|err| {
        GdxError::tool(
            "io_failed",
            format!("Cannot read {}: {err}", path.display()),
        )
    })?;
    let updated = set_setting_in_text(&text, section, key, value);
    fs::write(&path, updated).map_err(|err| {
        GdxError::tool(
            "io_failed",
            format!("Cannot write {}: {err}", path.display()),
        )
    })
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

fn read_setting_from_text(text: &str, section: &str, key: &str) -> Option<String> {
    let mut in_section = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_section = &trimmed[1..trimmed.len() - 1] == section;
            continue;
        }
        if in_section {
            if let Some((candidate, value)) = trimmed.split_once('=') {
                if candidate.trim() == key {
                    return Some(unquote(value.trim()));
                }
            }
        }
    }
    None
}

fn set_setting_in_text(text: &str, section: &str, key: &str, value: &str) -> String {
    let replacement = format!("{key}=\"{}\"", escape_project_value(value));
    let mut lines: Vec<String> = text.lines().map(ToString::to_string).collect();
    let section_header = format!("[{section}]");
    let mut section_start = None;
    let mut section_end = lines.len();

    for (index, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            if trimmed == section_header {
                section_start = Some(index);
            } else if section_start.is_some() {
                section_end = index;
                break;
            }
        }
    }

    if let Some(start) = section_start {
        for line in lines.iter_mut().take(section_end).skip(start + 1) {
            if line
                .trim_start()
                .strip_prefix(key)
                .and_then(|rest| rest.trim_start().strip_prefix('='))
                .is_some()
            {
                *line = replacement;
                return finish_lines(lines);
            }
        }
        lines.insert(section_end, replacement);
        return finish_lines(lines);
    }

    if !lines.last().map(|line| line.is_empty()).unwrap_or(true) {
        lines.push(String::new());
    }
    lines.push(section_header);
    lines.push(replacement);
    finish_lines(lines)
}

fn finish_lines(lines: Vec<String>) -> String {
    let mut text = lines.join("\n");
    text.push('\n');
    text
}

fn unquote(value: &str) -> String {
    if value.len() >= 2 && value.starts_with('"') && value.ends_with('"') {
        value[1..value.len() - 1]
            .replace("\\\"", "\"")
            .replace("\\\\", "\\")
    } else {
        value.to_string()
    }
}

fn escape_project_value(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_project_setting() {
        let text = "config_version=5\n\n[application]\nconfig/name=\"Demo\"\nrun/main_scene=\"res://main.tscn\"\n";

        assert_eq!(
            read_setting_from_text(text, "application", "config/name"),
            Some("Demo".to_string())
        );
        assert_eq!(
            read_setting_from_text(text, "application", "run/main_scene"),
            Some("res://main.tscn".to_string())
        );
    }

    #[test]
    fn sets_project_setting_in_existing_section() {
        let text = "config_version=5\n\n[application]\nconfig/name=\"Demo\"\n";
        let updated = set_setting_in_text(text, "application", "run/main_scene", "res://main.tscn");

        assert!(updated.contains("config/name=\"Demo\""));
        assert!(updated.contains("run/main_scene=\"res://main.tscn\""));
    }
}
