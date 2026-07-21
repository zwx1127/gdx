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
mod run_store;
mod traversal;

use clap::Parser;
use cli::Cli;
use commands::run;
use progress::ProgressReporter;

fn main() {
    let cli = Cli::parse();
    let progress = ProgressReporter::new(cli.progress, cli.command_name());
    progress.command_started();
    let heartbeat = progress.start_heartbeat();
    match run(&cli, progress.clone()) {
        Ok(mut value) => {
            progress.enrich_success(&mut value);
            drop(heartbeat);
            progress.completed(true);
            output::emit_ok(value);
        }
        Err(mut error) => {
            progress.enrich_error(&mut error);
            drop(heartbeat);
            progress.completed(false);
            output::emit_err(error, progress.mode());
        }
    }
}
