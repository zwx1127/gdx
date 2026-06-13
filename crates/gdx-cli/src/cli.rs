use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

use crate::commands::{
    asset, code, export, init, play, project_cmd, resource, scene, session, test_cmd,
};

#[derive(Debug, Parser)]
#[command(name = "gdx", version, about = "Godot automation CLI for gdx")]
pub struct Cli {
    #[arg(long, global = true, value_name = "PATH")]
    pub godot: Option<PathBuf>,

    #[arg(long, global = true, value_name = "DIR")]
    pub project: Option<PathBuf>,

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

#[derive(Debug, Args)]
pub struct ProjectCommand {
    #[command(subcommand)]
    pub command: ProjectSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum ProjectSubcommand {
    Create(init::CreateArgs),
    Install(project_cmd::InstallArgs),
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
