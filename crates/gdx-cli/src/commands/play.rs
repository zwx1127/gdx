use std::fs;
use std::path::{Path, PathBuf};

use clap::Args;
use serde_json::json;

use super::session;

use crate::constants::GDX_RUNTIME_CAPTURE_RUNNER_RES;
use crate::context::{validate_res_path, AppContext};
use crate::error::{GdxError, GdxResult};
use crate::godot::{self, GodotCommand};
use crate::project::{ensure_parent_dir, godot_path_string, read_main_scene};

#[derive(Debug, Args)]
pub struct CaptureArgs {
    #[arg(long)]
    pub scene: Option<String>,

    #[arg(long)]
    pub out: PathBuf,

    #[arg(long, default_value_t = 10)]
    pub frames: u32,

    #[arg(long, default_value_t = 1280)]
    pub width: u32,

    #[arg(long, default_value_t = 720)]
    pub height: u32,
}

#[derive(Debug, Args)]
pub struct RecordArgs {
    #[arg(long)]
    pub scene: Option<String>,

    #[arg(long)]
    pub out: PathBuf,

    #[arg(long, default_value_t = 3.0)]
    pub duration: f64,

    #[arg(long, default_value_t = 60)]
    pub fps: u32,

    #[arg(long, default_value_t = 1280)]
    pub width: u32,

    #[arg(long, default_value_t = 720)]
    pub height: u32,

    #[arg(long)]
    pub input_sequence: Option<PathBuf>,

    /// Retain unique raw Godot logs after the recording process exits.
    #[arg(long)]
    pub logs_dir: Option<PathBuf>,
}

pub fn run_capture(ctx: &AppContext, args: &CaptureArgs) -> GdxResult<serde_json::Value> {
    let project = ctx.project()?;
    let scene = resolve_scene(&project.root, args.scene.as_deref())?;
    validate_res_path("--scene", &scene)?;
    if args.width == 0 || args.height == 0 {
        return Err(GdxError::user(
            "invalid_resolution",
            "Width and height must be greater than zero",
        ));
    }
    let capture = ctx.abs_path(&args.out);
    ensure_parent_dir(&capture)?;
    let binary = ctx.locate_godot()?;
    let result = ctx.run_godot(GodotCommand {
        binary,
        args: vec![
            "--path".to_string(),
            godot_path_string(&project.root),
            "--resolution".to_string(),
            format!("{}x{}", args.width, args.height),
            "--single-window".to_string(),
            GDX_RUNTIME_CAPTURE_RUNNER_RES.to_string(),
        ],
        envs: vec![
            ("GDX_TARGET_SCENE".to_string(), scene.clone()),
            ("GDX_CAPTURE_OUT".to_string(), godot_path_string(&capture)),
            ("GDX_CAPTURE_FRAMES".to_string(), args.frames.to_string()),
        ],
        timeout_secs: 120,
        deadline: None,
        operation: "capture.run".to_string(),
        subject: Some(scene.clone()),
    })?;
    if result.status_code != 0 {
        return Err(godot::godot_failed(&result).with_suggestion(runtime_capture_suggestion()));
    }

    let size = fs::metadata(&capture)
        .map(|metadata| metadata.len())
        .unwrap_or(0);
    if size == 0 {
        return Err(GdxError::tool(
            "capture_not_created",
            format!(
                "Godot completed but capture is missing or empty: {}",
                capture.display()
            ),
        ));
    }

    Ok(json!({
        "ok": true,
        "command": "capture.run",
        "project": godot_path_string(&project.root),
        "scene": scene,
        "capture": godot_path_string(&capture),
        "frames": args.frames,
        "resolution": [args.width, args.height],
        "artifacts": {
            "capture": godot_path_string(&capture)
        }
    }))
}

