#![forbid(unsafe_code)]

use clap::{Parser, Subcommand};

use crate::commands::{doctor::DoctorArgs, init::InitArgs, live::LiveArgs, symbols::SymbolsArgs};

mod commands;

#[derive(Debug, Parser)]
#[command(name = "hls")]
#[command(about = "Read-only Hyperliquid spot screener")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Init(InitArgs),
    Doctor(DoctorArgs),
    Symbols(SymbolsArgs),
    Live(LiveArgs),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    match Cli::parse().command {
        Command::Init(args) => commands::init::run(args).await,
        Command::Doctor(args) => commands::doctor::run(args).await,
        Command::Symbols(args) => commands::symbols::run(args).await,
        Command::Live(args) => commands::live::run(args).await,
    }
}
