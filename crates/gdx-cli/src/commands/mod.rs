mod asset;
mod code;
mod env;
mod export;
mod init;
mod play;
mod project_cmd;
mod resource;
mod scene;
mod session;
mod test_cmd;

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
    Doctor,
    Project(ProjectCommand),
    Scene(SceneCommand),
    Asset(AssetCommand),
    Script(ScriptCommand),
    Daemon(DaemonCommand),
    Run(RunCommand),
    Resource(ResourceCommand),
    Test(TestCommand),
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
    Init(init::InitArgs),
    Install(project_cmd::InstallArgs),
    Inspect(project_cmd::InspectArgs),
    Setting(ProjectSettingCommand),
    Autoload(ProjectAutoloadCommand),
    Input(ProjectInputCommand),
}

#[derive(Debug, Args)]
pub struct ProjectSettingCommand {
    #[command(subcommand)]
    pub command: ProjectSettingSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum ProjectSettingSubcommand {
    Get(project_cmd::SettingGetArgs),
    Set(project_cmd::SettingSetArgs),
    List(project_cmd::SettingListArgs),
}

#[derive(Debug, Args)]
pub struct ProjectAutoloadCommand {
    #[command(subcommand)]
    pub command: ProjectAutoloadSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum ProjectAutoloadSubcommand {
    Add(project_cmd::AutoloadAddArgs),
    Remove(project_cmd::AutoloadRemoveArgs),
    List(project_cmd::AutoloadListArgs),
}

#[derive(Debug, Args)]
pub struct ProjectInputCommand {
    #[command(subcommand)]
    pub command: ProjectInputSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum ProjectInputSubcommand {
    Add(project_cmd::InputAddArgs),
    Remove(project_cmd::InputRemoveArgs),
    List(project_cmd::InputListArgs),
}

#[derive(Debug, Subcommand)]
pub enum SceneSubcommand {
    Create(scene::CreateArgs),
    Build(scene::BuildArgs),
    Tree(scene::TreeArgs),
    Node(SceneNodeCommand),
    Save(scene::SaveArgs),
}

#[derive(Debug, Args)]
pub struct SceneNodeCommand {
    #[command(subcommand)]
    pub command: SceneNodeSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum SceneNodeSubcommand {
    Add(scene::AddNodeArgs),
    SetProperty(scene::SetPropertyArgs),
}

#[derive(Debug, Args)]
pub struct AssetCommand {
    #[command(subcommand)]
    pub command: AssetSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum AssetSubcommand {
    Copy(asset::CopyArgs),
    Import(asset::ImportArgs),
    Inspect(asset::InspectArgs),
}

#[derive(Debug, Args)]
pub struct ScriptCommand {
    #[command(subcommand)]
    pub command: ScriptSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum ScriptSubcommand {
    Create(code::CreateArgs),
    Attach(code::AttachArgs),
    Check(code::CheckArgs),
    CheckAll(code::CheckAllArgs),
}

#[derive(Debug, Args)]
pub struct DaemonCommand {
    #[command(subcommand)]
    pub command: DaemonSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum DaemonSubcommand {
    Start(session::StartArgs),
    Status(session::StatusArgs),
    Stop(session::StopArgs),
    Capture(session::CaptureArgs),
    Input(session::InputArgs),
    Call(session::CallArgs),
    State(session::StateArgs),
}

#[derive(Debug, Args)]
pub struct RunCommand {
    #[command(subcommand)]
    pub command: RunSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum RunSubcommand {
    Capture(play::CaptureArgs),
}

#[derive(Debug, Args)]
pub struct ResourceCommand {
    #[command(subcommand)]
    pub command: ResourceSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum ResourceSubcommand {
    Create(resource::CreateArgs),
    Inspect(resource::InspectArgs),
}

#[derive(Debug, Args)]
pub struct TestCommand {
    #[command(subcommand)]
    pub command: TestSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum TestSubcommand {
    Run(test_cmd::RunArgs),
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
        Commands::Doctor => env::run(cli),
        Commands::Project(command) => match &command.command {
            ProjectSubcommand::Init(args) => init::run(args),
            ProjectSubcommand::Install(args) => project_cmd::run_install(args),
            ProjectSubcommand::Inspect(args) => project_cmd::run_inspect(args),
            ProjectSubcommand::Setting(command) => match &command.command {
                ProjectSettingSubcommand::Get(args) => project_cmd::run_setting_get(args),
                ProjectSettingSubcommand::Set(args) => project_cmd::run_setting_set(args),
                ProjectSettingSubcommand::List(args) => project_cmd::run_setting_list(args),
            },
            ProjectSubcommand::Autoload(command) => match &command.command {
                ProjectAutoloadSubcommand::Add(args) => project_cmd::run_autoload_add(args),
                ProjectAutoloadSubcommand::Remove(args) => project_cmd::run_autoload_remove(args),
                ProjectAutoloadSubcommand::List(args) => project_cmd::run_autoload_list(args),
            },
            ProjectSubcommand::Input(command) => match &command.command {
                ProjectInputSubcommand::Add(args) => project_cmd::run_input_add(cli, args),
                ProjectInputSubcommand::Remove(args) => project_cmd::run_input_remove(cli, args),
                ProjectInputSubcommand::List(args) => project_cmd::run_input_list(cli, args),
            },
        },
        Commands::Scene(command) => match &command.command {
            SceneSubcommand::Create(args) => scene::run_create(cli, args),
            SceneSubcommand::Build(args) => scene::run_build(cli, args),
            SceneSubcommand::Tree(args) => scene::run_tree(args),
            SceneSubcommand::Node(command) => match &command.command {
                SceneNodeSubcommand::Add(args) => scene::run_add_node(args),
                SceneNodeSubcommand::SetProperty(args) => scene::run_set_property(args),
            },
            SceneSubcommand::Save(args) => scene::run_save(args),
        },
        Commands::Asset(command) => match &command.command {
            AssetSubcommand::Copy(args) => asset::run_copy(args),
            AssetSubcommand::Import(args) => asset::run_import(cli, args),
            AssetSubcommand::Inspect(args) => asset::run_inspect(cli, args),
        },
        Commands::Script(command) => match &command.command {
            ScriptSubcommand::Create(args) => code::run_create(args),
            ScriptSubcommand::Attach(args) => code::run_attach(cli, args),
            ScriptSubcommand::Check(args) => code::run_check(cli, args),
            ScriptSubcommand::CheckAll(args) => code::run_check_all(cli, args),
        },
        Commands::Daemon(command) => match &command.command {
            DaemonSubcommand::Start(args) => session::run_start(cli, args),
            DaemonSubcommand::Status(args) => session::run_status(args),
            DaemonSubcommand::Stop(args) => session::run_stop(args),
            DaemonSubcommand::Capture(args) => session::run_capture(args),
            DaemonSubcommand::Input(args) => session::run_input(args),
            DaemonSubcommand::Call(args) => session::run_call(args),
            DaemonSubcommand::State(args) => session::run_state(args),
        },
        Commands::Run(command) => match &command.command {
            RunSubcommand::Capture(args) => play::run_capture(cli, args),
        },
        Commands::Resource(command) => match &command.command {
            ResourceSubcommand::Create(args) => resource::run_create(cli, args),
            ResourceSubcommand::Inspect(args) => resource::run_inspect(cli, args),
        },
        Commands::Test(command) => match &command.command {
            TestSubcommand::Run(args) => test_cmd::run(cli, args),
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
    fn parses_project_init() {
        let cli = Cli::try_parse_from([
            "gdx",
            "project",
            "init",
            "--project",
            "demo",
            "--name",
            "Demo",
            "--json",
        ])
        .unwrap();

        assert!(matches!(cli.command, Commands::Project(_)));
    }

    #[test]
    fn rejects_removed_top_level_init() {
        let err =
            Cli::try_parse_from(["gdx", "init", "--path", "demo", "--name", "Demo"]).unwrap_err();

        assert_eq!(err.kind(), clap::error::ErrorKind::InvalidSubcommand);
    }

    #[test]
    fn parses_project_install() {
        let cli = Cli::try_parse_from(["gdx", "project", "install", "--project", "demo", "--json"])
            .unwrap();

        assert!(matches!(cli.command, Commands::Project(_)));
    }

    #[test]
    fn parses_scene_create() {
        let cli = Cli::try_parse_from([
            "gdx",
            "scene",
            "create",
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
    fn parses_scene_node_set_property() {
        let cli = Cli::try_parse_from([
            "gdx",
            "scene",
            "node",
            "set-property",
            "--project",
            "demo",
            "--node",
            "/Title",
            "--property",
            "text",
            "--value",
            "Hello",
        ])
        .unwrap();

        assert!(matches!(cli.command, Commands::Scene(_)));
    }

    #[test]
    fn parses_daemon_start() {
        let cli =
            Cli::try_parse_from(["gdx", "daemon", "start", "--project", "demo", "--json"]).unwrap();

        assert!(matches!(cli.command, Commands::Daemon(_)));
    }

    #[test]
    fn parses_run_capture() {
        let cli = Cli::try_parse_from([
            "gdx",
            "run",
            "capture",
            "--project",
            "demo",
            "--out",
            "shot.png",
        ])
        .unwrap();

        assert!(matches!(cli.command, Commands::Run(_)));
    }

    #[test]
    fn rejects_removed_scene_build_out_arg() {
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

        assert_eq!(err.kind(), clap::error::ErrorKind::UnknownArgument);
    }

    #[test]
    fn parses_scene_build() {
        let cli = Cli::try_parse_from([
            "gdx",
            "scene",
            "build",
            "--project",
            "demo",
            "--spec",
            "s.json",
        ])
        .unwrap();

        assert!(matches!(cli.command, Commands::Scene(_)));
    }

    #[test]
    fn parses_project_setting_autoload_input() {
        for args in [
            vec![
                "gdx",
                "project",
                "setting",
                "set",
                "--project",
                "demo",
                "--section",
                "application",
                "--key",
                "run/main_scene",
                "--value",
                "res://main.tscn",
            ],
            vec![
                "gdx",
                "project",
                "autoload",
                "add",
                "--project",
                "demo",
                "--name",
                "Game",
                "--path",
                "res://scripts/game.gd",
            ],
            vec![
                "gdx",
                "project",
                "input",
                "add",
                "--project",
                "demo",
                "--action",
                "ui_accept",
                "--keycode",
                "32",
            ],
        ] {
            let cli = Cli::try_parse_from(args).unwrap();

            assert!(matches!(cli.command, Commands::Project(_)));
        }
    }

    #[test]
    fn parses_asset_script_resource_test_commands() {
        for args in [
            vec![
                "gdx",
                "asset",
                "copy",
                "--project",
                "demo",
                "--from",
                "a.png",
                "--to",
                "res://assets/a.png",
            ],
            vec![
                "gdx",
                "script",
                "create",
                "--project",
                "demo",
                "--path",
                "res://scripts/main.gd",
            ],
            vec![
                "gdx",
                "resource",
                "inspect",
                "--project",
                "demo",
                "--path",
                "res://x.tres",
            ],
            vec![
                "gdx",
                "test",
                "run",
                "--project",
                "demo",
                "--path",
                "res://tests/test.gd",
            ],
        ] {
            let cli = Cli::try_parse_from(args).unwrap();

            assert!(matches!(
                cli.command,
                Commands::Asset(_)
                    | Commands::Script(_)
                    | Commands::Resource(_)
                    | Commands::Test(_)
            ));
        }
    }

    #[test]
    fn rejects_removed_top_level_daemon_commands() {
        for command in ["serve", "status", "kill", "capture"] {
            let err = Cli::try_parse_from(["gdx", command, "--project", "demo"]).unwrap_err();

            assert_eq!(err.kind(), clap::error::ErrorKind::InvalidSubcommand);
        }
    }

    #[test]
    fn rejects_removed_scene_set_value_json() {
        let err = Cli::try_parse_from([
            "gdx",
            "scene",
            "node",
            "set-property",
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
