use std::cmp::Reverse;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::error::{GdxError, GdxResult};
use crate::project::{ensure_dir, godot_path_string};

const RETENTION_FILE: &str = "retention.json";
const METADATA_FILE: &str = "run.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPolicy {
    pub keep_successes: usize,
    pub keep_failures: usize,
    pub success_max_age_days: u64,
    pub failure_max_age_days: u64,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            keep_successes: 200,
            keep_failures: 50,
            success_max_age_days: 7,
            failure_max_age_days: 30,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct RetentionState {
    #[serde(default)]
    policy: RetentionPolicy,
    #[serde(default)]
    legacy_migrated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunMetadata {
    pub run_id: String,
    pub operation: String,
    pub subject: Option<String>,
    pub parent_pid: u32,
    pub child_pid: Option<u32>,
    pub started_at_ms: u128,
    pub finished_at_ms: Option<u128>,
    pub duration_ms: Option<u128>,
    pub status: String,
    pub exit_code: Option<i32>,
    pub stdout_log: String,
    pub stderr_log: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RunEntry {
    pub run_id: String,
    pub status: String,
    pub operation: Option<String>,
    pub subject: Option<String>,
    pub child_pid: Option<u32>,
    pub started_at_ms: u128,
    pub duration_ms: Option<u128>,
    pub bytes: u64,
    pub path: String,
    pub legacy: bool,
}

pub struct RunHandle {
    pub run_id: String,
    pub run_dir: PathBuf,
    pub stdout_log: PathBuf,
    pub stderr_log: PathBuf,
    metadata: RunMetadata,
}

impl RunHandle {
    pub fn begin(project: &Path, operation: &str, subject: Option<&str>) -> GdxResult<Self> {
        let run_id = format!("r_{}", Uuid::new_v4().simple());
        let run_dir = runs_root(project).join(&run_id);
        ensure_dir(&run_dir)?;
        let stdout_log = run_dir.join("godot.stdout.log");
        let stderr_log = run_dir.join("godot.stderr.log");
        let metadata = RunMetadata {
            run_id: run_id.clone(),
            operation: operation.to_string(),
            subject: subject.map(str::to_string),
            parent_pid: std::process::id(),
            child_pid: None,
            started_at_ms: unix_time_ms(),
            finished_at_ms: None,
            duration_ms: None,
            status: "running".to_string(),
            exit_code: None,
            stdout_log: godot_path_string(&stdout_log),
            stderr_log: godot_path_string(&stderr_log),
        };
        let handle = Self {
            run_id,
            run_dir,
            stdout_log,
            stderr_log,
            metadata,
        };
        handle.write_metadata()?;
        Ok(handle)
    }

    pub fn set_child_pid(&mut self, pid: u32) -> GdxResult<()> {
        self.metadata.child_pid = Some(pid);
        self.write_metadata()
    }

    pub fn finish(&mut self, status: &str, exit_code: Option<i32>, duration: Duration) {
        self.metadata.status = status.to_string();
        self.metadata.exit_code = exit_code;
        self.metadata.finished_at_ms = Some(unix_time_ms());
        self.metadata.duration_ms = Some(duration.as_millis());
        let _ = self.write_metadata();
    }

    fn write_metadata(&self) -> GdxResult<()> {
        let path = self.run_dir.join(METADATA_FILE);
        let text = serde_json::to_string_pretty(&self.metadata)
            .map_err(|err| GdxError::tool("json_failed", format!("Cannot serialize run: {err}")))?;
        fs::write(&path, text).map_err(|err| {
            GdxError::tool(
                "io_failed",
                format!("Cannot write {}: {err}", path.display()),
            )
        })
    }
}

pub fn runs_root(project: &Path) -> PathBuf {
    project.join(".gdx").join("runs")
}

pub fn list(project: &Path, status: &str, limit: usize) -> GdxResult<Value> {
    let mut entries = collect_entries(project)?;
    entries.sort_by_key(|entry| Reverse(entry.started_at_ms));
    if status != "all" {
        entries.retain(|entry| entry.status == status);
    }
    let total = entries.len();
    entries.truncate(limit);
    let state = read_state(project);
    Ok(json!({
        "ok": true,
        "command": "runs.list",
        "project": godot_path_string(project),
        "total": total,
        "returned": entries.len(),
        "entries": entries,
        "retention": state.policy,
        "legacy_migrated": state.legacy_migrated,
    }))
}

pub fn prune(
    project: &Path,
    policy: RetentionPolicy,
    apply: bool,
    include_legacy: bool,
) -> GdxResult<Value> {
    let mut entries = collect_entries(project)?;
    let state = read_state(project);
    let include_legacy = include_legacy || state.legacy_migrated;
    entries.retain(|entry| include_legacy || !entry.legacy);
    let candidates = prune_candidates(&entries, &policy);
    let candidate_count = candidates.len();
    let candidate_bytes = candidates.iter().map(|entry| entry.bytes).sum::<u64>();
    let preview: Vec<_> = candidates
        .iter()
        .take(100)
        .map(|entry| entry.run_id.clone())
        .collect();
    let mut deleted = 0usize;
    let mut freed = 0u64;
    let mut delete_failures = Vec::new();
    if apply {
        for entry in &candidates {
            let path = runs_root(project).join(&entry.run_id);
            match safe_remove_run_dir(project, &path) {
                Ok(()) => {
                    deleted += 1;
                    freed += entry.bytes;
                }
                Err(error) => delete_failures.push(json!({
                    "run_id": entry.run_id,
                    "message": error.to_string(),
                })),
            }
        }
        write_state(
            project,
            &RetentionState {
                policy: policy.clone(),
                legacy_migrated: include_legacy,
            },
        )?;
    }
    Ok(json!({
        "ok": true,
        "command": "runs.prune",
        "project": godot_path_string(project),
        "applied": apply,
        "candidate_count": candidate_count,
        "candidate_bytes": candidate_bytes,
        "candidate_run_ids": preview,
        "candidates_truncated": candidate_count > 100,
        "deleted": deleted,
        "freed_bytes": freed,
        "delete_failures": delete_failures,
        "policy": policy,
        "legacy_included": include_legacy,
    }))
}

pub fn auto_prune(project: &Path) -> Value {
    let state = read_state(project);
    match prune(project, state.policy, true, state.legacy_migrated) {
        Ok(value) => json!({
            "pruned_runs": value["deleted"],
            "freed_bytes": value["freed_bytes"],
            "legacy_migrated": value["legacy_included"],
            "warnings": value["delete_failures"],
        }),
        Err(error) => json!({
            "pruned_runs": 0,
            "freed_bytes": 0,
            "warning": error.to_string(),
        }),
    }
}

fn prune_candidates<'a>(entries: &'a [RunEntry], policy: &RetentionPolicy) -> Vec<&'a RunEntry> {
    let now = unix_time_ms();
    let mut successes: Vec<_> = entries
        .iter()
        .filter(|entry| matches!(entry.status.as_str(), "succeeded" | "unknown"))
        .collect();
    let mut failures: Vec<_> = entries
        .iter()
        .filter(|entry| matches!(entry.status.as_str(), "failed" | "timed_out" | "cancelled"))
        .collect();
    successes.sort_by_key(|entry| Reverse(entry.started_at_ms));
    failures.sort_by_key(|entry| Reverse(entry.started_at_ms));
    let mut candidates = Vec::new();
    for (index, entry) in successes.into_iter().enumerate() {
        if index >= policy.keep_successes
            || older_than(now, entry.started_at_ms, policy.success_max_age_days)
        {
            candidates.push(entry);
        }
    }
    for (index, entry) in failures.into_iter().enumerate() {
        if index >= policy.keep_failures
            || older_than(now, entry.started_at_ms, policy.failure_max_age_days)
        {
            candidates.push(entry);
        }
    }
    candidates
}

