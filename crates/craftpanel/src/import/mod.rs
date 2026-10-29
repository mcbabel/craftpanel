pub mod detect;
mod ledger;
mod mcsmanager;

use std::path::PathBuf;

use anyhow::Result;
use clap::{Args, Subcommand};

#[derive(Debug, Subcommand)]
pub enum ImportCommand {
    #[command(about = "Take over the servers of an MCSManager daemon on this machine")]
    Mcsmanager(McsmanagerArgs),
    #[command(hide = true)]
    Ledger,
}

#[derive(Debug, Args)]
pub struct McsmanagerArgs {
    #[arg(long, default_value = "/opt/mcsmanager/daemon", help = "The MCSManager daemon folder")]
    pub from: PathBuf,
    #[arg(long, help = "The panel user who will own the imported servers")]
    pub owner: String,
    #[arg(long = "only", value_name = "UUID_OR_NAME", help = "Import only this instance; repeatable")]
    pub only: Vec<String>,
    #[arg(long, help = "Copy the server folders instead of moving them")]
    pub copy: bool,
    #[arg(long, help = "Show what would be imported and change nothing")]
    pub dry_run: bool,
    #[arg(long, help = "Also take instances whose folder lies outside MCSManager's InstanceData")]
    pub allow_outside: bool,
}

pub async fn run(command: ImportCommand) -> Result<()> {
    match command {
        ImportCommand::Mcsmanager(args) => mcsmanager::run(args).await,
        ImportCommand::Ledger => ledger::serve().await,
    }
}
