#![forbid(unsafe_code)]

use std::{panic, sync::Once};

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

static PANIC_HOOK_INSTALL: Once = Once::new();

fn restore_before_delegating_panic(restore: impl FnOnce(), delegate: impl FnOnce()) {
    restore();
    delegate();
}

fn install_terminal_restoring_panic_hook() {
    PANIC_HOOK_INSTALL.call_once(|| {
        let previous = panic::take_hook();
        panic::set_hook(Box::new(move |panic_info| {
            restore_before_delegating_panic(commands::live::restore_active_tui_after_panic, || {
                previous(panic_info)
            });
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
    use std::{cell::RefCell, rc::Rc};

    use clap::Parser;

    use super::{Cli, Command, restore_before_delegating_panic};
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
    fn panic_hook_restores_before_delegating() {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let restore_calls = Rc::clone(&calls);
        let delegate_calls = Rc::clone(&calls);

        restore_before_delegating_panic(
            move || restore_calls.borrow_mut().push("restore"),
            move || delegate_calls.borrow_mut().push("delegate"),
        );

        assert_eq!(*calls.borrow(), vec!["restore", "delegate"]);
    }
}
