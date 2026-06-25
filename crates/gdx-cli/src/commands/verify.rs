use std::path::PathBuf;

use clap::Args;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::commands::{code, session, test_cmd};
use crate::context::{read_json_file, validate_non_empty, AppContext};
use crate::daemon;
use crate::error::{GdxError, GdxResult};
use crate::project::{ensure_parent_dir, godot_path_string};

#[derive(Debug, Args)]
pub struct VerifyArgs {
    #[arg(long)]
    pub spec: PathBuf,
}

#[derive(Debug, Deserialize)]
struct RuntimeSpec {
    #[serde(default)]
    checks: ChecksSpec,
    #[serde(default)]
    script_check: Option<ScriptCheckSpec>,
    #[serde(default)]
    tests: Vec<TestSpec>,
    #[serde(default)]
    daemon: Option<DaemonSpec>,
    #[serde(default)]
    steps: Vec<StepSpec>,
}

#[derive(Debug, Default, Deserialize)]
struct ChecksSpec {
    #[serde(default)]
    script: Option<ScriptCheckSpec>,
}

#[derive(Debug, Clone, Deserialize)]
struct ScriptCheckSpec {
    #[serde(default = "default_root")]
    root: String,
    #[serde(default = "default_true")]
    strict: bool,
}

#[derive(Debug, Deserialize)]
struct TestSpec {
    path: String,
    #[serde(default = "default_test_method")]
    method: String,
}

#[derive(Debug, Deserialize)]
struct DaemonSpec {
    scene: Option<String>,
    port: Option<u16>,
    #[serde(default = "default_width")]
    width: u32,
    #[serde(default = "default_height")]
    height: u32,
    #[serde(default)]
    restart: bool,
    #[serde(default = "default_true")]
    stop: bool,
}

#[derive(Debug, Deserialize)]
struct StepSpec {
    #[serde(default)]
    call: Option<CallStep>,
    #[serde(default)]
    state: Option<StateStep>,
    #[serde(default)]
    capture: Option<CaptureStep>,
    #[serde(default)]
    input_click_node: Option<ClickNodeStep>,
    #[serde(default)]
    input_activate: Option<ActivateStep>,
    #[serde(default)]
    input_tap: Option<TapStep>,
    #[serde(default)]
    input_drag: Option<DragStep>,
    #[serde(default)]
    input_swipe: Option<DragStep>,
    #[serde(default)]
    input_pinch: Option<PinchStep>,
    #[serde(default)]
    input_touch_sequence: Option<TouchSequenceStep>,
}

#[derive(Debug, Deserialize)]
struct CallStep {
    target: String,
    method: String,
    #[serde(default)]
    args: Vec<Value>,
}