pub fn run_record(ctx: &AppContext, args: &RecordArgs) -> GdxResult<serde_json::Value> {
    let project = ctx.project()?;
    let scene = resolve_scene(&project.root, args.scene.as_deref())?;
    validate_res_path("--scene", &scene)?;
    if args.width == 0 || args.height == 0 {
        return Err(GdxError::user(
            "invalid_resolution",
            "Width and height must be greater than zero",
        ));
    }
    let frames = recording_frame_count(args.duration, args.fps)?;
    let input_events = match &args.input_sequence {
        Some(spec) => Some(session::read_touch_sequence(ctx, spec)?),
        None => None,
    };
    let input_sequence = args.input_sequence.as_ref().map(|spec| ctx.abs_path(spec));
    let recording = ctx.abs_path(&args.out);
    validate_recording_out(&recording)?;
    ensure_parent_dir(&recording)?;
    let logs_dir = args.logs_dir.as_ref().map(|path| ctx.abs_path(path));
    if let Some(directory) = &logs_dir {
        fs::create_dir_all(directory).map_err(|err| {
            GdxError::tool("io_failed", format!("Cannot create logs directory: {err}"))
        })?;
    }
    let binary = ctx.locate_godot()?;
    let mut command_args = vec![
        "--path".to_string(),
        godot_path_string(&project.root),
        "--resolution".to_string(),
        format!("{}x{}", args.width, args.height),
        "--single-window".to_string(),
        "--write-movie".to_string(),
        godot_path_string(&recording),
        "--fixed-fps".to_string(),
        args.fps.to_string(),
        "--quit-after".to_string(),
        frames.to_string(),
    ];
    let mut envs = Vec::new();
    if let Some(input_sequence) = &input_sequence {
        command_args.push(GDX_RUNTIME_CAPTURE_RUNNER_RES.to_string());
        envs.push(("GDX_TARGET_SCENE".to_string(), scene.clone()));
        envs.push((
            "GDX_INPUT_SEQUENCE".to_string(),
            godot_path_string(input_sequence),
        ));
    } else {
        command_args.push(scene.clone());
    }
    let result = ctx.run_godot(GodotCommand {
        binary,
        args: command_args,
        envs,
        timeout_secs: recording_timeout_secs(args.duration),
        deadline: None,
        operation: "capture.record".to_string(),
        subject: Some(scene.clone()),
    })?;
    let retained_logs = match &logs_dir {
        Some(directory) => Some(retain_recording_logs(
            directory,
            &result.stdout_log,
            &result.stderr_log,
        )?),
        None => None,
    };
    if result.status_code != 0 {
        let mut error = godot::godot_failed(&result).with_suggestion(runtime_capture_suggestion());
        if let Some(logs) = &retained_logs {
            error.insert_detail("artifacts", logs.clone());
        }
        return Err(error);
    }

    let size = fs::metadata(&recording)
        .map(|metadata| metadata.len())
        .unwrap_or(0);
    if size == 0 {
        let mut error = GdxError::tool(
            "recording_not_created",
            format!(
                "Godot completed but recording is missing or empty: {}",
                recording.display()
            ),
        );
        if let Some(logs) = &retained_logs {
            error.insert_detail("artifacts", logs.clone());
        }
        return Err(error);
    }

    let mut artifacts = json!({"recording": godot_path_string(&recording)});
    if let Some(logs) = retained_logs {
        artifacts
            .as_object_mut()
            .unwrap()
            .extend(logs.as_object().unwrap().clone());
    }
    Ok(json!({
        "ok": true,
        "command": "capture.record",
        "elapsed_ms": result.duration_ms,
        "project": godot_path_string(&project.root),
        "scene": scene,
        "recording": godot_path_string(&recording),
        "duration_seconds": args.duration,
        "fps": args.fps,
        "frames": frames,
        "resolution": [args.width, args.height],
        "input_sequence": input_sequence.as_ref().map(|path| godot_path_string(path)),
        "input_events": input_events.as_ref().map(|events| events.len()),
        "artifacts": artifacts
    }))
}

// Unique, create-new names avoid overwriting evidence from repeated runs.
fn retain_recording_logs(
    directory: &Path,
    stdout: &Path,
    stderr: &Path,
) -> GdxResult<serde_json::Value> {
    fn copy_log(directory: &Path, source: &Path, prefix: &str) -> GdxResult<PathBuf> {
        let mut dest = tempfile::Builder::new()
            .prefix(prefix)
            .suffix(".log")
            .tempfile_in(directory)
            .map_err(|err| {
                GdxError::tool("io_failed", format!("Cannot create recording log: {err}"))
            })?;
        let mut input = fs::File::open(source).map_err(|err| {
            GdxError::tool("io_failed", format!("Cannot read recording log: {err}"))
        })?;
        std::io::copy(&mut input, &mut dest).map_err(|err| {
            GdxError::tool("io_failed", format!("Cannot copy recording log: {err}"))
        })?;
        let (_, path) = dest.keep().map_err(|err| {
            GdxError::tool("io_failed", format!("Cannot retain recording log: {err}"))
        })?;
        Ok(path)
    }
    let stdout = copy_log(directory, stdout, "record-stdout-")?;
    let stderr = copy_log(directory, stderr, "record-stderr-")?;
    Ok(json!({"stdout_log": godot_path_string(&stdout), "stderr_log": godot_path_string(&stderr)}))
}

