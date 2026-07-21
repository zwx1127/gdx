use clap::Args;

use crate::context::AppContext;
use crate::error::{GdxError, GdxResult};
use crate::run_store::{self, RetentionPolicy};

#[derive(Debug, Args)]
pub struct ListArgs {
    #[arg(long, default_value = "all")]
    pub status: String,

    #[arg(long, default_value_t = 50)]
    pub limit: usize,
}

#[derive(Debug, Args)]
pub struct PruneArgs {
    #[arg(long, default_value_t = 200)]
    pub keep_successes: usize,

    #[arg(long, default_value_t = 50)]
    pub keep_failures: usize,

    #[arg(long, default_value_t = 7)]
    pub success_max_age_days: u64,

    #[arg(long, default_value_t = 30)]
    pub failure_max_age_days: u64,

    #[arg(long)]
    pub apply: bool,
}

pub fn run_list(ctx: &AppContext, args: &ListArgs) -> GdxResult<serde_json::Value> {
    if !matches!(
        args.status.as_str(),
        "all" | "succeeded" | "failed" | "running" | "timed_out" | "cancelled" | "unknown"
    ) {
        return Err(GdxError::user(
            "invalid_run_status",
            "--status must be all, succeeded, failed, running, timed_out, cancelled, or unknown",
        ));
    }
    let project = ctx.project()?;
    run_store::list(&project.root, &args.status, args.limit)
}

pub fn run_prune(ctx: &AppContext, args: &PruneArgs) -> GdxResult<serde_json::Value> {
    let project = ctx.project()?;
    run_store::prune(
        &project.root,
        RetentionPolicy {
            keep_successes: args.keep_successes,
            keep_failures: args.keep_failures,
            success_max_age_days: args.success_max_age_days,
            failure_max_age_days: args.failure_max_age_days,
        },
        args.apply,
        true,
    )
}
