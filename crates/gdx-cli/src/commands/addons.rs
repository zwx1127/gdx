use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::constants::{
    GDX_DAEMON_SERVER_GD, GDX_DAEMON_SERVER_TSCN, GDX_RUNTIME_CAPTURE_RUNNER_GD,
    GDX_RUNTIME_CAPTURE_RUNNER_TSCN, GDX_TOOLS_AUTOMATION_GD, GDX_TOOLS_CREATE_SCENE_GD,
};
use crate::error::{GdxError, GdxResult};
use crate::project::ensure_parent_dir;

pub(crate) const MANAGED_RUNTIME_VERSION: &str = "0.1.1";
pub(crate) const MANAGED_PROTOCOL_VERSION: u64 = 1;

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

#[derive(Debug, Clone, Copy)]
pub(crate) struct AddonUpdateOptions {
    pub check: bool,
    pub force: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct AddonUpdateReport {
    pub up_to_date: bool,
    pub changed: bool,
    pub check: bool,
    pub restart_daemon_required: bool,
    pub files: Vec<AddonFileReport>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct AddonInspectReport {
    pub installed: bool,
    pub up_to_date: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct AddonFileReport {
    pub path: String,
    pub status_before: &'static str,
    pub action: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DaemonRuntimeReport {
    pub status: &'static str,
    pub compatible: bool,
    pub expected_version: &'static str,
    pub actual_version: Option<String>,
    pub expected_protocol_version: u64,
    pub actual_protocol_version: Option<u64>,
    pub expected_hash: String,
    pub actual_hash: Option<String>,
}

pub(crate) fn install_gdx_addons(project: &Path) -> GdxResult<Vec<String>> {
    let report = update_gdx_addons(
        project,
        AddonUpdateOptions {
            check: false,
            force: true,
        },
    )?;
    Ok(report.files.into_iter().map(|file| file.path).collect())
}

pub(crate) fn update_gdx_addons(
    project: &Path,
    options: AddonUpdateOptions,
) -> GdxResult<AddonUpdateReport> {
    let mut files = Vec::new();
    let mut wrote = false;

    for file in ADDON_FILES {
        let status = inspect_file(project, file)?;
        let action = action_for(status.status_before, options);
        if should_write(status.status_before, options) {
            write_text(&project.join(Path::new(file.path)), file.contents)?;
            wrote = true;
        }
        files.push(AddonFileReport {
            path: file.path.to_string(),
            status_before: status.status_before.as_str(),
            action,
        });
    }

    let up_to_date_before = files
        .iter()
        .all(|file| file.status_before == AddonStatus::Current.as_str());
    let up_to_date = if options.check {
        up_to_date_before
    } else {
        true
    };

    Ok(AddonUpdateReport {
        up_to_date,
        changed: wrote,
        check: options.check,
        restart_daemon_required: wrote,
        files,
    })
}

pub(crate) fn inspect_gdx_addons(project: &Path) -> GdxResult<AddonInspectReport> {
    let mut installed = true;
    let mut up_to_date = true;
    for file in ADDON_FILES {
        let status = inspect_file(project, file)?;
        installed &= status.status_before != AddonStatus::Missing;
        up_to_date &= status.status_before == AddonStatus::Current;
    }

    Ok(AddonInspectReport {
        installed,
        up_to_date,
    })
}

pub(crate) fn inspect_daemon_runtime(project: &Path) -> GdxResult<DaemonRuntimeReport> {
    let daemon_files: Vec<_> = ADDON_FILES
        .iter()
        .filter(|file| file.path.starts_with("addons/gdx_daemon/"))
        .collect();
    let expected_hash = aggregate_hash(
        daemon_files
            .iter()
            .map(|file| (file.path, file.contents.as_bytes())),
    );
    let mut actual_parts = Vec::new();
    let mut missing = false;
    let mut current = true;
    for file in daemon_files {
        let path = project.join(file.path);
        match fs::read(&path) {
            Ok(bytes) => {
                current &= bytes == file.contents.as_bytes();
                actual_parts.push((file.path, bytes));
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                missing = true;
                current = false;
            }
            Err(err) => {
                return Err(GdxError::tool(
                    "io_failed",
                    format!("Cannot read {}: {err}", path.display()),
                ));
            }
        }
    }
    let actual_hash = (!missing).then(|| {
        aggregate_hash(
            actual_parts
                .iter()
                .map(|(path, bytes)| (*path, bytes.as_slice())),
        )
    });
    let server_path = project.join(GDX_DAEMON_SERVER_GD);
    let server = fs::read_to_string(&server_path).ok();
    let actual_version = server
        .as_deref()
        .and_then(|text| parse_string_constant(text, "RUNTIME_VERSION"));
    let actual_protocol_version = server
        .as_deref()
        .and_then(|text| parse_u64_constant(text, "PROTOCOL_VERSION"));
    let compatible = current
        && actual_version.as_deref() == Some(MANAGED_RUNTIME_VERSION)
        && actual_protocol_version == Some(MANAGED_PROTOCOL_VERSION);
    Ok(DaemonRuntimeReport {
        status: if missing {
            "missing"
        } else if compatible {
            "current"
        } else {
            "outdated"
        },
        compatible,
        expected_version: MANAGED_RUNTIME_VERSION,
        actual_version,
        expected_protocol_version: MANAGED_PROTOCOL_VERSION,
        actual_protocol_version,
        expected_hash,
        actual_hash,
    })
}

struct FileStatus {
    status_before: AddonStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AddonStatus {
    Missing,
    Current,
    Outdated,
}

impl AddonStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Missing => "missing",
            Self::Current => "current",
            Self::Outdated => "outdated",
        }
    }
}

fn inspect_file(project: &Path, file: &ResourceFile) -> GdxResult<FileStatus> {
    let path = project.join(PathBuf::from(file.path));
    if !path.exists() {
        return Ok(FileStatus {
            status_before: AddonStatus::Missing,
        });
    }

    let actual = fs::read(&path).map_err(|err| {
        GdxError::tool(
            "io_failed",
            format!("Cannot read {}: {err}", path.display()),
        )
    })?;
    let status_before = if actual == file.contents.as_bytes() {
        AddonStatus::Current
    } else {
        AddonStatus::Outdated
    };
    Ok(FileStatus { status_before })
}

fn action_for(status: AddonStatus, options: AddonUpdateOptions) -> &'static str {
    if options.check {
        if options.force {
            return "would_rewrite";
        }
        return match status {
            AddonStatus::Missing => "would_create",
            AddonStatus::Outdated => "would_update",
            AddonStatus::Current => "unchanged",
        };
    }
    if options.force {
        return "rewritten";
    }
    match status {
        AddonStatus::Missing => "created",
        AddonStatus::Outdated => "updated",
        AddonStatus::Current => "unchanged",
    }
}

fn should_write(status: AddonStatus, options: AddonUpdateOptions) -> bool {
    if options.check {
        return false;
    }
    options.force || status != AddonStatus::Current
}

fn write_text(path: &Path, contents: &str) -> GdxResult<()> {
    ensure_parent_dir(path)?;
    fs::write(path, contents).map_err(|err| {
        GdxError::tool(
            "io_failed",
            format!("Cannot write {}: {err}", path.display()),
        )
    })
}

fn content_hash(bytes: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("fnv1a64:{hash:016x}")
}

fn aggregate_hash<'a>(parts: impl IntoIterator<Item = (&'a str, &'a [u8])>) -> String {
    let mut bytes = Vec::new();
    for (path, contents) in parts {
        bytes.extend_from_slice(path.as_bytes());
        bytes.push(0);
        bytes.extend_from_slice(contents);
        bytes.push(0xFF);
    }
    content_hash(&bytes)
}