#[derive(Debug, Deserialize)]
struct StateStep {
    target: String,
    method: Option<String>,
    property: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CaptureStep {
    out: PathBuf,
    #[serde(default = "default_capture_frames")]
    frames: u32,
}

#[derive(Debug, Deserialize)]
struct ClickNodeStep {
    target: String,
    #[serde(default = "default_button")]
    button: i64,
    #[serde(default = "default_click_frames")]
    frames: u32,
}

#[derive(Debug, Deserialize)]
struct ActivateStep {
    target: String,
}

#[derive(Debug, Deserialize)]
struct TapStep {
    position: Vec<f64>,
    #[serde(default)]
    index: u32,
    #[serde(default = "default_click_frames")]
    frames: u32,
}

#[derive(Debug, Deserialize)]
struct DragStep {
    from: Vec<f64>,
    to: Vec<f64>,
    #[serde(default)]
    index: u32,
    #[serde(default = "default_touch_steps")]
    steps: u32,
    #[serde(default = "default_touch_frames")]
    frames: u32,
}

#[derive(Debug, Deserialize)]
struct PinchStep {
    center: Vec<f64>,
    start_distance: f64,
    end_distance: f64,
    #[serde(default)]
    angle: f64,
    #[serde(default = "default_pinch_steps")]
    steps: u32,
    #[serde(default = "default_touch_frames")]
    frames: u32,
}

#[derive(Debug, Deserialize)]
struct TouchSequenceStep {
    events: Vec<session::TouchEvent>,
}

pub fn run(ctx: &AppContext, args: &VerifyArgs) -> GdxResult<Value> {
    let project = ctx.project()?;
    let spec_path = ctx.abs_path(&args.spec);
    let spec_value = read_json_file(ctx, &args.spec)?;
    let spec: RuntimeSpec = serde_json::from_value(spec_value).map_err(|err| {
        GdxError::user(
            "invalid_verify_spec",
            format!("Verify spec has invalid shape: {err}"),
        )
    })?;

    let mut results = Vec::new();
    let mut daemon_started_by_verify = false;
    let mut daemon_stop = false;

    let execution = (|| -> GdxResult<()> {
        if let Some(script) = spec.script_check.or(spec.checks.script) {
            let value = if script.strict {
                code::run_check_all(ctx, &code::CheckAllArgs { root: script.root })?
            } else {
                code::run_load_check(ctx, &code::LoadCheckArgs { root: script.root })?
            };
            results.push(json!({ "kind": "script_check", "result": value }));
        }

        for (index, test) in spec.tests.iter().enumerate() {
            let value = test_cmd::run(
                ctx,
                &test_cmd::RunArgs {
                    path: test.path.clone(),
                    method: test.method.clone(),
                },
            )
            .map_err(|err| step_error("test", index, err))?;
            results.push(json!({ "kind": "test", "index": index, "result": value }));
        }

        if let Some(daemon_spec) = &spec.daemon {
            let value = session::run_start(
                ctx,
                &session::StartArgs {
                    scene: daemon_spec.scene.clone(),
                    port: daemon_spec.port,
                    width: daemon_spec.width,
                    height: daemon_spec.height,
                    restart: daemon_spec.restart,
                },
            )?;
            daemon_started_by_verify = value
                .get("already_running")
                .and_then(Value::as_bool)
                .map(|already| !already)
                .unwrap_or(false);
            daemon_stop = daemon_spec.stop;
            results.push(json!({ "kind": "daemon_start", "result": value }));
        }

        for (index, step) in spec.steps.iter().enumerate() {
            let value = run_step(ctx, &project.root, index, step)
                .map_err(|err| step_error("step", index, err))?;
            results.push(value);
        }

        Ok(())
    })();

    if daemon_started_by_verify && daemon_stop {
        match session::run_stop(ctx, &session::StopArgs { force: true }) {
            Ok(value) => results.push(json!({ "kind": "daemon_stop", "result": value })),
            Err(err) if execution.is_ok() => return Err(err),
            Err(_) => {}
        }
    }

    execution?;

    Ok(json!({
        "ok": true,
        "command": "verify",
        "project": godot_path_string(&project.root),
        "spec": godot_path_string(&spec_path),
        "results": results
    }))
}

fn run_step(
    ctx: &AppContext,
    project_root: &std::path::Path,
    index: usize,
    step: &StepSpec,
) -> GdxResult<Value> {
    let variants = [
        step.call.is_some(),
        step.state.is_some(),
        step.capture.is_some(),
        step.input_click_node.is_some(),
        step.input_activate.is_some(),
        step.input_tap.is_some(),
        step.input_drag.is_some(),
        step.input_swipe.is_some(),
        step.input_pinch.is_some(),
        step.input_touch_sequence.is_some(),
    ]
    .into_iter()
    .filter(|present| *present)
    .count();
    if variants != 1 {
        return Err(GdxError::user(
            "invalid_verify_step",
            "Each verify step must contain exactly one action",
        ));
    }

    if let Some(call) = &step.call {
        validate_non_empty("target", &call.target)?;
        validate_non_empty("method", &call.method)?;
        let result = daemon::rpc(
            project_root,
            "call_method",
            json!({
                "target": call.target,
                "method": call.method,
                "args": call.args
            }),
            30,
        )?;
        return Ok(json!({ "kind": "call", "index": index, "result": result }));
    }
    if let Some(state) = &step.state {
        validate_non_empty("target", &state.target)?;
        let result = daemon::rpc(
            project_root,
            "get_state",
            json!({
                "target": state.target,
                "method": state.method,
                "property": state.property
            }),
            10,
        )?;
        return Ok(json!({ "kind": "state", "index": index, "result": result }));
    }
    if let Some(capture) = &step.capture {
        let out = ctx.abs_path(&capture.out);
        ensure_parent_dir(&out)?;
        let result = daemon::rpc(
            project_root,
            "capture",
            json!({
                "out": godot_path_string(&out),
                "frames": capture.frames
            }),
            30,
        )?;
        return Ok(json!({
            "kind": "capture",
            "index": index,
            "capture": godot_path_string(&out),
            "result": result
        }));
    }
    if let Some(click) = &step.input_click_node {
        validate_non_empty("target", &click.target)?;
        let result = daemon::rpc(
            project_root,
            "click_node",
            json!({
                "target": click.target,
                "button": click.button,
                "frames": click.frames
            }),
            10 + u64::from(click.frames),
        )?;
        return Ok(json!({ "kind": "input_click_node", "index": index, "result": result }));
    }
    if let Some(activate) = &step.input_activate {
        validate_non_empty("target", &activate.target)?;
        let result = daemon::rpc(
            project_root,
            "activate_node",
            json!({
                "target": activate.target
            }),
            10,
        )?;
        return Ok(json!({ "kind": "input_activate", "index": index, "result": result }));
    }
    if let Some(tap) = &step.input_tap {
        let events = session::tap_events(tap.position.clone(), tap.index, tap.frames)?;
        let result = session::run_touch_sequence_rpc(project_root, events)?;
        return Ok(json!({ "kind": "input_tap", "index": index, "result": result }));
    }
    if let Some(drag) = &step.input_drag {
        let events = session::drag_events(
            drag.from.clone(),
            drag.to.clone(),
            drag.index,
            drag.steps,
            drag.frames,
        )?;
        let result = session::run_touch_sequence_rpc(project_root, events)?;
        return Ok(json!({ "kind": "input_drag", "index": index, "result": result }));
    }
    if let Some(swipe) = &step.input_swipe {
        let events = session::drag_events(
            swipe.from.clone(),
            swipe.to.clone(),
            swipe.index,
            swipe.steps,
            swipe.frames,
        )?;
        let result = session::run_touch_sequence_rpc(project_root, events)?;
        return Ok(json!({ "kind": "input_swipe", "index": index, "result": result }));
    }
    if let Some(pinch) = &step.input_pinch {
        let events = session::pinch_events(
            pinch.center.clone(),
            pinch.start_distance,
            pinch.end_distance,
            pinch.angle,
            pinch.steps,
            pinch.frames,
        )?;
        let result = session::run_touch_sequence_rpc(project_root, events)?;
        return Ok(json!({ "kind": "input_pinch", "index": index, "result": result }));
    }
    if let Some(sequence) = &step.input_touch_sequence {
        session::validate_touch_events(&sequence.events)?;
        let result = session::run_touch_sequence_rpc(project_root, sequence.events.clone())?;
        return Ok(json!({ "kind": "input_touch_sequence", "index": index, "result": result }));
    }

    unreachable!("verify step variant count was checked")
}

fn step_error(kind: &str, index: usize, mut err: GdxError) -> GdxError {
    let details = match err.details.take() {
        Some(mut existing) if existing.is_object() => {
            existing["verify_step_kind"] = json!(kind);
            existing["verify_step_index"] = json!(index);
            existing
        }
        Some(existing) => json!({
            "context": existing,
            "verify_step_kind": kind,
            "verify_step_index": index
        }),
        None => json!({
            "verify_step_kind": kind,
            "verify_step_index": index
        }),
    };
    err.with_details(details)
}

fn default_root() -> String {
    "res://".to_string()
}

fn default_test_method() -> String {
    "run_tests".to_string()
}

fn default_true() -> bool {
    true
}

fn default_width() -> u32 {
    1280
}

fn default_height() -> u32 {
    720
}

fn default_capture_frames() -> u32 {
    10
}

fn default_button() -> i64 {
    1
}

fn default_click_frames() -> u32 {
    2
}

fn default_touch_steps() -> u32 {
    8
}

fn default_touch_frames() -> u32 {
    1
}

fn default_pinch_steps() -> u32 {
    10
}
