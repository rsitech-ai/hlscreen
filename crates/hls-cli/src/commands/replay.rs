use std::path::PathBuf;

use anyhow::Context;
use clap::Args;
use hls_store::replay::{ReplayOptions, replay_run};
use hls_tui::app::render_main_table;

use crate::commands::record::parse_symbols;

#[derive(Debug, Args)]
pub struct ReplayArgs {
    #[arg(long)]
    pub run_id: String,

    #[arg(long)]
    pub symbols: Option<String>,

    #[arg(long, default_value = ".hls")]
    pub data_dir: PathBuf,
}

pub async fn run(args: ReplayArgs) -> anyhow::Result<()> {
    let summary = replay_run(ReplayOptions::new(
        &args.data_dir,
        &args.run_id,
        parse_symbols(args.symbols.as_deref()),
    ))
    .with_context(|| format!("replay recording run '{}'", args.run_id))?;

    let output = render_main_table(&summary.snapshots).replacen(
        "READ-ONLY Hyperliquid spot live screen",
        "READ-ONLY Hyperliquid spot replay",
        1,
    );
    print!("{output}");

    Ok(())
}
