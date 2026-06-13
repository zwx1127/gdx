use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::constants::{
    GDX_DAEMON_SERVER_GD, GDX_DAEMON_SERVER_TSCN, GDX_RUNTIME_CAPTURE_RUNNER_GD,
    GDX_RUNTIME_CAPTURE_RUNNER_TSCN, GDX_TOOLS_AUTOMATION_GD, GDX_TOOLS_CREATE_SCENE_GD,
};
use crate::error::{GdxError, GdxResult};
use crate::project::ensure_parent_dir;

pub(crate) const CLI_VERSION: &str = env!("CARGO_PKG_VERSION");

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
    pub cli_version: &'static str,
    pub up_to_date: bool,
    pub changed: bool,
    pub check: bool,
    pub restart_daemon_required: bool,
    pub files: Vec<AddonFileReport>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct AddonInspectReport {
    pub cli_version: &'static str,
    pub installed: bool,
    pub up_to_date: bool,
    pub files: Vec<AddonFileReport>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct AddonFileReport {
    pub path: String,
    pub status_before: &'static str,
    pub action: &'static str,
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
            expected_hash: status.expected_hash,
            actual_hash: status.actual_hash,
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
        cli_version: CLI_VERSION,
        up_to_date,
        changed: wrote,
        check: options.check,
        restart_daemon_required: wrote,
        files,
    })
}

pub(crate) fn inspect_gdx_addons(project: &Path) -> GdxResult<AddonInspectReport> {
    let mut files = Vec::new();
    for file in ADDON_FILES {
        let status = inspect_file(project, file)?;
        files.push(AddonFileReport {
            path: file.path.to_string(),
            status_before: status.status_before.as_str(),
            action: "none",
            expected_hash: status.expected_hash,
            actual_hash: status.actual_hash,
        });
    }
    let installed = files
        .iter()
        .all(|file| file.status_before != AddonStatus::Missing.as_str());
    let up_to_date = files
        .iter()
        .all(|file| file.status_before == AddonStatus::Current.as_str());

    Ok(AddonInspectReport {
        cli_version: CLI_VERSION,
        installed,
        up_to_date,
        files,
    })
}

struct FileStatus {
    status_before: AddonStatus,
    expected_hash: String,
    actual_hash: Option<String>,
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
    let expected_hash = content_hash(file.contents.as_bytes());
    let path = project.join(PathBuf::from(file.path));
    if !path.exists() {
        return Ok(FileStatus {
            status_before: AddonStatus::Missing,
            expected_hash,
            actual_hash: None,
        });
    }

    let actual = fs::read(&path).map_err(|err| {
        GdxError::tool(
            "io_failed",
            format!("Cannot read {}: {err}", path.display()),
        )
    })?;
    let actual_hash = content_hash(&actual);
    let status_before = if actual == file.contents.as_bytes() {
        AddonStatus::Current
    } else {
        AddonStatus::Outdated
    };
    Ok(FileStatus {
        status_before,
        expected_hash,
        actual_hash: Some(actual_hash),
    })
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
