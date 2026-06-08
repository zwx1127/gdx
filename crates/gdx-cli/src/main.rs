mod commands;
mod constants;
mod daemon;
mod error;
mod godot;
mod output;
mod project;

use clap::Parser;
use commands::{run, Cli};

fn main() {
    let cli = Cli::parse();
    match run(&cli) {
        Ok(value) => output::emit_ok(cli.json, value),
        Err(error) => output::emit_err(cli.json, error),
    }
}
