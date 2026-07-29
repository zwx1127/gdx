use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

use crate::commands::{
    asset, code, export, init, play, project_cmd, resource, scene, session, test_cmd,
};
use crate::progress::ProgressMode;

#[derive(Debug, Parser)]
#[command(name = "gdx", version, about = "Godot automation CLI for gdx")]
pub struct Cli {
    #[arg(long, global = true, value_name = "PATH")]
    pub godot: Option<PathBuf>,

    #[arg(long, global = true, value_name = "DIR")]
    pub project: Option<PathBuf>,

    #[arg(long, global = true, value_name = "PATH", env = "GDX_GODOT_STANDARD")]
    pub godot_standard: Option<PathBuf>,

    #[arg(long, global = true, value_name = "PATH", env = "GDX_GODOT_DOTNET")]
    pub godot_dotnet: Option<PathBuf>,

    #[arg(long, global = true, value_enum, default_value_t = ProgressMode::Auto)]
    pub progress: ProgressMode,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Doctor,
    Project(ProjectCommand),
    Setting(SettingCommand),
    Autoload(AutoloadCommand),
    #[command(name = "input-map")]
    InputMap(InputMapCommand),
    Asset(AssetCommand),
    Script(ScriptCommand),
    Scene(SceneCommand),
    Node(NodeCommand),
    Daemon(DaemonCommand),
    Input(InputCommand),
    Call(CallCommand),
    State(StateCommand),
    Capture(CaptureCommand),
    Resource(ResourceCommand),
    Test(TestCommand),
    Export(ExportCommand),
    Verify(crate::commands::verify::VerifyArgs),
}

impl Cli {
    pub fn command_name(&self) -> &'static str {
        match &self.command {
            Commands::Doctor => "doctor",
            Commands::Project(command) => match command.command {
                ProjectSubcommand::Create(_) => "project.create",
                ProjectSubcommand::Inspect(_) => "project.inspect",
                ProjectSubcommand::Update(_) => "project.update",
            },
            Commands::Setting(command) => match command.command {
                SettingSubcommand::Get(_) => "setting.get",
                SettingSubcommand::Set(_) => "setting.set",
                SettingSubcommand::List(_) => "setting.list",
            },
            Commands::Autoload(command) => match command.command {
                AutoloadSubcommand::Add(_) => "autoload.add",
                AutoloadSubcommand::Remove(_) => "autoload.remove",
                AutoloadSubcommand::List(_) => "autoload.list",
            },
            Commands::InputMap(command) => match command.command {
                InputMapSubcommand::Add(_) => "input-map.add",
                InputMapSubcommand::Remove(_) => "input-map.remove",
                InputMapSubcommand::List(_) => "input-map.list",
            },
            Commands::Asset(command) => match command.command {
                AssetSubcommand::Import(_) => "asset.import",
                AssetSubcommand::Inspect(_) => "asset.inspect",
            },
            Commands::Script(command) => match command.command {
                ScriptSubcommand::Create(_) => "script.create",
                ScriptSubcommand::Attach(_) => "script.attach",
                ScriptSubcommand::Check(_) => "script.check",
                ScriptSubcommand::CheckAll(_) => "script.check-all",
                ScriptSubcommand::LoadCheck(_) => "script.load-check",
            },
            Commands::Scene(command) => match command.command {
                SceneSubcommand::Create(_) => "scene.create",
                SceneSubcommand::Build(_) => "scene.build",
                SceneSubcommand::Tree(_) => "scene.tree",
                SceneSubcommand::Save(_) => "scene.save",
            },
            Commands::Node(command) => match command.command {
                NodeSubcommand::Create(_) => "node.create",
                NodeSubcommand::Set(_) => "node.set",
            },
            Commands::Daemon(command) => match command.command {
                DaemonSubcommand::Start(_) => "daemon.start",
                DaemonSubcommand::Status(_) => "daemon.status",
                DaemonSubcommand::Stop(_) => "daemon.stop",
            },
            Commands::Input(command) => match command.command {
                InputSubcommand::Send(_) => "input.send",
                InputSubcommand::Click(_) => "input.click",
                InputSubcommand::ClickNode(_) => "input.click-node",
                InputSubcommand::Touch(_) => "input.touch",
                InputSubcommand::Tap(_) => "input.tap",
                InputSubcommand::Drag(_) => "input.drag",
                InputSubcommand::Pinch(_) => "input.pinch",
                InputSubcommand::Sequence(_) => "input.sequence",
                InputSubcommand::Activate(_) => "input.activate",
            },
            Commands::Call(_) => "call.invoke",
            Commands::State(_) => "state.get",
            Commands::Capture(command) => match command.command {
                CaptureSubcommand::Run(_) => "capture.run",
                CaptureSubcommand::Daemon(_) => "capture.daemon",
                CaptureSubcommand::Record(_) => "capture.record",
            },
            Commands::Resource(command) => match command.command {
                ResourceSubcommand::Create(_) => "resource.create",
                ResourceSubcommand::Inspect(_) => "resource.inspect",
            },
            Commands::Test(_) => "test.run",
            Commands::Export(_) => "export.build",
            Commands::Verify(_) => "verify",
        }
    }
}

