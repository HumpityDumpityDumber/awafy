use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

mod commands;

#[allow(dead_code)]
mod models;

mod auth;
mod https;
mod player;
mod socket;

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Login {
        #[arg(short, long, default_value_t = false)]
        abema: bool,
    },
    Daemon {
        #[arg(short, long, default_value_t = false)]
        abema: bool,
    },
    Name,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    match &cli.command {
        Commands::Login { abema } => commands::login::login(*abema).await,
        Commands::Daemon { abema } => commands::daemon::main(*abema).await,
        Commands::Name => commands::name::main().await,
    }
}
