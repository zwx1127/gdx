use std::path::{Path, PathBuf};

use clap::{ArgAction, ArgGroup, Args};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::context::{read_json_file, validate_non_empty, validate_res_path, AppContext};
use crate::daemon::{self, SpawnDaemon};
use crate::error::{GdxError, GdxResult};
use crate::project::{ensure_parent_dir, godot_path_string, read_main_scene};

#[derive(Debug, Args)]
pub struct StartArgs {
    #[arg(long)]
    pub scene: Option<String>,

    #[arg(long)]
    pub port: Option<u16>,

    #[arg(long, default_value_t = 1280)]
    pub width: u32,

    #[arg(long, default_value_t = 720)]
    pub height: u32,

    #[arg(long)]
    pub restart: bool,
}

#[derive(Debug, Args)]
pub struct StatusArgs {}

#[derive(Debug, Args)]
pub struct StopArgs {
    #[arg(long)]
    pub force: bool,
}

#[derive(Debug, Args)]
pub struct CaptureArgs {
    #[arg(long)]
    pub out: PathBuf,

    #[arg(long, default_value_t = 10)]
    pub frames: u32,
}

#[derive(Debug, Args)]
#[command(group(
    ArgGroup::new("daemon_input_event")
        .required(true)
        .args(["keycode", "mouse_button", "mouse_motion"])
))]
pub struct InputArgs {
    #[arg(long)]
    pub keycode: Option<i64>,

    #[arg(long)]
    pub mouse_button: Option<i64>,

    #[arg(long)]
    pub mouse_motion: bool,

    #[arg(long, num_args = 2, allow_hyphen_values = true)]
    pub position: Option<Vec<f64>>,

    #[arg(long, default_value_t = true)]
    pub pressed: bool,
}

#[derive(Debug, Args)]
pub struct ClickArgs {
    #[arg(long, num_args = 2, allow_hyphen_values = true, required = true)]
    pub position: Vec<f64>,

    #[arg(long, default_value_t = 1)]
    pub button: i64,

    #[arg(long, default_value_t = 2)]
    pub frames: u32,
}

#[derive(Debug, Args)]
pub struct ClickNodeArgs {
    #[arg(long)]
    pub target: String,

    #[arg(long, default_value_t = 1)]
    pub button: i64,

    #[arg(long, default_value_t = 2)]
    pub frames: u32,
}

#[derive(Debug, Args)]
pub struct TouchArgs {
    #[arg(long, default_value_t = 0)]
    pub index: u32,

    #[arg(long, num_args = 2, allow_hyphen_values = true, required = true)]
    pub position: Vec<f64>,

    #[arg(long, action = ArgAction::Set, required = true)]
    pub pressed: bool,
}

#[derive(Debug, Args)]
pub struct TapArgs {
    #[arg(long, num_args = 2, allow_hyphen_values = true, required = true)]
    pub position: Vec<f64>,

    #[arg(long, default_value_t = 0)]
    pub index: u32,

    #[arg(long, default_value_t = 2)]
    pub frames: u32,
}

#[derive(Debug, Args)]
pub struct DragArgs {
    #[arg(
        long = "from",
        num_args = 2,
        allow_hyphen_values = true,
        required = true
    )]
    pub from: Vec<f64>,

    #[arg(long = "to", num_args = 2, allow_hyphen_values = true, required = true)]
    pub to: Vec<f64>,

    #[arg(long, default_value_t = 0)]
    pub index: u32,

    #[arg(long, default_value_t = 8)]
    pub steps: u32,

    #[arg(long, default_value_t = 1)]
    pub frames: u32,
}

#[derive(Debug, Args)]
pub struct PinchArgs {
    #[arg(long, num_args = 2, allow_hyphen_values = true, required = true)]
    pub center: Vec<f64>,

    #[arg(long)]
    pub start_distance: f64,

    #[arg(long)]
    pub end_distance: f64,

    #[arg(long, default_value_t = 0.0)]
    pub angle: f64,

    #[arg(long, default_value_t = 10)]
    pub steps: u32,

    #[arg(long, default_value_t = 1)]
    pub frames: u32,
}

