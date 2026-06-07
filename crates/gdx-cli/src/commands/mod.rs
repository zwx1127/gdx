mod asset;
mod code;
mod env;
mod export;
mod init;
mod play;
mod scene;

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use serde_json::Value;

use crate::error::GdxResult;

#[derive(Debug, Parser)]
#[command(name = "gdx", version, about = "Godot automation CLI for gdx MVP-0")]
pub struct Cli {
    #[arg(long, global = true)]
    pub json: bool,

    #[arg(long, global = true, value_name = "PATH")]
    pub godot: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Env,
    Init(InitCommand),
    Scene(SceneCommand),
    Asset(AssetCommand),
    Code(CodeCommand),
    Play(PlayCommand),
    Export(ExportCommand),
}

#[derive(Debug, Args)]
pub struct InitCommand {
    #[command(subcommand)]
    pub command: InitSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum InitSubcommand {
    Basic(init::BasicArgs),
}

#[derive(Debug, Args)]
pub struct SceneCommand {
    #[command(subcommand)]
    pub command: SceneSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum SceneSubcommand {
    Build(scene::BuildArgs),
}

#[derive(Debug, Args)]
pub struct AssetCommand {
    #[command(subcommand)]
    pub command: AssetSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum AssetSubcommand {
    Import(asset::ImportArgs),
}

#[derive(Debug, Args)]
pub struct CodeCommand {
    #[command(subcommand)]
    pub command: CodeSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum CodeSubcommand {
    Check(code::CheckArgs),
}

#[derive(Debug, Args)]
pub struct PlayCommand {
    #[command(subcommand)]
    pub command: PlaySubcommand,
}

#[derive(Debug, Subcommand)]
pub enum PlaySubcommand {
    Run(play::RunArgs),
}

#[derive(Debug, Args)]
pub struct ExportCommand {
    #[command(subcommand)]
    pub command: ExportSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum ExportSubcommand {
    Build(export::BuildArgs),
}

pub fn run(cli: &Cli) -> GdxResult<Value> {
    match &cli.command {
        Commands::Env => env::run(cli),
        Commands::Init(command) => match &command.command {
            InitSubcommand::Basic(args) => init::run_basic(args),
        },
        Commands::Scene(command) => match &command.command {
            SceneSubcommand::Build(args) => scene::run_build(cli, args),
        },
        Commands::Asset(command) => match &command.command {
            AssetSubcommand::Import(args) => asset::run_import(cli, args),
        },
        Commands::Code(command) => match &command.command {
            CodeSubcommand::Check(args) => code::run_check(cli, args),
        },
        Commands::Play(command) => match &command.command {
            PlaySubcommand::Run(args) => play::run_play(cli, args),
        },
        Commands::Export(command) => match &command.command {
            ExportSubcommand::Build(args) => export::run_build(cli, args),
        },
    }
}
