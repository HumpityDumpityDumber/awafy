use anyhow::Result;
use clap::{Parser, Subcommand};

mod commands;

#[allow(dead_code)]
mod models;

mod auth;
mod https;

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

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Login {} => commands::login::login().await,
        Commands::Daemon {} => commands::daemon::main().await,
    }
}
