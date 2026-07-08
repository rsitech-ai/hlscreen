use std::{fs, path::PathBuf};

use anyhow::{Context, bail};
use clap::Args;
use hls_core::market_state::LiveMarketState;
use hls_features::engine::FeatureEngine;
use hls_hyperliquid::ws::parser::parse_ws_ndjson;
use hls_screen::ScreenRequest;
use hls_store::recorder::{RecordOptions, record_fixture_ndjson};
use hls_tui::app::render_screened_table;

use crate::commands::record::{default_run_id, enabled_outputs, parse_symbols};

#[derive(Debug, Args)]
pub struct LiveArgs {
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

    #[arg(long)]
    pub record: bool,

    #[arg(long)]
    pub raw: bool,

    #[arg(long)]
    pub parquet: bool,

    #[arg(long)]
    pub normalized: bool,

    #[arg(long)]
    pub run_id: Option<String>,

    #[arg(long, default_value = ".hls")]
    pub data_dir: PathBuf,

    #[arg(long, hide = true)]
    pub fixture_file: Option<PathBuf>,

    #[arg(long, hide = true)]
    pub once: bool,
}

pub async fn run(args: LiveArgs) -> anyhow::Result<()> {
    let Some(fixture_file) = &args.fixture_file else {
        bail!(
            "live network mode is not implemented in this slice; use --fixture-file for deterministic mock live"
        );
    };

    if !args.once {
        bail!("fixture-backed live mode currently requires --once");
    }

    let raw = fs::read_to_string(fixture_file)
        .with_context(|| format!("read {}", fixture_file.display()))?;

    if args.record {
        if args.parquet {
            bail!(
                "Parquet output is not implemented in this slice; use --normalized for replayable JSONL"
            );
        }
        let run_id = args.run_id.clone().unwrap_or_else(default_run_id);
        let (raw_enabled, normalized_enabled) = enabled_outputs(args.raw, args.normalized);
        let summary = record_fixture_ndjson(
            &raw,
            RecordOptions::new(
                &args.data_dir,
                &run_id,
                parse_symbols(args.symbols.as_deref()),
                raw_enabled,
                normalized_enabled,
            ),
        )?;
        println!("recording run: {}", summary.run_id);
        println!("clean_shutdown={}", summary.clean_shutdown);
    }

    let events = parse_ws_ndjson(&raw)?;
    let symbols = selected_symbols(&args, &events);
    let mut state = LiveMarketState::new(symbols);

    for event in events {
        state.apply(event)?;
    }

    let snapshots = FeatureEngine::default().snapshots(&state, latest_update_ms(&state));
    print!(
        "{}",
        render_screened_table(
            &snapshots,
            "READ-ONLY Hyperliquid spot live screen",
            &ScreenRequest {
                preset: args.preset,
                where_expr: args.r#where,
                sort: args.sort,
            }
        )?
    );

    Ok(())
}

fn selected_symbols(
    args: &LiveArgs,
    events: &[hls_core::market_state::MarketEvent],
) -> Vec<String> {
    if let Some(symbols) = &args.symbols {
        return symbols
            .split(',')
            .map(str::trim)
            .filter(|symbol| !symbol.is_empty())
            .map(ToOwned::to_owned)
            .collect();
    }

    let mut symbols: Vec<String> = events
        .iter()
        .filter_map(hls_core::market_state::MarketEvent::hl_coin)
        .map(ToOwned::to_owned)
        .collect();
    symbols.sort();
    symbols.dedup();
    symbols.truncate(args.top);
    symbols
}

fn latest_update_ms(state: &LiveMarketState) -> i64 {
    state
        .states()
        .filter_map(|symbol_state| symbol_state.last_update_ms)
        .max()
        .unwrap_or_default()
}
