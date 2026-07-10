use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use hls_core::{
    HlsError, HlsResult,
    confidence::DataConfidenceSnapshot,
    market_state::{FeatureSnapshot, LiveMarketState, MarketEvent},
};
use hls_features::engine::{ConfidenceInputs, FeatureEngine};

use crate::{
    metadata::{BackfillConfidenceImpact, MetadataRegistry},
    normalized::read_normalized_events,
    parquet::read_normalized_events_from_parquet,
    paths::{resolve_registered_data_path, validate_run_id},
    schema::StorageSchemaManifest,
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ReplayInputFormat {
    #[default]
    Jsonl,
    Parquet,
}

#[derive(Clone, Debug)]
pub struct ReplayOptions {
    pub data_dir: PathBuf,
    pub run_id: String,
    pub symbols: Vec<String>,
    pub input_format: ReplayInputFormat,
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
            input_format: ReplayInputFormat::Jsonl,
        }
    }

    pub fn with_input_format(mut self, input_format: ReplayInputFormat) -> Self {
        self.input_format = input_format;
        self
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReplaySummary {
    pub run_id: String,
    pub events_read: u64,
    pub snapshot_ts_ms: i64,
    pub snapshots: Vec<FeatureSnapshot>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReplayParityReport {
    pub run_id: String,
    pub snapshot_ts_ms: i64,
    pub baseline_written: bool,
    pub matched: bool,
    pub baseline_count: usize,
    pub replay_count: usize,
    pub drift_count: usize,
    pub missing_count: usize,
    pub extra_count: usize,
    pub details: Vec<String>,
}

pub fn replay_run(options: ReplayOptions) -> HlsResult<ReplaySummary> {
    validate_run_id(&options.run_id)?;
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

    let events = read_replay_events(&options, &registry)?;

    if events.is_empty() {
        return Err(HlsError::Config(format!(
            "recording run '{}' has no {} normalized events to replay",
            options.run_id,
            replay_input_label(options.input_format)
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
    let confidence_inputs = confidence_inputs_from_gaps(&registry, &options.run_id)?;
    let snapshots = FeatureEngine::default().snapshots_with_confidence_inputs(
        &state,
        now_ms,
        &confidence_inputs,
    );

    Ok(ReplaySummary {
        run_id: options.run_id,
        events_read: events.len() as u64,
        snapshot_ts_ms: now_ms,
        snapshots,
    })
}

fn read_replay_events(
    options: &ReplayOptions,
    registry: &MetadataRegistry,
) -> HlsResult<Vec<MarketEvent>> {
    let files = registry.list_files(&options.run_id)?;
    let event_type = match options.input_format {
        ReplayInputFormat::Jsonl => "normalized_jsonl",
        ReplayInputFormat::Parquet => "normalized_parquet",
    };

    if options.input_format == ReplayInputFormat::Parquet {
        let schema_path = format!("parquet/events/run={}/schema.json", options.run_id);
        if !options.data_dir.join(&schema_path).exists() {
            return Err(HlsError::Config(format!(
                "recording run '{}' has no normalized-event Parquet schema manifest",
                options.run_id
            )));
        }
        let schema_path = resolve_registered_data_path(&options.data_dir, &schema_path)?;
        StorageSchemaManifest::read_from_path(schema_path)?.validate_supported()?;
    }

    let mut events = Vec::new();
    for file in files.iter().filter(|file| file.event_type == event_type) {
        let path = resolve_registered_data_path(&options.data_dir, &file.path)?;
        match options.input_format {
            ReplayInputFormat::Jsonl => events.extend(read_normalized_events(path)?),
            ReplayInputFormat::Parquet => {
                events.extend(read_normalized_events_from_parquet(path)?);
            }
        }
    }
    Ok(events)
}

fn replay_input_label(input_format: ReplayInputFormat) -> &'static str {
    match input_format {
        ReplayInputFormat::Jsonl => "JSONL",
        ReplayInputFormat::Parquet => "Parquet",
    }
}

pub fn verify_or_insert_confidence_parity(
    options: &ReplayOptions,
    summary: &ReplaySummary,
) -> HlsResult<ReplayParityReport> {
    let registry = MetadataRegistry::open(options.data_dir.join("hls.sqlite"))?;
    let baseline =
        registry.list_confidence_snapshots_at(&summary.run_id, summary.snapshot_ts_ms)?;
    if baseline.is_empty() {
        registry.insert_confidence_snapshots(
            &summary.run_id,
            summary.snapshot_ts_ms,
            &summary.snapshots,
        )?;
        return Ok(ReplayParityReport {
            run_id: summary.run_id.clone(),
            snapshot_ts_ms: summary.snapshot_ts_ms,
            baseline_written: true,
            matched: true,
            baseline_count: summary.snapshots.len(),
            replay_count: summary.snapshots.len(),
            drift_count: 0,
            missing_count: 0,
            extra_count: 0,
            details: Vec::new(),
        });
    }

    Ok(compare_confidence_snapshots(
        &summary.run_id,
        summary.snapshot_ts_ms,
        baseline
            .into_iter()
            .map(|record| (record.symbol, record.confidence))
            .collect(),
        summary
            .snapshots
            .iter()
            .map(|snapshot| (snapshot.symbol.clone(), snapshot.confidence.clone()))
            .collect(),
    ))
}

fn compare_confidence_snapshots(
    run_id: &str,
    snapshot_ts_ms: i64,
    baseline: BTreeMap<String, DataConfidenceSnapshot>,
    replay: BTreeMap<String, DataConfidenceSnapshot>,
) -> ReplayParityReport {
    let mut drift_count = 0;
    let mut missing_count = 0;
    let mut extra_count = 0;
    let mut details = Vec::new();

    for (symbol, replay_confidence) in &replay {
        match baseline.get(symbol) {
            Some(baseline_confidence) if baseline_confidence == replay_confidence => {}
            Some(baseline_confidence) => {
                drift_count += 1;
                details.push(format!(
                    "{symbol}: baseline score={} level={:?} reasons={:?}; replay score={} level={:?} reasons={:?}",
                    baseline_confidence.score,
                    baseline_confidence.level,
                    baseline_confidence.reasons,
                    replay_confidence.score,
                    replay_confidence.level,
                    replay_confidence.reasons
                ));
            }
            None => {
                missing_count += 1;
                details.push(format!("{symbol}: missing confidence baseline"));
            }
        }
    }

    for symbol in baseline.keys() {
        if !replay.contains_key(symbol) {
            extra_count += 1;
            details.push(format!("{symbol}: baseline has no replayed row"));
        }
    }

    let matched = drift_count == 0 && missing_count == 0 && extra_count == 0;
    ReplayParityReport {
        run_id: run_id.to_owned(),
        snapshot_ts_ms,
        baseline_written: false,
        matched,
        baseline_count: baseline.len(),
        replay_count: replay.len(),
        drift_count,
        missing_count,
        extra_count,
        details,
    }
}

fn confidence_inputs_from_gaps(
    registry: &MetadataRegistry,
    run_id: &str,
) -> HlsResult<ConfidenceInputs> {
    let mut inputs = ConfidenceInputs::default();
    for gap in registry.list_gaps(run_id)? {
        if gap.recovered || gap_has_restored_confidence(registry, run_id, &gap.gap_id)? {
            continue;
        }
        for symbol in gap.affected_symbols {
            inputs = inputs.with_gap_symbol(symbol);
        }
    }
    Ok(inputs)
}

fn gap_has_restored_confidence(
    registry: &MetadataRegistry,
    run_id: &str,
    gap_id: &str,
) -> HlsResult<bool> {
    Ok(registry
        .list_backfill_attempts_for_gap(run_id, gap_id)?
        .into_iter()
        .any(|attempt| attempt.confidence_impact == BackfillConfidenceImpact::Restored))
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