#[derive(Debug, Args)]
pub struct SequenceArgs {
    #[arg(long)]
    pub spec: PathBuf,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum TouchEvent {
    Touch {
        index: u32,
        position: Vec<f64>,
        pressed: bool,
    },
    Drag {
        index: u32,
        position: Vec<f64>,
        relative: Vec<f64>,
    },
    Wait {
        frames: u32,
    },
}

#[derive(Debug, Deserialize)]
struct TouchSequenceSpec {
    events: Vec<TouchEvent>,
}

#[derive(Debug, Args)]
pub struct ActivateArgs {
    #[arg(long)]
    pub target: String,
}

#[derive(Debug, Args)]
pub struct CallArgs {
    #[arg(long)]
    pub target: String,

    #[arg(long)]
    pub method: String,

    #[arg(long, default_value = "[]")]
    pub args_json: String,
}

#[derive(Debug, Args)]
pub struct StateArgs {
    #[arg(long)]
    pub target: String,

    #[arg(long)]
    pub method: Option<String>,

    #[arg(long)]
    pub property: Option<String>,
}

pub fn run_start(ctx: &AppContext, args: &StartArgs) -> GdxResult<serde_json::Value> {
    let project = ctx.project()?;
    let scene = resolve_scene(&project.root, args.scene.as_deref())?;
    validate_res_path("--scene", &scene)?;
    if args.width == 0 || args.height == 0 {
        return Err(GdxError::user(
            "invalid_resolution",
            "Width and height must be greater than zero",
        ));
    }
    if let Ok(existing) = daemon::read_session(&project.root) {
        if daemon::ping_session(&existing) {
            let capabilities = capabilities_for_status(&existing);
            if !args.restart {
                return Ok(json!({
                    "ok": true,
                    "command": "daemon.start",
                    "project": godot_path_string(&project.root),
                    "already_running": true,
                    "session": existing,
                    "capabilities": capabilities
                }));
            }
            let _ = daemon::rpc_session(&existing, "shutdown", json!({}), 3);
            let _ = daemon::kill_process(existing.pid, true);
            daemon::remove_session(&project.root);
        } else {
            daemon::remove_session(&project.root);
        }
    }

    let binary = ctx.locate_godot()?;
    let port = match args.port {
        Some(port) => port,
        None => daemon::find_free_port()?,
    };
    let token = Uuid::new_v4().simple().to_string();
    let session = daemon::spawn_daemon(SpawnDaemon {
        binary,
        project: project.root.clone(),
        scene,
        port,
        token,
        width: args.width,
        height: args.height,
    })?;
    daemon::write_session(&project.root, &session)?;
    let capabilities = capabilities_for_status(&session);

    Ok(json!({
        "ok": true,
        "command": "daemon.start",
        "project": godot_path_string(&project.root),
        "already_running": false,
        "session": session,
        "capabilities": capabilities
    }))
}

fn resolve_scene(project: &std::path::Path, explicit: Option<&str>) -> GdxResult<String> {
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

pub fn run_status(ctx: &AppContext, _args: &StatusArgs) -> GdxResult<serde_json::Value> {
    let project = ctx.project()?;
    match daemon::read_session(&project.root) {
        Ok(session) => {
            let running = daemon::ping_session(&session);
            let capabilities = if running {
                capabilities_for_status(&session)
            } else {
                serde_json::Value::Null
            };
            Ok(json!({
                "ok": true,
                "command": "daemon.status",
                "project": godot_path_string(&project.root),
                "running": running,
                "session": session,
                "capabilities": capabilities
            }))
        }
        Err(_) => Ok(json!({
            "ok": true,
            "command": "daemon.status",
            "project": godot_path_string(&project.root),
            "running": false,
            "session": null,
            "capabilities": null
        })),
    }
}

fn capabilities_for_status(session: &daemon::DaemonSession) -> serde_json::Value {
    match daemon::rpc_session(session, "capabilities", json!({}), 2) {
        Ok(mut value) if value.is_object() => {
            value["status"] = json!("known");
            value
        }
        Ok(value) => json!({
            "status": "known",
            "value": value
        }),
        Err(err) if err.error == "unknown_method" => json!({
            "status": "unknown",
            "reason": "unsupported_capabilities_rpc"
        }),
        Err(err) => json!({
            "status": "unknown",
            "reason": err.error,
            "message": err.message
        }),
    }
}

pub fn run_stop(ctx: &AppContext, args: &StopArgs) -> GdxResult<serde_json::Value> {
    let project = ctx.project()?;
    let session = match daemon::read_session(&project.root) {
        Ok(session) => session,
        Err(_) => {
            return Ok(json!({
                "ok": true,
                "command": "daemon.stop",
                "project": godot_path_string(&project.root),
                "killed": false,
                "reason": "not_running"
            }));
        }
    };
    let shutdown_ok = daemon::rpc_session(&session, "shutdown", json!({}), 3).is_ok();
    if !shutdown_ok || args.force {
        let _ = daemon::kill_process(session.pid, args.force);
    }
    daemon::remove_session(&project.root);
    Ok(json!({
        "ok": true,
        "command": "daemon.stop",
        "project": godot_path_string(&project.root),
        "killed": true,
        "shutdown_rpc": shutdown_ok
    }))
}

pub fn run_capture(ctx: &AppContext, args: &CaptureArgs) -> GdxResult<serde_json::Value> {
    let project = ctx.project()?;
    let out = ctx.abs_path(&args.out);
    ensure_parent_dir(&out)?;
    let result = daemon::rpc(
        &project.root,
        "capture",
        json!({
            "out": godot_path_string(&out),
            "frames": args.frames
        }),
        30,
    )?;
    Ok(json!({
        "ok": true,
        "command": "capture.daemon",
        "project": godot_path_string(&project.root),
        "capture": godot_path_string(&out),
        "result": result
    }))
}

pub fn run_input(ctx: &AppContext, args: &InputArgs) -> GdxResult<serde_json::Value> {
    let project = ctx.project()?;
    let position = args.position.clone().unwrap_or_else(|| vec![0.0, 0.0]);
    let params = if let Some(keycode) = args.keycode {
        json!({ "kind": "key", "keycode": keycode, "pressed": args.pressed })
    } else if let Some(button) = args.mouse_button {
        json!({
            "kind": "mouse_button",
            "button": button,
            "position": position,
            "pressed": args.pressed
        })
    } else {
        json!({ "kind": "mouse_motion", "position": position })
    };
    let result = daemon::rpc(&project.root, "input_event", params, 10)?;
    Ok(json!({
        "ok": true,
        "command": "input.send",
        "project": godot_path_string(&project.root),
        "result": result
    }))
}

pub fn run_click(ctx: &AppContext, args: &ClickArgs) -> GdxResult<serde_json::Value> {
    if args.button <= 0 {
        return Err(GdxError::user(
            "invalid_button",
            "--button must be greater than zero",
        ));
    }
    let project = ctx.project()?;
    let result = daemon::rpc(
        &project.root,
        "input_click",
        json!({
            "button": args.button,
            "position": args.position,
            "frames": args.frames
        }),
        10 + u64::from(args.frames),
    )?;
    Ok(json!({
        "ok": true,
        "command": "input.click",
        "project": godot_path_string(&project.root),
        "result": result
    }))
}

pub fn run_click_node(ctx: &AppContext, args: &ClickNodeArgs) -> GdxResult<serde_json::Value> {
    validate_non_empty("target", &args.target)?;
    if args.button <= 0 {
        return Err(GdxError::user(
            "invalid_button",
            "--button must be greater than zero",
        ));
    }
    let project = ctx.project()?;
    let result = daemon::rpc(
        &project.root,
        "click_node",
        json!({
            "target": args.target,
            "button": args.button,
            "frames": args.frames
        }),
        10 + u64::from(args.frames),
    )?;
    Ok(json!({
        "ok": true,
        "command": "input.click-node",
        "project": godot_path_string(&project.root),
        "result": result
    }))
}

pub fn run_touch(ctx: &AppContext, args: &TouchArgs) -> GdxResult<Value> {
    let events = vec![TouchEvent::Touch {
        index: args.index,
        position: point_vec(&args.position, "--position")?,
        pressed: args.pressed,
    }];
    run_touch_command(ctx, "input.touch", events)
}

pub fn run_tap(ctx: &AppContext, args: &TapArgs) -> GdxResult<Value> {
    let events = tap_events(args.position.clone(), args.index, args.frames)?;
    run_touch_command(ctx, "input.tap", events)
}

pub fn run_drag(ctx: &AppContext, args: &DragArgs) -> GdxResult<Value> {
    let events = drag_events(
        args.from.clone(),
        args.to.clone(),
        args.index,
        args.steps,
        args.frames,
    )?;
    run_touch_command(ctx, "input.drag", events)
}

pub fn run_swipe(ctx: &AppContext, args: &DragArgs) -> GdxResult<Value> {
    let events = drag_events(
        args.from.clone(),
        args.to.clone(),
        args.index,
        args.steps,
        args.frames,
    )?;
    run_touch_command(ctx, "input.swipe", events)
}

pub fn run_pinch(ctx: &AppContext, args: &PinchArgs) -> GdxResult<Value> {
    let events = pinch_events(
        args.center.clone(),
        args.start_distance,
        args.end_distance,
        args.angle,
        args.steps,
        args.frames,
    )?;
    run_touch_command(ctx, "input.pinch", events)
}

pub fn run_sequence(ctx: &AppContext, args: &SequenceArgs) -> GdxResult<Value> {
    let events = read_touch_sequence(ctx, &args.spec)?;
    run_touch_command(ctx, "input.sequence", events)
}

pub(crate) fn run_touch_sequence_rpc(
    project_root: &Path,
    events: Vec<TouchEvent>,
) -> GdxResult<Value> {
    validate_touch_events(&events)?;
    let timeout = touch_timeout(&events);
    daemon::rpc(
        project_root,
        "touch_sequence",
        json!({
            "events": events
        }),
        timeout,
    )
}

pub(crate) fn tap_events(
    position: Vec<f64>,
    index: u32,
    frames: u32,
) -> GdxResult<Vec<TouchEvent>> {
    let position = point_vec(&position, "--position")?;
    Ok(vec![
        TouchEvent::Touch {
            index,
            position: position.clone(),
            pressed: true,
        },
        TouchEvent::Wait { frames },
        TouchEvent::Touch {
            index,
            position,
            pressed: false,
        },
        TouchEvent::Wait { frames },
    ])
}

pub(crate) fn drag_events(
    from: Vec<f64>,
    to: Vec<f64>,
    index: u32,
    steps: u32,
    frames: u32,
) -> GdxResult<Vec<TouchEvent>> {
    let start = point_pair(&from, "--from")?;
    let end = point_pair(&to, "--to")?;
    let mut events = vec![
        TouchEvent::Touch {
            index,
            position: vec![start[0], start[1]],
            pressed: true,
        },
        TouchEvent::Wait { frames },
    ];
    let mut previous = start;
    if steps > 0 {
        for step in 1..=steps {
            let t = f64::from(step) / f64::from(steps);
            let next = [
                start[0] + (end[0] - start[0]) * t,
                start[1] + (end[1] - start[1]) * t,
            ];
            events.push(TouchEvent::Drag {
                index,
                position: vec![next[0], next[1]],
                relative: vec![next[0] - previous[0], next[1] - previous[1]],
            });
            events.push(TouchEvent::Wait { frames });
            previous = next;
        }
    }
    events.push(TouchEvent::Touch {
        index,
        position: vec![end[0], end[1]],
        pressed: false,
    });
    events.push(TouchEvent::Wait { frames });
    Ok(events)
}

pub(crate) fn pinch_events(
    center: Vec<f64>,
    start_distance: f64,
    end_distance: f64,
    angle: f64,
    steps: u32,
    frames: u32,
) -> GdxResult<Vec<TouchEvent>> {
    let center = point_pair(&center, "--center")?;
    validate_distance("--start-distance", start_distance)?;
    validate_distance("--end-distance", end_distance)?;
    if !angle.is_finite() {
        return Err(GdxError::user(
            "invalid_angle",
            "--angle must be a finite number",
        ));
    }

    let direction = [angle.cos(), angle.sin()];
    let start_a = offset_point(center, direction, start_distance / 2.0);
    let start_b = offset_point(center, direction, -start_distance / 2.0);
    let end_a = offset_point(center, direction, end_distance / 2.0);
    let end_b = offset_point(center, direction, -end_distance / 2.0);
    let mut current_a = start_a;
    let mut current_b = start_b;

    let mut events = vec![
        TouchEvent::Touch {
            index: 0,
            position: vec![start_a[0], start_a[1]],
            pressed: true,
        },
        TouchEvent::Touch {
            index: 1,
            position: vec![start_b[0], start_b[1]],
            pressed: true,
        },
        TouchEvent::Wait { frames },
    ];

    if steps > 0 {
        for step in 1..=steps {
            let t = f64::from(step) / f64::from(steps);
            let next_a = [
                start_a[0] + (end_a[0] - start_a[0]) * t,
                start_a[1] + (end_a[1] - start_a[1]) * t,
            ];
            let next_b = [
                start_b[0] + (end_b[0] - start_b[0]) * t,
                start_b[1] + (end_b[1] - start_b[1]) * t,
            ];
            events.push(TouchEvent::Drag {
                index: 0,
                position: vec![next_a[0], next_a[1]],
                relative: vec![next_a[0] - current_a[0], next_a[1] - current_a[1]],
            });
            events.push(TouchEvent::Drag {
                index: 1,
                position: vec![next_b[0], next_b[1]],
                relative: vec![next_b[0] - current_b[0], next_b[1] - current_b[1]],
            });
            events.push(TouchEvent::Wait { frames });
            current_a = next_a;
            current_b = next_b;
        }
    }

    events.push(TouchEvent::Touch {
        index: 0,
        position: vec![end_a[0], end_a[1]],
        pressed: false,
    });
    events.push(TouchEvent::Touch {
        index: 1,
        position: vec![end_b[0], end_b[1]],
        pressed: false,
    });
    events.push(TouchEvent::Wait { frames });
    Ok(events)
}

pub(crate) fn validate_touch_events(events: &[TouchEvent]) -> GdxResult<()> {
    if events.is_empty() {
        return Err(GdxError::user(
            "invalid_touch_sequence",
            "touch sequence must contain at least one event",
        ));
    }
    for event in events {
        match event {
            TouchEvent::Touch { position, .. } => {
                point_pair(position, "touch.position")?;
            }
            TouchEvent::Drag {
                position, relative, ..
            } => {
                point_pair(position, "drag.position")?;
                point_pair(relative, "drag.relative")?;
            }
            TouchEvent::Wait { .. } => {}
        }
    }
    Ok(())
}

fn read_touch_sequence(ctx: &AppContext, spec: &Path) -> GdxResult<Vec<TouchEvent>> {
    let value = read_json_file(ctx, spec)?;
    let spec: TouchSequenceSpec = serde_json::from_value(value).map_err(|err| {
        GdxError::user(
            "invalid_touch_sequence",
            format!("Touch sequence spec has invalid shape: {err}"),
        )
    })?;
    validate_touch_events(&spec.events)?;
    Ok(spec.events)
}

fn run_touch_command(ctx: &AppContext, command: &str, events: Vec<TouchEvent>) -> GdxResult<Value> {
    let project = ctx.project()?;
    let result = run_touch_sequence_rpc(&project.root, events)?;
    Ok(json!({
        "ok": true,
        "command": command,
        "project": godot_path_string(&project.root),
        "result": result
    }))
}

fn touch_timeout(events: &[TouchEvent]) -> u64 {
    10 + events
        .iter()
        .map(|event| match event {
            TouchEvent::Wait { frames } => u64::from(*frames),
            _ => 0,
        })
        .sum::<u64>()
}

fn point_vec(position: &[f64], label: &str) -> GdxResult<Vec<f64>> {
    let pair = point_pair(position, label)?;
    Ok(vec![pair[0], pair[1]])
}

fn point_pair(position: &[f64], label: &str) -> GdxResult<[f64; 2]> {
    if position.len() != 2 {
        return Err(GdxError::user(
            "invalid_position",
            format!("{label} must contain exactly two numbers"),
        ));
    }
    if !position[0].is_finite() || !position[1].is_finite() {
        return Err(GdxError::user(
            "invalid_position",
            format!("{label} must contain finite numbers"),
        ));
    }
    Ok([position[0], position[1]])
}

fn validate_distance(label: &str, value: f64) -> GdxResult<()> {
    if !value.is_finite() || value <= 0.0 {
        return Err(GdxError::user(
            "invalid_distance",
            format!("{label} must be greater than zero"),
        ));
    }
    Ok(())
}

fn offset_point(center: [f64; 2], direction: [f64; 2], distance: f64) -> [f64; 2] {
    [
        center[0] + direction[0] * distance,
        center[1] + direction[1] * distance,
    ]
}

pub fn run_activate(ctx: &AppContext, args: &ActivateArgs) -> GdxResult<serde_json::Value> {
    validate_non_empty("target", &args.target)?;
    let project = ctx.project()?;
    let result = daemon::rpc(
        &project.root,
        "activate_node",
        json!({
            "target": args.target
        }),
        10,
    )?;
    Ok(json!({
        "ok": true,
        "command": "input.activate",
        "project": godot_path_string(&project.root),
        "result": result
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tap_expands_to_press_wait_release_wait() {
        let events = tap_events(vec![120.0, 240.0], 2, 3).unwrap();

        assert_eq!(
            events,
            vec![
                TouchEvent::Touch {
                    index: 2,
                    position: vec![120.0, 240.0],
                    pressed: true,
                },
                TouchEvent::Wait { frames: 3 },
                TouchEvent::Touch {
                    index: 2,
                    position: vec![120.0, 240.0],
                    pressed: false,
                },
                TouchEvent::Wait { frames: 3 },
            ]
        );
    }

    #[test]
    fn drag_includes_relative_motion() {
        let events = drag_events(vec![0.0, 0.0], vec![10.0, 20.0], 0, 2, 1).unwrap();

        assert!(matches!(
            events[2],
            TouchEvent::Drag {
                position: ref pos,
                relative: ref rel,
                ..
            } if pos == &vec![5.0, 10.0] && rel == &vec![5.0, 10.0]
        ));
        assert!(matches!(
            events[4],
            TouchEvent::Drag {
                position: ref pos,
                relative: ref rel,
                ..
            } if pos == &vec![10.0, 20.0] && rel == &vec![5.0, 10.0]
        ));
    }

    #[test]
    fn pinch_uses_two_touch_indexes() {
        let events = pinch_events(vec![100.0, 100.0], 20.0, 40.0, 0.0, 1, 1).unwrap();

        assert_eq!(
            events[0],
            TouchEvent::Touch {
                index: 0,
                position: vec![110.0, 100.0],
                pressed: true,
            }
        );
        assert_eq!(
            events[1],
            TouchEvent::Touch {
                index: 1,
                position: vec![90.0, 100.0],
                pressed: true,
            }
        );
    }

    #[test]
    fn rejects_invalid_positions() {
        let err = tap_events(vec![1.0], 0, 1).unwrap_err();

        assert_eq!(err.error, "invalid_position");
    }
}

pub fn run_call(ctx: &AppContext, args: &CallArgs) -> GdxResult<serde_json::Value> {
    validate_non_empty("target", &args.target)?;
    validate_non_empty("method", &args.method)?;
    let call_args: serde_json::Value = serde_json::from_str(&args.args_json).map_err(|err| {
        GdxError::user(
            "invalid_args_json",
            format!("--args-json must be valid JSON array: {err}"),
        )
    })?;
    if !call_args.is_array() {
        return Err(GdxError::user(
            "invalid_args_json",
            "--args-json must be a JSON array",
        ));
    }
    let project = ctx.project()?;
    let result = daemon::rpc(
        &project.root,
        "call_method",
        json!({
            "target": args.target,
            "method": args.method,
            "args": call_args
        }),
        30,
    )?;
    Ok(json!({
        "ok": true,
        "command": "call.invoke",
        "project": godot_path_string(&project.root),
        "result": result
    }))
}

pub fn run_state(ctx: &AppContext, args: &StateArgs) -> GdxResult<serde_json::Value> {
    validate_non_empty("target", &args.target)?;
    let project = ctx.project()?;
    let result = daemon::rpc(
        &project.root,
        "get_state",
        json!({
            "target": args.target,
            "method": args.method,
            "property": args.property
        }),
        10,
    )?;
    Ok(json!({
        "ok": true,
        "command": "state.get",
        "project": godot_path_string(&project.root),
        "result": result
    }))
}