#[derive(Debug, Args)]
pub struct ProjectCommand {
    #[command(subcommand)]
    pub command: ProjectSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum ProjectSubcommand {
    Create(init::CreateArgs),
    Inspect(project_cmd::InspectArgs),
    Update(project_cmd::UpdateArgs),
}

#[derive(Debug, Args)]
pub struct SettingCommand {
    #[command(subcommand)]
    pub command: SettingSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum SettingSubcommand {
    Get(project_cmd::SettingGetArgs),
    Set(project_cmd::SettingSetArgs),
    List(project_cmd::SettingListArgs),
}

#[derive(Debug, Args)]
pub struct AutoloadCommand {
    #[command(subcommand)]
    pub command: AutoloadSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum AutoloadSubcommand {
    Add(project_cmd::AutoloadAddArgs),
    Remove(project_cmd::AutoloadRemoveArgs),
    List(project_cmd::AutoloadListArgs),
}

#[derive(Debug, Args)]
pub struct InputMapCommand {
    #[command(subcommand)]
    pub command: InputMapSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum InputMapSubcommand {
    Add(project_cmd::InputAddArgs),
    Remove(project_cmd::InputRemoveArgs),
    List(project_cmd::InputListArgs),
}

#[derive(Debug, Args)]
pub struct AssetCommand {
    #[command(subcommand)]
    pub command: AssetSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum AssetSubcommand {
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
    LoadCheck(code::LoadCheckArgs),
}

#[derive(Debug, Args)]
pub struct SceneCommand {
    #[command(subcommand)]
    pub command: SceneSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum SceneSubcommand {
    Create(scene::CreateArgs),
    Build(scene::BuildArgs),
    Tree(scene::TreeArgs),
    Save(scene::SaveArgs),
}

#[derive(Debug, Args)]
pub struct NodeCommand {
    #[command(subcommand)]
    pub command: NodeSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum NodeSubcommand {
    Create(scene::AddNodeArgs),
    Set(scene::SetPropertyArgs),
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
}

#[derive(Debug, Args)]
pub struct InputCommand {
    #[command(subcommand)]
    pub command: InputSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum InputSubcommand {
    Send(session::InputArgs),
    Click(session::ClickArgs),
    ClickNode(session::ClickNodeArgs),
    Touch(session::TouchArgs),
    Tap(session::TapArgs),
    Drag(session::DragArgs),
    Pinch(session::PinchArgs),
    Sequence(session::SequenceArgs),
    Activate(session::ActivateArgs),
}

#[derive(Debug, Args)]
pub struct CallCommand {
    #[command(subcommand)]
    pub command: CallSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum CallSubcommand {
    Invoke(session::CallArgs),
}

#[derive(Debug, Args)]
pub struct StateCommand {
    #[command(subcommand)]
    pub command: StateSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum StateSubcommand {
    Get(session::StateArgs),
}

#[derive(Debug, Args)]
pub struct CaptureCommand {
    #[command(subcommand)]
    pub command: CaptureSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum CaptureSubcommand {
    Run(play::CaptureArgs),
    Daemon(session::CaptureArgs),
    Record(play::RecordArgs),
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