fn resolve_scene(project: &Path, explicit: Option<&str>) -> GdxResult<String> {
    if let Some(scene) = explicit {
        return Ok(scene.to_string());
    }
    read_main_scene(project)?.ok_or_else(|| {
        GdxError::user(
            "main_scene_missing",
            "No scene was provided and project has no main scene",
        )
        .with_suggestion("Pass --scene res://... or create one with gdx scene create --set-main.")
    })
}

fn validate_recording_out(path: &Path) -> GdxResult<()> {
    let is_avi = path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.eq_ignore_ascii_case("avi"))
        .unwrap_or(false);
    if is_avi {
        Ok(())
    } else {
        Err(GdxError::user(
            "invalid_recording_extension",
            "--out must end with .avi for Godot Movie Writer recording in v1",
        ))
    }
}

fn recording_frame_count(duration_seconds: f64, fps: u32) -> GdxResult<u32> {
    if !duration_seconds.is_finite() || duration_seconds <= 0.0 {
        return Err(GdxError::user(
            "invalid_duration",
            "--duration must be a finite number greater than zero",
        ));
    }
    if fps == 0 {
        return Err(GdxError::user(
            "invalid_fps",
            "--fps must be greater than zero",
        ));
    }
    let frames = (duration_seconds * f64::from(fps)).ceil();
    if !frames.is_finite() || frames > f64::from(u32::MAX) {
        return Err(GdxError::user(
            "invalid_duration",
            "--duration multiplied by --fps is too large",
        ));
    }
    Ok(frames.max(1.0) as u32)
}

fn recording_timeout_secs(duration_seconds: f64) -> u64 {
    120u64.saturating_add((duration_seconds * 4.0).ceil() as u64)
}

fn runtime_capture_suggestion() -> &'static str {
    if std::env::consts::OS == "linux" {
        "Ensure DISPLAY or WAYLAND_DISPLAY is set. In headless environments, run Godot with xvfb-run -a."
    } else {
        "Ensure Godot can open a normal runtime window on this machine."
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retained_logs_survive_sources_and_never_overwrite() {
        let sources = tempfile::tempdir().unwrap();
        let output = tempfile::tempdir().unwrap();
        let stdout = sources.path().join("out");
        let stderr = sources.path().join("err");
        fs::write(&stdout, b"Compatibility - llvmpipe\n").unwrap();
        fs::write(&stderr, b"Vulkan unavailable\n\xff").unwrap();
        let first = retain_recording_logs(output.path(), &stdout, &stderr).unwrap();
        let second = retain_recording_logs(output.path(), &stdout, &stderr).unwrap();
        assert_ne!(first["stdout_log"], second["stdout_log"]);
        assert_ne!(first["stderr_log"], second["stderr_log"]);
        drop(sources);
        assert_eq!(
            fs::read(first["stdout_log"].as_str().unwrap()).unwrap(),
            b"Compatibility - llvmpipe\n"
        );
        assert_eq!(
            fs::read(first["stderr_log"].as_str().unwrap()).unwrap(),
            b"Vulkan unavailable\n\xff"
        );
    }

    #[test]
    fn retained_logs_report_unwritable_destination() {
        let source = tempfile::NamedTempFile::new().unwrap();
        assert_eq!(
            retain_recording_logs(source.path(), source.path(), source.path())
                .unwrap_err()
                .error,
            "io_failed"
        );
    }

    #[test]
    fn recording_frame_count_rounds_up() {
        assert_eq!(recording_frame_count(1.25, 30).unwrap(), 38);
        assert_eq!(recording_frame_count(0.001, 60).unwrap(), 1);
    }

    #[test]
    fn recording_frame_count_rejects_invalid_values() {
        assert_eq!(
            recording_frame_count(0.0, 60).unwrap_err().error,
            "invalid_duration"
        );
        assert_eq!(
            recording_frame_count(1.0, 0).unwrap_err().error,
            "invalid_fps"
        );
    }

    #[test]
    fn recording_output_must_be_avi() {
        validate_recording_out(Path::new("recording.avi")).unwrap();
        validate_recording_out(Path::new("recording.AVI")).unwrap();
        assert_eq!(
            validate_recording_out(Path::new("recording.mp4"))
                .unwrap_err()
                .error,
            "invalid_recording_extension"
        );
    }
}
