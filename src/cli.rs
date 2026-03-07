use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "mntpack", version, about = "Mintiler Package Manager")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Sync {
        repo: String,
        #[arg(short = 'v', long = "version")]
        version: Option<String>,
        #[arg(short = 'g', long = "global")]
        global: bool,
    },
    Run {
        package: String,
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },
    List,
    Update,
    Doctor,
}