fn collect_entries(project: &Path) -> GdxResult<Vec<RunEntry>> {
    let root = runs_root(project);
    if !root.exists() {
        return Ok(Vec::new());
    }
    let mut entries = Vec::new();
    for item in fs::read_dir(&root).map_err(|err| {
        GdxError::tool(
            "io_failed",
            format!("Cannot read {}: {err}", root.display()),
        )
    })? {
        let Ok(item) = item else { continue };
        let path = item.path();
        if !path.is_dir()
            || !item
                .file_name()
                .to_str()
                .map(|name| name.starts_with("r_"))
                .unwrap_or(false)
        {
            continue;
        }
        let bytes = directory_bytes(&path);
        let metadata_path = path.join(METADATA_FILE);
        let parsed = fs::read_to_string(&metadata_path).ok().and_then(|text| {
            serde_json::from_str::<RunMetadata>(text.trim_start_matches('\u{feff}')).ok()
        });
        if let Some(metadata) = parsed {
            entries.push(RunEntry {
                run_id: metadata.run_id,
                status: metadata.status,
                operation: Some(metadata.operation),
                subject: metadata.subject,
                child_pid: metadata.child_pid,
                started_at_ms: metadata.started_at_ms,
                duration_ms: metadata.duration_ms,
                bytes,
                path: godot_path_string(&path),
                legacy: false,
            });
        } else {
            let started_at_ms = item
                .metadata()
                .ok()
                .and_then(|metadata| metadata.modified().ok())
                .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
                .map(|duration| duration.as_millis())
                .unwrap_or(0);
            let has_unreadable_metadata = metadata_path.exists();
            entries.push(RunEntry {
                run_id: item.file_name().to_string_lossy().into_owned(),
                status: if has_unreadable_metadata {
                    "running".to_string()
                } else {
                    "unknown".to_string()
                },
                operation: has_unreadable_metadata.then(|| "metadata_unreadable".to_string()),
                subject: None,
                child_pid: None,
                started_at_ms,
                duration_ms: None,
                bytes,
                path: godot_path_string(&path),
                legacy: !has_unreadable_metadata,
            });
        }
    }
    Ok(entries)
}

