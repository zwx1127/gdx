mod build_info;
mod cli;
mod commands;
mod constants;
mod context;
mod daemon;
mod diagnostics;
mod error;
mod godot;
mod output;
mod process;
mod progress;
mod project;
mod traversal;

use clap::{error::ErrorKind, Parser};
use cli::Cli;
use commands::run;
use error::GdxError;
use progress::ProgressReporter;
use serde_json::json;

fn main() {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => handle_cli_error(error),
    };
    let progress = ProgressReporter::new(cli.progress, cli.command_name());
    progress.command_started();
    let heartbeat = progress.start_heartbeat();
    match run(&cli, progress.clone()) {
        Ok(value) => {
            drop(heartbeat);
            progress.completed(true);
            output::emit_ok(&value);
        }
        Err(mut error) => {
            progress.enrich_error(&mut error);
            drop(heartbeat);
            let value = output::error_value(&error);
            progress.completed(false);
            output::emit_err_value(&value, error.exit_code);
        }
    }
}

fn handle_cli_error(error: clap::Error) -> ! {
    if matches!(
        error.kind(),
        ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
    ) {
        error.exit();
    }
    let kind = format!("{:?}", error.kind());
    let message = error.to_string().trim().to_string();
    output::emit_err(
        GdxError::new("invalid_cli_arguments", message, 2)
            .with_details(json!({ "clap_error_kind": kind })),
    )
}
