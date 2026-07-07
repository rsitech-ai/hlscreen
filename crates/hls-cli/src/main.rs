#![forbid(unsafe_code)]

use clap::{Parser, Subcommand};

use crate::commands::{
    doctor::DoctorArgs, init::InitArgs, live::LiveArgs, record::RecordArgs, replay::ReplayArgs,
    screen::ScreenArgs, symbols::SymbolsArgs,
};

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
    Record(RecordArgs),
    Replay(ReplayArgs),
    Screen(ScreenArgs),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    match Cli::parse().command {
        Command::Init(args) => commands::init::run(args).await,
        Command::Doctor(args) => commands::doctor::run(args).await,
        Command::Symbols(args) => commands::symbols::run(args).await,
        Command::Live(args) => commands::live::run(args).await,
        Command::Record(args) => commands::record::run(args).await,
        Command::Replay(args) => commands::replay::run(args).await,
        Command::Screen(args) => commands::screen::run(args).await,
    }
}
