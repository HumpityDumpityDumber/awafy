use anyhow::Result;
use clap::{Parser, Subcommand};

mod auth;
mod daemon;
mod models;

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Login,
    Daemon,
}

fn main() -> Result<(), anyhow::Error> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Login {} => auth::login(),
        Commands::Daemon {} => daemon::main(),
    }
}
