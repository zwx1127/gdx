use std::io::{self, IsTerminal, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use clap::ValueEnum;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::error::GdxError;

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, ValueEnum)]
#[value(rename_all = "lower")]
pub enum ProgressMode {
    #[default]
    Auto,
    Stderr,
    Ndjson,
    Off,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ResolvedProgressMode {
    Human,
    Ndjson,
    Off,
}

#[derive(Debug, Clone)]
pub struct ProgressReporter {
    inner: Arc<ProgressInner>,
}

pub struct ProgressHeartbeat {
    stopped: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

#[derive(Debug)]
struct ProgressInner {
    mode: ResolvedProgressMode,
    command: String,
    run_id: String,
    started: Instant,
}

impl ProgressReporter {
    pub fn new(mode: ProgressMode, command: impl Into<String>) -> Self {
        let mode = match mode {
            ProgressMode::Auto if io::stderr().is_terminal() => ResolvedProgressMode::Human,
            ProgressMode::Auto => ResolvedProgressMode::Ndjson,
            ProgressMode::Stderr => ResolvedProgressMode::Human,
            ProgressMode::Ndjson => ResolvedProgressMode::Ndjson,
            ProgressMode::Off => ResolvedProgressMode::Off,
        };
        Self {
            inner: Arc::new(ProgressInner {
                mode,
                command: command.into(),
                run_id: format!("c_{}", Uuid::new_v4().simple()),
                started: Instant::now(),
            }),
        }
    }

    pub fn off() -> Self {
        Self::new(ProgressMode::Off, "test")
    }

    pub fn mode(&self) -> ResolvedProgressMode {
        self.inner.mode
    }

    pub fn run_id(&self) -> &str {
        &self.inner.run_id
    }

    pub fn command(&self) -> &str {
        &self.inner.command
    }

    pub fn elapsed(&self) -> Duration {
        self.inner.started.elapsed()
    }

    pub fn command_started(&self) {
        self.event("command_started", json!({}));
    }

    pub fn start_heartbeat(&self) -> ProgressHeartbeat {
        let stopped = Arc::new(AtomicBool::new(false));
        if self.mode() == ResolvedProgressMode::Off {
            return ProgressHeartbeat {
                stopped,
                thread: None,
            };
        }
        let reporter = self.clone();
        let thread_stopped = Arc::clone(&stopped);
        let handle = thread::spawn(move || {
            while !thread_stopped.load(Ordering::SeqCst) {
                thread::park_timeout(Duration::from_secs(4));
                if !thread_stopped.load(Ordering::SeqCst) {
                    reporter.event("heartbeat", json!({}));
                }
            }
        });
        ProgressHeartbeat {
            stopped,
            thread: Some(handle),
        }
    }

    pub fn child_started(
        &self,
        child_run_id: &str,
        pid: u32,
        subject: Option<&str>,
        timeout_secs: u64,
    ) {
        self.event(
            "child_started",
            json!({
                "child_run_id": child_run_id,
                "child_pid": pid,
                "subject": subject,
                "timeout_secs": timeout_secs,
            }),
        );
    }

    pub fn item_finished(
        &self,
        completed: usize,
        total: usize,
        subject: &str,
        duration_ms: u128,
        ok: bool,
    ) {
        self.event(
            "item_finished",
            json!({
                "completed": completed,
                "total": total,
                "subject": subject,
                "duration_ms": duration_ms,
                "ok": ok,
            }),
        );
    }

    pub fn heartbeat(&self, child_run_id: &str, pid: u32, subject: Option<&str>) {
        self.event(
            "heartbeat",
            json!({
                "child_run_id": child_run_id,
                "child_pid": pid,
                "subject": subject,
            }),
        );
    }

    pub fn event(&self, event: &str, fields: Value) {
        if self.inner.mode == ResolvedProgressMode::Off {
            return;
        }
        let mut value = json!({
            "type": "progress",
            "event": event,
            "command": self.inner.command,
            "run_id": self.inner.run_id,
            "timestamp_ms": unix_time_ms(),
            "elapsed_ms": self.elapsed().as_millis(),
        });
        if let (Some(target), Some(source)) = (value.as_object_mut(), fields.as_object()) {
            for (key, field) in source {
                target.insert(key.clone(), field.clone());
            }
        }

        let mut stderr = io::stderr().lock();
        match self.inner.mode {
            ResolvedProgressMode::Human => {
                let subject = fields
                    .get("subject")
                    .and_then(Value::as_str)
                    .map(|value| format!(" {value}"))
                    .unwrap_or_default();
                let progress = match (
                    fields.get("completed").and_then(Value::as_u64),
                    fields.get("total").and_then(Value::as_u64),
                ) {
                    (Some(done), Some(total)) => format!(" {done}/{total}"),
                    _ => String::new(),
                };
                let _ = writeln!(
                    stderr,
                    "[gdx] {} {}{}{} {}ms",
                    self.inner.command,
                    event,
                    progress,
                    subject,
                    self.elapsed().as_millis()
                );
            }
            ResolvedProgressMode::Ndjson => {
                if let Ok(text) = serde_json::to_string(&value) {
                    let _ = writeln!(stderr, "{text}");
                }
            }
            ResolvedProgressMode::Off => {}
        }
        let _ = stderr.flush();
    }

    pub fn completed(&self, ok: bool) {
        self.event("completed", json!({ "ok": ok }));
    }

    pub fn enrich_error(&self, error: &mut GdxError) {
        error.context.command = Some(self.command().to_string());
        error.context.run_id = Some(self.run_id().to_string());
        error
            .context
            .elapsed_ms
            .get_or_insert(self.elapsed().as_millis());
    }
}

impl Drop for ProgressHeartbeat {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::SeqCst);
        if let Some(handle) = self.thread.take() {
            handle.thread().unpark();
            let _ = handle.join();
        }
    }
}

fn unix_time_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_modes_resolve_deterministically() {
        assert_eq!(
            ProgressReporter::new(ProgressMode::Ndjson, "test").mode(),
            ResolvedProgressMode::Ndjson
        );
        assert_eq!(ProgressReporter::off().mode(), ResolvedProgressMode::Off);
    }
}
