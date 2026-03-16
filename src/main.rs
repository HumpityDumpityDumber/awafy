use clap::{Parser, Subcommand};
use reqwest::Error;

mod auth;
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
    Auth,
    Daemon,
}

fn main() -> Result<(), Error> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Auth {} => auth::auth(),
        Commands::Daemon {} => todo!(),
    }
}
