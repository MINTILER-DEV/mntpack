mod cli;
mod commands;
mod config;
mod github;
mod installer;
mod package;
mod shim;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};
use config::RuntimeContext;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let runtime = RuntimeContext::load_or_init()?;

    match cli.command {
        Commands::Sync {
            repo,
            version,
            global,
        } => commands::sync::execute(&runtime, &repo, version.as_deref(), global).await?,
        Commands::Run { package, args } => commands::run::execute(&runtime, &package, &args)?,
        Commands::List => commands::list::execute(&runtime)?,
        Commands::Update => commands::update::execute(&runtime).await?,
        Commands::Doctor => commands::doctor::execute(&runtime)?,
        Commands::Config { action } => commands::config::execute(&runtime, action)?,
    }

    Ok(())
}
