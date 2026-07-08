use std::{fs, path::PathBuf};

use anyhow::{Context, bail};
use clap::Args;
use hls_core::market_state::{FeatureSnapshot, LiveMarketState, MarketEvent};
use hls_features::engine::FeatureEngine;
use hls_hyperliquid::ws::parser::parse_ws_ndjson;
use hls_screen::ScreenRequest;
use hls_store::replay::{ReplayOptions, replay_run};
use hls_tui::app::render_screened_table;

use crate::commands::metadata::{attach_metadata, load_metadata_enrichments};
use crate::commands::record::parse_symbols;

#[derive(Debug, Args)]
pub struct ScreenArgs {
    #[arg(long)]
    pub run_id: Option<String>,

    #[arg(long)]
    pub symbols: Option<String>,

    #[arg(long, default_value_t = 50)]
    pub top: usize,

    #[arg(long)]
    pub preset: Option<String>,

    #[arg(long)]
    pub r#where: Option<String>,

    #[arg(long)]
    pub sort: Option<String>,

    #[arg(long, default_value = ".hls")]
    pub data_dir: PathBuf,

    #[arg(long, hide = true)]
    pub fixture_file: Option<PathBuf>,

    #[arg(long, hide = true)]
    pub metadata_file: Option<PathBuf>,
}

pub async fn run(args: ScreenArgs) -> anyhow::Result<()> {
    let request = ScreenRequest {
        preset: args.preset.clone(),
        where_expr: args.r#where.clone(),
        sort: args.sort.clone(),
    };
    let mut snapshots = if let Some(fixture_file) = &args.fixture_file {
        snapshots_from_fixture(
            fixture_file,
            parse_symbols(args.symbols.as_deref()),
            args.top,
        )?
    } else if let Some(run_id) = &args.run_id {
        replay_run(ReplayOptions::new(
            &args.data_dir,
            run_id,
            parse_symbols(args.symbols.as_deref()),
        ))?
        .snapshots
    } else {
        bail!("screen requires --fixture-file or --run-id in this slice");
    };
    attach_metadata(
        &mut snapshots,
        load_metadata_enrichments(args.metadata_file.as_ref())?,
    );

    print!(
        "{}",
        render_screened_table(&snapshots, "READ-ONLY Hyperliquid spot screen", &request)?
    );

    Ok(())
}

fn snapshots_from_fixture(
    fixture_file: &PathBuf,
    symbols: Vec<String>,
    top: usize,
) -> anyhow::Result<Vec<FeatureSnapshot>> {
    let raw = fs::read_to_string(fixture_file)
        .with_context(|| format!("read {}", fixture_file.display()))?;
    let events = parse_ws_ndjson(&raw)?;
    let symbols = if symbols.is_empty() {
        selected_symbols(&events, top)
    } else {
        symbols
    };
    let mut state = LiveMarketState::new(symbols);
    for event in events {
        state.apply(event)?;
    }
    Ok(FeatureEngine::default().snapshots(&state, latest_update_ms(&state)))
}

fn selected_symbols(events: &[MarketEvent], top: usize) -> Vec<String> {
    let mut symbols: Vec<String> = events
        .iter()
        .filter_map(MarketEvent::hl_coin)
        .map(ToOwned::to_owned)
        .collect();
    symbols.sort();
    symbols.dedup();
    symbols.truncate(top);
    symbols
}

fn latest_update_ms(state: &LiveMarketState) -> i64 {
    state
        .states()
        .filter_map(|symbol_state| symbol_state.last_update_ms)
        .max()
        .unwrap_or_default()
}
