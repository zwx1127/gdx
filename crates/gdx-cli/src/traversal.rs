use std::path::{Component, Path};

pub fn is_generated_path(project: &Path, path: &Path) -> bool {
    if path == project {
        return false;
    }
    let Ok(relative) = path.strip_prefix(project) else {
        return true;
    };
    relative.components().any(|component| {
        let Component::Normal(name) = component else {
            return false;
        };
        matches!(
            name.to_str(),
            Some(".git" | ".gdx" | ".godot" | ".agent-relay" | "export")
        )
    })
}

pub fn is_explicitly_excluded(
    project: &Path,
    path: &Path,
    excluded: &[std::path::PathBuf],
) -> bool {
    excluded.iter().any(|value| {
        let absolute = if value.is_absolute() {
            value.clone()
        } else {
            project.join(value)
        };
        path == absolute || path.starts_with(&absolute)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_paths_are_pruned_but_addons_are_not() {
        let project = Path::new("D:/Game");
        assert!(is_generated_path(project, Path::new("D:/Game/.gdx/runs/1")));
        assert!(!is_generated_path(
            project,
            Path::new("D:/Game/addons/gdx_tools/automation.gd")
        ));
    }
}
