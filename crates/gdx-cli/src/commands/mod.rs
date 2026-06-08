mod asset;
mod code;
mod env;
mod export;
mod init;
mod play;
mod project_cmd;
mod scene;
mod session;

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use serde_json::Value;

use crate::error::GdxResult;

#[derive(Debug, Parser)]
#[command(name = "gdx", version, about = "Godot automation CLI for gdx")]
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
    Init(init::InitArgs),
    Project(ProjectCommand),
    Scene(SceneCommand),
    Asset(AssetCommand),
    Code(CodeCommand),
    Capture(session::CaptureArgs),
    Serve(session::ServeArgs),
    Status(session::StatusArgs),
    Kill(session::KillArgs),
    Play(PlayCommand),
    Export(ExportCommand),
}

#[derive(Debug, Args)]
pub struct SceneCommand {
    #[command(subcommand)]
    pub command: SceneSubcommand,
}

#[derive(Debug, Args)]
pub struct ProjectCommand {
    #[command(subcommand)]
    pub command: ProjectSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum ProjectSubcommand {
    Setup(project_cmd::SetupArgs),
    Inspect(project_cmd::InspectArgs),
}

#[derive(Debug, Subcommand)]
pub enum SceneSubcommand {
    New(scene::NewArgs),
    Tree(scene::TreeArgs),
    AddNode(scene::AddNodeArgs),
    Set(scene::SetArgs),
    Save(scene::SaveArgs),
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
        Commands::Init(args) => init::run(args),
        Commands::Project(command) => match &command.command {
            ProjectSubcommand::Setup(args) => project_cmd::run_setup(args),
            ProjectSubcommand::Inspect(args) => project_cmd::run_inspect(args),
        },
        Commands::Scene(command) => match &command.command {
            SceneSubcommand::New(args) => scene::run_new(cli, args),
            SceneSubcommand::Tree(args) => scene::run_tree(args),
            SceneSubcommand::AddNode(args) => scene::run_add_node(args),
            SceneSubcommand::Set(args) => scene::run_set(args),
            SceneSubcommand::Save(args) => scene::run_save(args),
        },
        Commands::Asset(command) => match &command.command {
            AssetSubcommand::Import(args) => asset::run_import(cli, args),
        },
        Commands::Code(command) => match &command.command {
            CodeSubcommand::Check(args) => code::run_check(cli, args),
        },
        Commands::Capture(args) => session::run_capture(args),
        Commands::Serve(args) => session::run_serve(cli, args),
        Commands::Status(args) => session::run_status(args),
        Commands::Kill(args) => session::run_kill(args),
        Commands::Play(command) => match &command.command {
            PlaySubcommand::Run(args) => play::run_play(cli, args),
        },
        Commands::Export(command) => match &command.command {
            ExportSubcommand::Build(args) => export::run_build(cli, args),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn parses_top_level_init() {
        let cli =
            Cli::try_parse_from(["gdx", "init", "--path", "demo", "--name", "Demo", "--json"])
                .unwrap();

        assert!(matches!(cli.command, Commands::Init(_)));
    }

    #[test]
    fn rejects_removed_init_basic_subcommand() {
        let err = Cli::try_parse_from(["gdx", "init", "basic", "--path", "demo", "--name", "Demo"])
            .unwrap_err();

        assert_eq!(err.kind(), clap::error::ErrorKind::UnknownArgument);
    }

    #[test]
    fn parses_project_setup() {
        let cli = Cli::try_parse_from(["gdx", "project", "setup", "--project", "demo", "--json"])
            .unwrap();

        assert!(matches!(cli.command, Commands::Project(_)));
    }

    #[test]
    fn parses_scene_new() {
        let cli = Cli::try_parse_from([
            "gdx",
            "scene",
            "new",
            "--project",
            "demo",
            "--out",
            "res://scenes/main.tscn",
            "--root-type",
            "Node2D",
            "--name",
            "Main",
            "--set-main",
            "--json",
        ])
        .unwrap();

        assert!(matches!(cli.command, Commands::Scene(_)));
    }

    #[test]
    fn rejects_removed_scene_build() {
        let err = Cli::try_parse_from([
            "gdx",
            "scene",
            "build",
            "--project",
            "demo",
            "--spec",
            "scene.json",
            "--out",
            "res://scenes/main.tscn",
        ])
        .unwrap_err();

        assert_eq!(err.kind(), clap::error::ErrorKind::InvalidSubcommand);
    }

    #[test]
    fn rejects_removed_scene_set_value_json() {
        let err = Cli::try_parse_from([
            "gdx",
            "scene",
            "set",
            "--project",
            "demo",
            "--node",
            "/Title",
            "--property",
            "text",
            "--value-json",
            "\"Hello\"",
        ])
        .unwrap_err();

        assert_eq!(err.kind(), clap::error::ErrorKind::UnknownArgument);
    }
}