fn safe_remove_run_dir(project: &Path, path: &Path) -> GdxResult<()> {
    let root = runs_root(project);
    if path.parent() != Some(root.as_path())
        || !path
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.starts_with("r_"))
            .unwrap_or(false)
    {
        return Err(GdxError::tool(
            "unsafe_prune_target",
            format!("Refusing to remove {}", path.display()),
        ));
    }
    fs::remove_dir_all(path).map_err(|err| {
        GdxError::tool(
            "io_failed",
            format!("Cannot remove {}: {err}", path.display()),
        )
    })
}

fn read_state(project: &Path) -> RetentionState {
    fs::read_to_string(runs_root(project).join(RETENTION_FILE))
        .ok()
        .and_then(|text| serde_json::from_str(text.trim_start_matches('\u{feff}')).ok())
        .unwrap_or_default()
}

fn write_state(project: &Path, state: &RetentionState) -> GdxResult<()> {
    let root = runs_root(project);
    ensure_dir(&root)?;
    let path = root.join(RETENTION_FILE);
    let text = serde_json::to_string_pretty(state).map_err(|err| {
        GdxError::tool("json_failed", format!("Cannot serialize retention: {err}"))
    })?;
    fs::write(&path, text).map_err(|err| {
        GdxError::tool(
            "io_failed",
            format!("Cannot write {}: {err}", path.display()),
        )
    })
}

fn directory_bytes(path: &Path) -> u64 {
    walkdir::WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter_map(|entry| entry.metadata().ok())
        .filter(|metadata| metadata.is_file())
        .map(|metadata| metadata.len())
        .sum()
}

fn older_than(now_ms: u128, then_ms: u128, days: u64) -> bool {
    now_ms.saturating_sub(then_ms) > Duration::from_secs(days.saturating_mul(86_400)).as_millis()
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
    fn retention_keeps_failures_separate_from_successes() {
        let now = unix_time_ms();
        let mut entries = Vec::new();
        for index in 0..10 {
            entries.push(RunEntry {
                run_id: format!("s{index}"),
                status: "succeeded".into(),
                operation: None,
                subject: None,
                child_pid: None,
                started_at_ms: now - index,
                duration_ms: None,
                bytes: 1,
                path: String::new(),
                legacy: false,
            });
            entries.push(RunEntry {
                run_id: format!("f{index}"),
                status: "failed".into(),
                operation: None,
                subject: None,
                child_pid: None,
                started_at_ms: now - index,
                duration_ms: None,
                bytes: 1,
                path: String::new(),
                legacy: false,
            });
        }
        let policy = RetentionPolicy {
            keep_successes: 3,
            keep_failures: 2,
            success_max_age_days: 7,
            failure_max_age_days: 30,
        };
        let candidates = prune_candidates(&entries, &policy);
        assert_eq!(candidates.len(), 15);
        assert!(candidates.iter().all(|entry| entry.status != "running"));
    }

    #[test]
    fn retention_scales_and_never_prunes_active_runs() {
        let now = unix_time_ms();
        let mut entries = Vec::new();
        for index in 0..1_000 {
            entries.push(RunEntry {
                run_id: format!("run_{index}"),
                status: if index % 10 == 0 {
                    "failed".into()
                } else {
                    "succeeded".into()
                },
                operation: None,
                subject: None,
                child_pid: None,
                started_at_ms: now - index,
                duration_ms: None,
                bytes: 1,
                path: String::new(),
                legacy: false,
            });
        }
        entries.push(RunEntry {
            run_id: "active".into(),
            status: "running".into(),
            operation: None,
            subject: None,
            child_pid: Some(std::process::id()),
            started_at_ms: 0,
            duration_ms: None,
            bytes: 1,
            path: String::new(),
            legacy: false,
        });

        let candidates = prune_candidates(&entries, &RetentionPolicy::default());

        assert_eq!(candidates.len(), 750);
        assert!(!candidates.iter().any(|entry| entry.run_id == "active"));
    }

    #[test]
    fn applied_prune_removes_only_verified_run_children() {
        let project = std::env::temp_dir().join(format!(
            "gdx_run_prune_{}_{}",
            std::process::id(),
            uuid::Uuid::new_v4().simple()
        ));
        let mut run = RunHandle::begin(&project, "test", Some("subject")).unwrap();
        let run_dir = run.run_dir.clone();
        run.finish("succeeded", Some(0), Duration::from_millis(1));

        let result = prune(
            &project,
            RetentionPolicy {
                keep_successes: 0,
                ..RetentionPolicy::default()
            },
            true,
            false,
        )
        .unwrap();

        assert_eq!(result["deleted"], 1);
        assert!(!run_dir.exists());
        let _ = fs::remove_dir_all(project);
    }
}
