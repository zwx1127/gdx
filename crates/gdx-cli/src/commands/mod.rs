pub(crate) mod addons;
pub(crate) mod asset;
pub(crate) mod code;
pub(crate) mod env;
pub(crate) mod export;
pub(crate) mod init;
pub(crate) mod play;
pub(crate) mod project_cmd;
pub(crate) mod resource;
pub(crate) mod scene;
pub(crate) mod session;
pub(crate) mod test_cmd;
pub(crate) mod verify;

use serde_json::Value;

use crate::cli::{
    AssetSubcommand, AutoloadSubcommand, CallSubcommand, CaptureSubcommand, Cli, Commands,
    DaemonSubcommand, ExportSubcommand, InputMapSubcommand, InputSubcommand, NodeSubcommand,
    ProjectSubcommand, ResourceSubcommand, SceneSubcommand, ScriptSubcommand, SettingSubcommand,
    StateSubcommand, TestSubcommand,
};
use crate::context::AppContext;
use crate::error::GdxResult;

pub fn run(cli: &Cli) -> GdxResult<Value> {
    let ctx = AppContext::new(cli.godot.clone(), cli.project.clone())?;
    match &cli.command {
        Commands::Doctor => env::run(&ctx),
        Commands::Project(command) => match &command.command {
            ProjectSubcommand::Create(args) => init::run_create(&ctx, args),
            ProjectSubcommand::Install(args) => project_cmd::run_install(&ctx, args),
            ProjectSubcommand::Inspect(args) => project_cmd::run_inspect(&ctx, args),
            ProjectSubcommand::Update(args) => project_cmd::run_update(&ctx, args),
        },
        Commands::Setting(command) => match &command.command {
            SettingSubcommand::Get(args) => project_cmd::run_setting_get(&ctx, args),
            SettingSubcommand::Set(args) => project_cmd::run_setting_set(&ctx, args),
            SettingSubcommand::List(args) => project_cmd::run_setting_list(&ctx, args),
        },
        Commands::Autoload(command) => match &command.command {
            AutoloadSubcommand::Add(args) => project_cmd::run_autoload_add(&ctx, args),
            AutoloadSubcommand::Remove(args) => project_cmd::run_autoload_remove(&ctx, args),
            AutoloadSubcommand::List(args) => project_cmd::run_autoload_list(&ctx, args),
        },
        Commands::InputMap(command) => match &command.command {
            InputMapSubcommand::Add(args) => project_cmd::run_input_add(&ctx, args),
            InputMapSubcommand::Remove(args) => project_cmd::run_input_remove(&ctx, args),
            InputMapSubcommand::List(args) => project_cmd::run_input_list(&ctx, args),
        },
        Commands::Asset(command) => match &command.command {
            AssetSubcommand::Copy(args) => asset::run_copy(&ctx, args),
            AssetSubcommand::Import(args) => asset::run_import(&ctx, args),
            AssetSubcommand::Inspect(args) => asset::run_inspect(&ctx, args),
        },
        Commands::Script(command) => match &command.command {
            ScriptSubcommand::Create(args) => code::run_create(&ctx, args),
            ScriptSubcommand::Attach(args) => code::run_attach(&ctx, args),
            ScriptSubcommand::Check(args) => code::run_check(&ctx, args),
            ScriptSubcommand::CheckAll(args) => code::run_check_all(&ctx, args),
            ScriptSubcommand::LoadCheck(args) => code::run_load_check(&ctx, args),
        },
        Commands::Scene(command) => match &command.command {
            SceneSubcommand::Create(args) => scene::run_create(&ctx, args),
            SceneSubcommand::Build(args) => scene::run_build(&ctx, args),
            SceneSubcommand::Tree(args) => scene::run_tree(&ctx, args),
            SceneSubcommand::Save(args) => scene::run_save(&ctx, args),
        },
        Commands::Node(command) => match &command.command {
            NodeSubcommand::Create(args) => scene::run_add_node(&ctx, args),
            NodeSubcommand::Set(args) => scene::run_set_property(&ctx, args),
        },
        Commands::Daemon(command) => match &command.command {
            DaemonSubcommand::Start(args) => session::run_start(&ctx, args),
            DaemonSubcommand::Status(args) => session::run_status(&ctx, args),
            DaemonSubcommand::Stop(args) => session::run_stop(&ctx, args),
        },
        Commands::Input(command) => match &command.command {
            InputSubcommand::Send(args) => session::run_input(&ctx, args),
            InputSubcommand::Click(args) => session::run_click(&ctx, args),
            InputSubcommand::ClickNode(args) => session::run_click_node(&ctx, args),
            InputSubcommand::Touch(args) => session::run_touch(&ctx, args),
            InputSubcommand::Tap(args) => session::run_tap(&ctx, args),
            InputSubcommand::Drag(args) => session::run_drag(&ctx, args),
            InputSubcommand::Swipe(args) => session::run_swipe(&ctx, args),
            InputSubcommand::Pinch(args) => session::run_pinch(&ctx, args),
            InputSubcommand::Sequence(args) => session::run_sequence(&ctx, args),
            InputSubcommand::Activate(args) => session::run_activate(&ctx, args),
        },
        Commands::Call(command) => match &command.command {
            CallSubcommand::Invoke(args) => session::run_call(&ctx, args),
        },
        Commands::State(command) => match &command.command {
            StateSubcommand::Get(args) => session::run_state(&ctx, args),
        },
        Commands::Capture(command) => match &command.command {
            CaptureSubcommand::Run(args) => play::run_capture(&ctx, args),
            CaptureSubcommand::Daemon(args) => session::run_capture(&ctx, args),
            CaptureSubcommand::Record(args) => play::run_record(&ctx, args),
        },
        Commands::Resource(command) => match &command.command {
            ResourceSubcommand::Create(args) => resource::run_create(&ctx, args),
            ResourceSubcommand::Inspect(args) => resource::run_inspect(&ctx, args),
        },
        Commands::Test(command) => match &command.command {
            TestSubcommand::Run(args) => test_cmd::run(&ctx, args),
        },
        Commands::Export(command) => match &command.command {
            ExportSubcommand::Build(args) => export::run_build(&ctx, args),
        },
        Commands::Verify(args) => verify::run(&ctx, args),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    fn parses(args: &[&str]) -> Cli {
        Cli::try_parse_from(args).unwrap()
    }

    #[test]
    fn parses_global_project_context() {
        let cli = parses(&[
            "gdx",
            "--project",
            "demo",
            "setting",
            "set",
            "--section",
            "application",
            "--key",
            "run/main_scene",
            "--value",
            "res://main.tscn",
        ]);

        assert_eq!(cli.project, Some("demo".into()));
        assert!(matches!(cli.command, Commands::Setting(_)));
    }

    #[test]
    fn parses_project_create() {
        let cli = parses(&[
            "gdx", "project", "create", "--path", "demo", "--name", "Demo",
        ]);

        assert!(matches!(cli.command, Commands::Project(_)));
    }

    #[test]
    fn parses_project_update() {
        for args in [
            vec!["gdx", "--project", "demo", "project", "update"],
            vec!["gdx", "--project", "demo", "project", "update", "--check"],
            vec!["gdx", "--project", "demo", "project", "update", "--force"],
        ] {
            let cli = parses(&args);

            assert!(matches!(cli.command, Commands::Project(_)));
        }
    }

    #[test]
    fn parses_v2_resource_commands() {
        for args in [
            vec![
                "gdx",
                "--project",
                "demo",
                "autoload",
                "add",
                "--name",
                "Game",
                "--path",
                "res://scripts/game.gd",
            ],
            vec![
                "gdx",
                "--project",
                "demo",
                "input-map",
                "add",
                "--action",
                "ui_accept",
                "--keycode",
                "32",
            ],
            vec![
                "gdx",
                "--project",
                "demo",
                "asset",
                "copy",
                "--from",
                "a.png",
                "--to",
                "res://assets/a.png",
            ],
            vec![
                "gdx",
                "--project",
                "demo",
                "script",
                "create",
                "--path",
                "res://scripts/main.gd",
            ],
            vec![
                "gdx",
                "--project",
                "demo",
                "script",
                "load-check",
                "--root",
                "res://",
            ],
            vec![
                "gdx",
                "--project",
                "demo",
                "resource",
                "inspect",
                "--path",
                "res://x.tres",
            ],
            vec![
                "gdx",
                "--project",
                "demo",
                "test",
                "run",
                "--path",
                "res://tests/test.gd",
            ],
            vec![
                "gdx",
                "--project",
                "demo",
                "verify",
                "--spec",
                "verify.json",
            ],
        ] {
            let cli = parses(&args);

            assert!(matches!(
                cli.command,
                Commands::Autoload(_)
                    | Commands::InputMap(_)
                    | Commands::Asset(_)
                    | Commands::Script(_)
                    | Commands::Resource(_)
                    | Commands::Test(_)
                    | Commands::Verify(_)
            ));
        }
    }

    #[test]
    fn parses_scene_node_and_capture_commands() {
        for args in [
            vec![
                "gdx",
                "--project",
                "demo",
                "scene",
                "tree",
                "--include-script",
                "--include-groups",
                "--include-methods",
                "--method-prefix",
                "gdx_",
            ],
            vec![
                "gdx",
                "--project",
                "demo",
                "scene",
                "create",
                "--out",
                "res://scenes/main.tscn",
                "--root-type",
                "Node2D",
                "--name",
                "Main",
                "--set-main",
            ],
            vec![
                "gdx",
                "--project",
                "demo",
                "node",
                "set",
                "--node",
                "/Title",
                "--property",
                "text",
                "--value",
                "Hello",
            ],
            vec![
                "gdx",
                "--project",
                "demo",
                "capture",
                "run",
                "--out",
                "shot.png",
            ],
            vec![
                "gdx",
                "--project",
                "demo",
                "capture",
                "daemon",
                "--out",
                "shot.png",
            ],
            vec![
                "gdx",
                "--project",
                "demo",
                "capture",
                "record",
                "--out",
                "recording.avi",
                "--duration",
                "1.5",
                "--fps",
                "30",
            ],
        ] {
            let cli = parses(&args);

            assert!(matches!(
                cli.command,
                Commands::Scene(_) | Commands::Node(_) | Commands::Capture(_)
            ));
        }
    }

    #[test]
    fn parses_daemon_runtime_commands() {
        for args in [
            vec!["gdx", "--project", "demo", "daemon", "start"],
            vec![
                "gdx",
                "--project",
                "demo",
                "input",
                "send",
                "--mouse-button",
                "1",
            ],
            vec![
                "gdx",
                "--project",
                "demo",
                "input",
                "click",
                "--position",
                "120",
                "240",
            ],
            vec![
                "gdx",
                "--project",
                "demo",
                "input",
                "click-node",
                "--target",
                "/ClickMe",
            ],
            vec![
                "gdx",
                "--project",
                "demo",
                "input",
                "tap",
                "--position",
                "120",
                "240",
            ],
            vec![
                "gdx",
                "--project",
                "demo",
                "input",
                "drag",
                "--from",
                "10",
                "20",
                "--to",
                "120",
                "240",
            ],
            vec![
                "gdx",
                "--project",
                "demo",
                "input",
                "swipe",
                "--from",
                "10",
                "20",
                "--to",
                "120",
                "240",
            ],
            vec![
                "gdx",
                "--project",
                "demo",
                "input",
                "pinch",
                "--center",
                "100",
                "100",
                "--start-distance",
                "60",
                "--end-distance",
                "20",
            ],
            vec![
                "gdx",
                "--project",
                "demo",
                "input",
                "touch",
                "--position",
                "120",
                "240",
                "--pressed",
                "true",
            ],
            vec![
                "gdx",
                "--project",
                "demo",
                "input",
                "sequence",
                "--spec",
                "touch.json",
            ],
            vec![
                "gdx",
                "--project",
                "demo",
                "input",
                "activate",
                "--target",
                "/ClickMe",
            ],
            vec![
                "gdx",
                "--project",
                "demo",
                "call",
                "invoke",
                "--target",
                "/",
                "--method",
                "start_game",
            ],
            vec![
                "gdx",
                "--project",
                "demo",
                "state",
                "get",
                "--target",
                "/",
                "--method",
                "gdx_state",
            ],
        ] {
            let cli = parses(&args);

            assert!(matches!(
                cli.command,
                Commands::Daemon(_) | Commands::Input(_) | Commands::Call(_) | Commands::State(_)
            ));
        }
    }

    #[test]
    fn rejects_removed_v1_shapes() {
        for args in [
            vec![
                "gdx",
                "project",
                "init",
                "--project",
                "demo",
                "--name",
                "Demo",
            ],
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
            vec!["gdx", "scene", "node", "add", "--project", "demo"],
            vec![
                "gdx",
                "daemon",
                "capture",
                "--project",
                "demo",
                "--out",
                "shot.png",
            ],
            vec![
                "gdx",
                "run",
                "capture",
                "--project",
                "demo",
                "--out",
                "shot.png",
            ],
        ] {
            let err = Cli::try_parse_from(args).unwrap_err();

            assert_eq!(err.kind(), clap::error::ErrorKind::InvalidSubcommand);
        }
    }
}
