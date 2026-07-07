use std::path::{Path, PathBuf};

use hls_core::{
    HlsError, HlsResult,
    market_state::{FeatureSnapshot, LiveMarketState, MarketEvent},
};
use hls_features::engine::FeatureEngine;

use crate::{metadata::MetadataRegistry, normalized::read_normalized_events};

#[derive(Clone, Debug)]
pub struct ReplayOptions {
    pub data_dir: PathBuf,
    pub run_id: String,
    pub symbols: Vec<String>,
}

impl ReplayOptions {
    pub fn new(
        data_dir: impl AsRef<Path>,
        run_id: impl Into<String>,
        symbols: Vec<String>,
    ) -> Self {
        Self {
            data_dir: data_dir.as_ref().to_path_buf(),
            run_id: run_id.into(),
            symbols,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReplaySummary {
    pub run_id: String,
    pub events_read: u64,
    pub snapshots: Vec<FeatureSnapshot>,
}

pub fn replay_run(options: ReplayOptions) -> HlsResult<ReplaySummary> {
    let registry = MetadataRegistry::open(options.data_dir.join("hls.sqlite"))?;
    let Some(run) = registry.get_run(&options.run_id)? else {
        return Err(HlsError::Config(format!(
            "recording run '{}' was not found",
            options.run_id
        )));
    };
    if run.clean_shutdown != Some(true) {
        return Err(HlsError::Config(format!(
            "recording run '{}' did not finish cleanly",
            options.run_id
        )));
    }

    let files = registry.list_files(&options.run_id)?;
    let mut events = Vec::new();
    for file in files
        .iter()
        .filter(|file| file.event_type == "normalized_jsonl")
    {
        let path = options.data_dir.join(&file.path);
        events.extend(read_normalized_events(path)?);
    }

    if events.is_empty() {
        return Err(HlsError::Config(format!(
            "recording run '{}' has no normalized events to replay",
            options.run_id
        )));
    }

    let symbols = if options.symbols.is_empty() {
        selected_symbols(&events)
    } else {
        options.symbols
    };
    let mut state = LiveMarketState::new(symbols);
    for event in events.iter().cloned() {
        state.apply(event)?;
    }

    let now_ms = latest_update_ms(&state);
    let snapshots = FeatureEngine::default().snapshots(&state, now_ms);

    Ok(ReplaySummary {
        run_id: options.run_id,
        events_read: events.len() as u64,
        snapshots,
    })
}

fn selected_symbols(events: &[MarketEvent]) -> Vec<String> {
    let mut symbols: Vec<String> = events
        .iter()
        .filter_map(MarketEvent::hl_coin)
        .map(ToOwned::to_owned)
        .collect();
    symbols.sort();
    symbols.dedup();
    symbols
}

fn latest_update_ms(state: &LiveMarketState) -> i64 {
    state
        .states()
        .filter_map(|symbol_state| symbol_state.last_update_ms)
        .max()
        .unwrap_or_default()
}
