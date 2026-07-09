#![forbid(unsafe_code)]

use clap::{Parser, Subcommand};

use crate::commands::{
    bench::BenchArgs,
    doctor::DoctorArgs,
    explain::ExplainArgs,
    init::InitArgs,
    live::{LiveArgs, TuiArgs},
    record::RecordArgs,
    replay::ReplayArgs,
    screen::ScreenArgs,
    server::ServerArgs,
    symbols::SymbolsArgs,
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
    Bench(BenchArgs),
    Symbols(SymbolsArgs),
    Live(LiveArgs),
    Tui(TuiArgs),
    Record(RecordArgs),
    Replay(ReplayArgs),
    Explain(ExplainArgs),
    Screen(ScreenArgs),
    Server(ServerArgs),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    match Cli::parse().command {
        Command::Init(args) => commands::init::run(args).await,
        Command::Doctor(args) => commands::doctor::run(args).await,
        Command::Bench(args) => commands::bench::run(args).await,
        Command::Symbols(args) => commands::symbols::run(args).await,
        Command::Live(args) => commands::live::run(args).await,
        Command::Tui(args) => commands::live::run_tui(args).await,
        Command::Record(args) => commands::record::run(args).await,
        Command::Replay(args) => commands::replay::run(args).await,
        Command::Explain(args) => commands::explain::run(args).await,
        Command::Screen(args) => commands::screen::run(args).await,
        Command::Server(args) => commands::server::run(args).await,
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::{Cli, Command};
    use crate::commands::live::LiveTuiColor;

    #[test]
    fn tui_command_defaults_to_unbounded_operator_session() {
        let cli = Cli::try_parse_from(["hls", "tui"]).expect("parse tui command");

        let Command::Tui(args) = cli.command else {
            panic!("expected tui command");
        };
        let live_args = args.into_live_args();

        assert!(live_args.tui);
        assert_eq!(live_args.top, 10);
        assert_eq!(live_args.refresh_secs, 1);
        assert_eq!(live_args.duration_secs, 0);
        assert_eq!(live_args.color, LiveTuiColor::Always);
        assert!(!live_args.record);
    }

    #[test]
    fn tui_command_preserves_explicit_user_overrides() {
        let cli = Cli::try_parse_from([
            "hls",
            "tui",
            "--duration-secs",
            "15",
            "--top",
            "25",
            "--refresh-secs",
            "3",
            "--color",
            "auto",
        ])
        .expect("parse tui command overrides");

        let Command::Tui(args) = cli.command else {
            panic!("expected tui command");
        };
        let live_args = args.into_live_args();

        assert_eq!(live_args.top, 25);
        assert_eq!(live_args.refresh_secs, 3);
        assert_eq!(live_args.duration_secs, 15);
        assert_eq!(live_args.color, LiveTuiColor::Auto);
    }
}