fn parse_string_constant(text: &str, name: &str) -> Option<String> {
    let prefix = format!("const {name} :=");
    text.lines()
        .find_map(|line| line.trim().strip_prefix(&prefix))
        .map(str::trim)
        .and_then(|value| value.strip_prefix('"'))
        .and_then(|value| value.strip_suffix('"'))
        .map(str::to_string)
}

fn parse_u64_constant(text: &str, name: &str) -> Option<u64> {
    let prefix = format!("const {name} :=");
    text.lines()
        .find_map(|line| line.trim().strip_prefix(&prefix))
        .map(str::trim)
        .and_then(|value| value.parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn managed_runtime_constants_match_embedded_daemon() {
        let server = include_str!("../../resources/addons/gdx_daemon/daemon_server.gd");
        assert_eq!(
            parse_string_constant(server, "RUNTIME_VERSION").as_deref(),
            Some(MANAGED_RUNTIME_VERSION)
        );
        assert_eq!(
            parse_u64_constant(server, "PROTOCOL_VERSION"),
            Some(MANAGED_PROTOCOL_VERSION)
        );
    }

    #[test]
    fn daemon_runtime_reports_missing_and_current() {
        let project = std::env::temp_dir().join(format!(
            "gdx_runtime_report_{}_{}",
            std::process::id(),
            uuid::Uuid::new_v4().simple()
        ));
        fs::create_dir_all(&project).unwrap();
        assert_eq!(inspect_daemon_runtime(&project).unwrap().status, "missing");

        install_gdx_addons(&project).unwrap();
        let report = inspect_daemon_runtime(&project).unwrap();
        assert_eq!(report.status, "current");
        assert!(report.compatible);
        let _ = fs::remove_dir_all(project);
    }
}
