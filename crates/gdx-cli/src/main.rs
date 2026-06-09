mod cli;
mod commands;
mod constants;
mod context;
mod daemon;
mod diagnostics;
mod error;
mod godot;
mod output;
mod project;

use clap::Parser;
use cli::Cli;
use commands::run;

fn main() {
    let cli = Cli::parse();
    match run(&cli) {
        Ok(value) => output::emit_ok(value),
        Err(error) => output::emit_err(error),
    }
}
