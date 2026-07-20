#![forbid(unsafe_code)]

use std::{panic, sync::Once};

use clap::{Parser, Subcommand};

use crate::commands::{
    alerts::AlertsArgs,
    analog::AnalogArgs,
    backfill::BackfillArgs,
    bench::BenchArgs,
    doctor::DoctorArgs,
    explain::ExplainArgs,
    export_parquet::ExportParquetArgs,
    extension::ExtensionArgs,
    init::InitArgs,
    live::{LiveArgs, TuiArgs},
    record::RecordArgs,
    replay::ReplayArgs,
    screen::ScreenArgs,
    server::ServerArgs,
    symbols::SymbolsArgs,
};

mod commands;

pub(crate) const HLS_RENDERER_ID: &str = "ratatui-workstation";
pub(crate) const HLS_VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), " (ratatui-workstation)");

#[derive(Debug, Parser)]
#[command(name = "hls")]
#[command(about = "Read-only Hyperliquid spot screener")]
#[command(version = HLS_VERSION)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Create a local safety/config template and optionally check public REST access.
    Init(InitArgs),
    /// Inspect local safety, storage, terminal, and public-data health.
    Doctor(DoctorArgs),
    /// Validate a versioned public fixture benchmark pack.
    Bench(BenchArgs),
    /// Search bounded replay-backed or prebuilt local analog evidence.
    Analog(AnalogArgs),
    /// Append coarse public candle coverage for recorded reconnect gaps.
    Backfill(BackfillArgs),
    /// Evaluate or inspect local-only alert evidence without external delivery.
    Alerts(AlertsArgs),
    /// List and filter public Hyperliquid spot symbols.
    Symbols(SymbolsArgs),
    /// Stream public market data in table or interactive TUI mode.
    Live(LiveArgs),
    /// Run the interactive read-only Ratatui workstation.
    Tui(TuiArgs),
    /// Record a deterministic fixture stream; network recording uses `live --record`.
    Record(RecordArgs),
    /// Replay a recorded run and optionally verify confidence parity.
    Replay(ReplayArgs),
    /// Explain the score components for one replayed symbol.
    Explain(ExplainArgs),
    /// Export normalized events or feature snapshots to local Parquet files.
    ExportParquet(ExportParquetArgs),
    /// Run a validated, read-only WebAssembly row annotation extension.
    Extension(ExtensionArgs),
    /// Filter and sort replayed or fixture-backed market rows.
    Screen(ScreenArgs),
    /// Serve a loopback-only read-only API for bounded local inspection.
    Server(ServerArgs),
}

static PANIC_HOOK_INSTALL: Once = Once::new();

fn install_terminal_restoring_panic_hook() {
    PANIC_HOOK_INSTALL.call_once(|| {
        let previous = panic::take_hook();
        panic::set_hook(Box::new(move |panic_info| {
            commands::live::handle_terminal_panic(|| previous(panic_info))
        }));
    });
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    install_terminal_restoring_panic_hook();
    match Cli::parse().command {
        Command::Init(args) => commands::init::run(args).await,
        Command::Doctor(args) => commands::doctor::run(args).await,
        Command::Bench(args) => commands::bench::run(args).await,
        Command::Analog(args) => commands::analog::run(args).await,
        Command::Backfill(args) => commands::backfill::run(args).await,
        Command::Alerts(args) => commands::alerts::run(args).await,
        Command::Symbols(args) => commands::symbols::run(args).await,
        Command::Live(args) => commands::live::run(args).await,
        Command::Tui(args) => commands::live::run_tui(args).await,
        Command::Record(args) => commands::record::run(args).await,
        Command::Replay(args) => commands::replay::run(args).await,
        Command::Explain(args) => commands::explain::run(args).await,
        Command::ExportParquet(args) => commands::export_parquet::run(args).await,
        Command::Extension(args) => commands::extension::run(args).await,
        Command::Screen(args) => commands::screen::run(args).await,
        Command::Server(args) => commands::server::run(args).await,
    }
}

#[cfg(test)]
mod tests {
    use clap::{CommandFactory, Parser};

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

    #[test]
    fn public_cli_help_describes_every_command_and_option() {
        let command = Cli::command();
        let mut missing = Vec::new();

        for subcommand in command.get_subcommands() {
            if subcommand.get_about().is_none() {
                missing.push(subcommand.get_name().to_owned());
            }
            for argument in subcommand.get_arguments() {
                if argument.get_id() == "help" || argument.is_hide_set() {
                    continue;
                }
                if argument.get_help().is_none() {
                    missing.push(format!("{}.{}", subcommand.get_name(), argument.get_id()));
                }
            }
        }

        assert!(missing.is_empty(), "missing CLI help: {missing:?}");
    }
}
