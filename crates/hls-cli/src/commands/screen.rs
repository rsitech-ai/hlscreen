use std::{fs, path::PathBuf};

use anyhow::{Context, bail};
use clap::Args;
use hls_core::market_state::{FeatureSnapshot, LiveMarketState, MarketEvent};
use hls_hyperliquid::ws::parser::parse_ws_ndjson;
use hls_screen::ScreenRequest;
use hls_store::replay::{ReplayOptions, replay_run};
use hls_tui::app::render_screened_table;

use crate::commands::fees::{apply_fee_profile, feature_engine, load_fee_profile};
use crate::commands::metadata::{attach_metadata, load_metadata_enrichments};
use crate::commands::record::parse_symbols;

#[derive(Debug, Args)]
pub struct ScreenArgs {
    /// Recorded run to replay and screen.
    #[arg(long)]
    pub run_id: Option<String>,

    /// Optional comma-separated feed identifiers to screen.
    #[arg(long)]
    pub symbols: Option<String>,

    /// Maximum fixture-derived symbol count before filtering.
    #[arg(long, default_value_t = 50)]
    pub top: usize,

    /// Built-in screen preset.
    #[arg(long)]
    pub preset: Option<String>,

    /// Screen DSL filter expression.
    #[arg(long)]
    pub r#where: Option<String>,

    /// Screen DSL sort expression.
    #[arg(long)]
    pub sort: Option<String>,

    /// Local recording directory.
    #[arg(long, default_value = ".hls")]
    pub data_dir: PathBuf,

    #[arg(long, hide = true)]
    pub fixture_file: Option<PathBuf>,

    #[arg(long, hide = true)]
    pub metadata_file: Option<PathBuf>,

    /// Apply an explicit local JSON/TOML fee profile for fee-aware filtering; does not query account fee tiers.
    #[arg(long)]
    pub fee_profile_file: Option<PathBuf>,
}

pub async fn run(args: ScreenArgs) -> anyhow::Result<()> {
    let request = ScreenRequest {
        preset: args.preset.clone(),
        where_expr: args.r#where.clone(),
        sort: args.sort.clone(),
    };
    let fee_profile = load_fee_profile(args.fee_profile_file.as_ref())?;
    let mut snapshots = if let Some(fixture_file) = &args.fixture_file {
        snapshots_from_fixture(
            fixture_file,
            parse_symbols(args.symbols.as_deref()),
            args.top,
            fee_profile.as_ref(),
        )?
    } else if let Some(run_id) = &args.run_id {
        let mut snapshots = replay_run(ReplayOptions::new(
            &args.data_dir,
            run_id,
            parse_symbols(args.symbols.as_deref()),
        ))?
        .snapshots;
        apply_fee_profile(&mut snapshots, fee_profile.as_ref());
        snapshots
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
    fee_profile: Option<&hls_core::fees::FeeProfile>,
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
    Ok(feature_engine(fee_profile).snapshots(&state, latest_update_ms(&state)))
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
