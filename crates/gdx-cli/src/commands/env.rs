use std::fs;
use std::path::Path;
use std::process::Command;

use serde_json::json;

use crate::context::AppContext;
use crate::error::{GdxError, GdxResult};
use crate::godot::{
    binary_is_dotnet, find_dotnet_on_path, find_standard_on_path, project_has_csharp, GodotCommand,
};
use crate::project::godot_path_string;

pub fn run(ctx: &AppContext) -> GdxResult<serde_json::Value> {
    let binary = ctx.locate_godot()?;
    let version = godot_version(ctx, &binary)?;
    if !version.starts_with('4') {
        return Err(GdxError::validation(
            "unsupported_godot_version",
            format!("Expected Godot 4.x, got {version}"),
        ));
    }

    let project_root = ctx.project.as_deref();
    let has_csharp = project_root.map(project_has_csharp).unwrap_or(false);
    let project_kind = if project_root.is_none() {
        "unknown"
    } else if has_csharp {
        "dotnet"
    } else {
        "gdscript"
    };
    let dotnet_build = binary_is_dotnet(&binary, &version);
    let sdk = dotnet_sdks();
    let has_dotnet_8 = sdk.versions.iter().any(|version| {
        version.split('.').next() == Some("8")
            || version
                .split('.')
                .next()
                .and_then(|part| part.parse::<u32>().ok())
                .map(|major| major > 8)
                .unwrap_or(false)
    });

    if has_csharp && !dotnet_build {
        return Err(GdxError::validation(
            "godot_dotnet_required",
            "The project contains C# files but the selected Godot build has no .NET support",
        )
        .with_stage("environment")
        .with_suggestion(
            "Configure --godot-dotnet or GDX_GODOT_DOTNET with a .NET-enabled Godot build.",
        ));
    }
    if has_csharp && !has_dotnet_8 {
        return Err(GdxError::validation(
            "dotnet_sdk_missing",
            "The C# project requires a 64-bit .NET 8 or newer SDK",
        )
        .with_stage("environment")
        .with_details(json!({
            "installed_sdks": sdk.versions,
            "dotnet_path": sdk.path,
            "required": ">=8.0",
        }))
        .with_suggestion("Install a current 64-bit .NET SDK, then rerun gdx doctor."));
    }

    let mut warnings = Vec::new();
    if project_root.is_some() && !has_csharp && dotnet_build && !has_dotnet_8 {
        warnings.push(
            "GDScript-only project is using .NET-enabled Godot without an installed SDK; configure standard Godot to avoid unnecessary .NET initialization."
                .to_string(),
        );
    }

    let os = std::env::consts::OS;
    let display_detected = match os {
        "linux" => {
            std::env::var_os("DISPLAY").is_some() || std::env::var_os("WAYLAND_DISPLAY").is_some()
        }
        _ => true,
    };
    let suggestion = if os == "linux" && !display_detected {
        Some("Runtime capture may fail; run in a desktop session or wrap Godot with xvfb-run -a.")
    } else {
        None
    };
    let standard_candidate = ctx
        .godot_standard
        .clone()
        .or_else(find_standard_on_path)
        .map(|path| godot_path_string(&path));
    let dotnet_candidate = ctx
        .godot_dotnet
        .clone()
        .or_else(find_dotnet_on_path)
        .map(|path| godot_path_string(&path));

    Ok(json!({
        "ok": true,
        "command": "doctor",
        "project": {
            "path": project_root.map(godot_path_string),
            "kind": project_kind,
            "has_csharp": has_csharp,
        },
        "godot": {
            "path": godot_path_string(&binary),
            "version": version,
            "build": if dotnet_build { "dotnet" } else { "standard" },
        },
        "godot_candidates": {
            "standard": standard_candidate,
            "dotnet": dotnet_candidate,
            "generic": std::env::var("GDX_GODOT").ok(),
        },
        "dotnet": {
            "path": sdk.path,
            "installed_sdks": sdk.versions,
            "required": if has_csharp { json!(">=8.0") } else { serde_json::Value::Null },
            "satisfied": !has_csharp || has_dotnet_8,
        },
        "warnings": warnings,
        "platform": os,
        "runtime_capture": {
            "requires_display": true,
            "display_detected": display_detected,
            "suggestion": suggestion,
        },
        "env": {
            "GDX_GODOT": std::env::var("GDX_GODOT").ok(),
            "GDX_GODOT_STANDARD": std::env::var("GDX_GODOT_STANDARD").ok(),
            "GDX_GODOT_DOTNET": std::env::var("GDX_GODOT_DOTNET").ok(),
        }
    }))
}

fn godot_version(ctx: &AppContext, binary: &Path) -> GdxResult<String> {
    let Some(project) = ctx.project.as_deref() else {
        return crate::godot::run_version(binary, 10, &ctx.progress);
    };
    let result = ctx.run_godot(GodotCommand {
        binary: binary.to_path_buf(),
        project: project.to_path_buf(),
        args: vec!["--version".to_string()],
        envs: Vec::new(),
        timeout_secs: 10,
        deadline: None,
        operation: "doctor.version".to_string(),
        subject: Some("godot --version".to_string()),
    })?;
    if result.status_code != 0 {
        return Err(crate::godot::godot_failed(&result));
    }
    fs::read_to_string(&result.stdout_log)
        .map(|text| text.trim().to_string())
        .map_err(|err| {
            GdxError::tool(
                "io_failed",
                format!("Cannot read {}: {err}", result.stdout_log.display()),
            )
        })
}

struct DotnetSdks {
    path: Option<String>,
    versions: Vec<String>,
}

fn dotnet_sdks() -> DotnetSdks {
    let path = which::which("dotnet").ok();
    let versions = path
        .as_deref()
        .and_then(|binary| Command::new(binary).arg("--list-sdks").output().ok())
        .filter(|output| output.status.success())
        .map(|output| {
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .filter_map(|line| line.split_whitespace().next())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    DotnetSdks {
        path: path.as_deref().map(path_string),
        versions,
    }
}

fn path_string(path: &Path) -> String {
    godot_path_string(path)
}
