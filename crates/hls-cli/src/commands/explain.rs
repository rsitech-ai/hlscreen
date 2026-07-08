use std::{fs, path::PathBuf};

use anyhow::{Context, bail};
use clap::Args;
use hls_core::market_state::{FeatureSnapshot, LiveMarketState};
use hls_features::engine::FeatureEngine;
use hls_hyperliquid::ws::parser::parse_ws_ndjson;
use hls_store::replay::{ReplayOptions, replay_run};
use hls_tui::detail::render_why_ranked_pane;
use serde_json::json;

#[derive(Debug, Args)]
pub struct ExplainArgs {
    #[arg(long)]
    pub symbol: String,

    #[arg(long)]
    pub run_id: Option<String>,

    #[arg(long, default_value = ".hls")]
    pub data_dir: PathBuf,

    #[arg(long)]
    pub json: bool,

    #[arg(long, hide = true)]
    pub fixture_file: Option<PathBuf>,
}

pub async fn run(args: ExplainArgs) -> anyhow::Result<()> {
    let snapshots = if let Some(fixture_file) = &args.fixture_file {
        snapshots_from_fixture(fixture_file, &args.symbol)?
    } else {
        let Some(run_id) = &args.run_id else {
            bail!("explain requires --run-id unless --fixture-file is provided");
        };
        replay_run(ReplayOptions::new(
            &args.data_dir,
            run_id,
            vec![args.symbol.clone()],
        ))
        .with_context(|| format!("replay recording run '{run_id}'"))?
        .snapshots
    };

    let snapshot = snapshots
        .iter()
        .find(|snapshot| snapshot.symbol == args.symbol)
        .with_context(|| format!("symbol '{}' was not found in replayed rows", args.symbol))?;

    if args.json {
        let Some(score_breakdown) = &snapshot.score_breakdown else {
            bail!("symbol '{}' has no score breakdown", snapshot.symbol);
        };
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "symbol": snapshot.symbol,
                "confidence": snapshot.confidence,
                "score_breakdown": score_breakdown,
            }))?
        );
    } else {
        print!("{}", render_why_ranked_pane(snapshot));
    }

    Ok(())
}

fn snapshots_from_fixture(
    fixture_file: &PathBuf,
    symbol: &str,
) -> anyhow::Result<Vec<FeatureSnapshot>> {
    let raw = fs::read_to_string(fixture_file)
        .with_context(|| format!("read {}", fixture_file.display()))?;
    let events = parse_ws_ndjson(&raw)?;
    let mut state = LiveMarketState::new([symbol.to_owned()]);
    for event in events {
        state.apply(event)?;
    }
    Ok(FeatureEngine::default().snapshots(&state, latest_update_ms(&state)))
}

fn latest_update_ms(state: &LiveMarketState) -> i64 {
    state
        .states()
        .filter_map(|symbol_state| symbol_state.last_update_ms)
        .max()
        .unwrap_or_default()
}
