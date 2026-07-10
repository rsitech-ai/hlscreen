use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use hls_core::{
    HlsError, HlsResult,
    market_state::{FeatureSnapshot, LiveMarketState, MarketEvent},
};
use hls_features::engine::{ConfidenceInputs, FeatureEngine};
use serde::{Deserialize, Serialize};

use crate::{
    metadata::MetadataRegistry,
    normalized::read_normalized_events,
    paths::{resolve_registered_data_path, validate_run_id},
};

const MIN_COMPARABLE_FIELDS: usize = 3;
pub const ANALOG_INDEX_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnalogSearchOptions {
    pub limit: usize,
    pub min_candidates: usize,
}

impl Default for AnalogSearchOptions {
    fn default() -> Self {
        Self {
            limit: 5,
            min_candidates: 1,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AnalogSearchRunOptions {
    pub data_dir: PathBuf,
    pub run_id: String,
    pub symbol: String,
    pub search: AnalogSearchOptions,
}

impl AnalogSearchRunOptions {
    pub fn new(
        data_dir: impl AsRef<Path>,
        run_id: impl Into<String>,
        symbol: impl Into<String>,
        search: AnalogSearchOptions,
    ) -> Self {
        Self {
            data_dir: data_dir.as_ref().to_path_buf(),
            run_id: run_id.into(),
            symbol: symbol.into(),
            search,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AnalogCandidate {
    pub symbol: String,
    pub snapshot_ts_ms: i64,
    pub snapshot: FeatureSnapshot,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AnalogIndex {
    pub schema_version: u32,
    pub source_run_id: String,
    pub target_symbol: String,
    pub target_ts_ms: i64,
    pub target_snapshot: FeatureSnapshot,
    pub candidates: Vec<AnalogCandidate>,
}

impl AnalogIndex {
    pub fn new(
        source_run_id: impl Into<String>,
        target_symbol: impl Into<String>,
        target_ts_ms: i64,
        target_snapshot: FeatureSnapshot,
        candidates: Vec<AnalogCandidate>,
    ) -> Self {
        Self {
            schema_version: ANALOG_INDEX_SCHEMA_VERSION,
            source_run_id: source_run_id.into(),
            target_symbol: target_symbol.into(),
            target_ts_ms,
            target_snapshot,
            candidates,
        }
    }

    pub fn validate(&self) -> HlsResult<()> {
        if self.schema_version != ANALOG_INDEX_SCHEMA_VERSION {
            return Err(HlsError::Config(format!(
                "unsupported analog index schema_version {}; expected {}",
                self.schema_version, ANALOG_INDEX_SCHEMA_VERSION
            )));
        }
        if self.source_run_id.trim().is_empty() {
            return Err(HlsError::Config(
                "analog index source_run_id is required".to_owned(),
            ));
        }
        if self.target_symbol.trim().is_empty() {
            return Err(HlsError::Config(
                "analog index target_symbol is required".to_owned(),
            ));
        }
        if self.target_snapshot.symbol != self.target_symbol {
            return Err(HlsError::Config(format!(
                "analog index target snapshot symbol '{}' does not match target_symbol '{}'",
                self.target_snapshot.symbol, self.target_symbol
            )));
        }
        Ok(())
    }

    pub fn write_json(&self, path: &Path) -> HlsResult<()> {
        self.validate()?;
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            fs::create_dir_all(parent)?;
        }
        let encoded = serde_json::to_string_pretty(self)
            .map_err(|err| HlsError::Parse(format!("encode analog index: {err}")))?;
        fs::write(path, encoded)?;
        Ok(())
    }

    pub fn read_json(path: &Path) -> HlsResult<Self> {
        let raw = fs::read_to_string(path)?;
        let index: Self = serde_json::from_str(&raw).map_err(|err| {
            HlsError::Parse(format!("parse analog index {}: {err}", path.display()))
        })?;
        index.validate()?;
        Ok(index)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AnalogSearchReport {
    pub run_id: Option<String>,
    pub target_symbol: String,
    pub target_ts_ms: i64,
    pub candidate_count: usize,
    pub insufficient_evidence: Option<String>,
    pub matches: Vec<AnalogMatch>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AnalogMatch {
    pub symbol: String,
    pub snapshot_ts_ms: i64,
    pub distance: f64,
    pub drivers: Vec<AnalogDriver>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AnalogDriver {
    pub field: String,
    pub target: f64,
    pub candidate: f64,
    pub contribution: f64,
}

pub fn search_analogs(
    run_id: Option<&str>,
    target: &FeatureSnapshot,
    target_ts_ms: i64,
    candidates: &[AnalogCandidate],
    options: AnalogSearchOptions,
) -> AnalogSearchReport {
    let mut matches: Vec<_> = candidates
        .iter()
        .filter(|candidate| {
            !(candidate.symbol == target.symbol && candidate.snapshot_ts_ms == target_ts_ms)
        })
        .filter_map(|candidate| score_candidate(target, candidate))
        .collect();

    matches.sort_by(|left, right| {
        left.distance
            .total_cmp(&right.distance)
            .then_with(|| left.symbol.cmp(&right.symbol))
            .then_with(|| left.snapshot_ts_ms.cmp(&right.snapshot_ts_ms))
    });

    let candidate_count = matches.len();
    let required_candidates = options.min_candidates.max(1);
    let limit = options.limit.max(1);
    let insufficient_evidence = if candidate_count < required_candidates {
        Some(format!(
            "insufficient comparable analog evidence: {candidate_count} candidates with at least {MIN_COMPARABLE_FIELDS} comparable fields; required {required_candidates}"
        ))
    } else {
        None
    };

    if insufficient_evidence.is_some() {
        matches.clear();
    } else {
        matches.truncate(limit);
    }

    AnalogSearchReport {
        run_id: run_id.map(ToOwned::to_owned),
        target_symbol: target.symbol.clone(),
        target_ts_ms,
        candidate_count,
        insufficient_evidence,
        matches,
    }
}

pub fn search_analogs_for_run(options: AnalogSearchRunOptions) -> HlsResult<AnalogSearchReport> {
    let search_options = options.search.clone();
    let index = build_analog_index_for_run(options)?;
    search_analogs_in_index(&index, search_options)
}

pub fn search_analogs_in_index(
    index: &AnalogIndex,
    options: AnalogSearchOptions,
) -> HlsResult<AnalogSearchReport> {
    index.validate()?;
    Ok(search_analogs(
        Some(index.source_run_id.as_str()),
        &index.target_snapshot,
        index.target_ts_ms,
        &index.candidates,
        options,
    ))
}

pub fn build_analog_index_for_run(options: AnalogSearchRunOptions) -> HlsResult<AnalogIndex> {
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

    let files = registry.list_files(&options.run_id)?;
    let mut events = Vec::new();
    for file in files
        .iter()
        .filter(|file| file.event_type == "normalized_jsonl")
    {
        let path = resolve_registered_data_path(&options.data_dir, &file.path)?;
        events.extend(read_normalized_events(path)?);
    }
    if events.is_empty() {
        return Err(HlsError::Config(format!(
            "recording run '{}' has no normalized events to search",
            options.run_id
        )));
    }

    let symbols = selected_symbols(&events);
    let confidence_inputs = confidence_inputs_from_gaps(&registry, &options.run_id)?;
    let candidates = replay_window_candidates(&events, symbols, &confidence_inputs)?;
    let target = candidates
        .iter()
        .rev()
        .find(|candidate| candidate.symbol == options.symbol)
        .cloned()
        .ok_or_else(|| {
            HlsError::Config(format!(
                "symbol '{}' was not found in replayed analog windows",
                options.symbol
            ))
        })?;

    Ok(AnalogIndex::new(
        options.run_id,
        target.symbol,
        target.snapshot_ts_ms,
        target.snapshot,
        candidates,
    ))
}

fn replay_window_candidates(
    events: &[MarketEvent],
    symbols: Vec<String>,
    confidence_inputs: &ConfidenceInputs,
) -> HlsResult<Vec<AnalogCandidate>> {
    let mut state = LiveMarketState::new(symbols);
    let engine = FeatureEngine::default();
    let mut candidates = BTreeMap::new();

    for event in events.iter().cloned() {
        state.apply(event)?;
        let now_ms = latest_update_ms(&state);
        if now_ms <= 0 {
            continue;
        }
        for snapshot in engine.snapshots_with_confidence_inputs(&state, now_ms, confidence_inputs) {
            candidates.insert(
                (snapshot.symbol.clone(), now_ms),
                AnalogCandidate {
                    symbol: snapshot.symbol.clone(),
                    snapshot_ts_ms: now_ms,
                    snapshot,
                },
            );
        }
    }

    Ok(candidates.into_values().collect())
}

fn score_candidate(target: &FeatureSnapshot, candidate: &AnalogCandidate) -> Option<AnalogMatch> {
    let mut drivers = Vec::new();
    push_optional_driver(
        &mut drivers,
        "spread_bps",
        target.spread_bps,
        candidate.snapshot.spread_bps,
        100.0,
    );
    push_optional_driver(
        &mut drivers,
        "tob_imbalance",
        target.tob_imbalance,
        candidate.snapshot.tob_imbalance,
        1.0,
    );
    push_optional_driver(
        &mut drivers,
        "signed_notional_flow_30s",
        target.signed_notional_flow_30s,
        candidate.snapshot.signed_notional_flow_30s,
        10_000.0,
    );
    push_optional_driver(
        &mut drivers,
        "bbo_ofi_proxy_30s",
        target.bbo_ofi_proxy_30s,
        candidate.snapshot.bbo_ofi_proxy_30s,
        10_000.0,
    );
    push_optional_driver(
        &mut drivers,
        "rv_5m",
        target.rv_5m,
        candidate.snapshot.rv_5m,
        1.0,
    );
    push_score_driver(
        &mut drivers,
        "liquidity_score",
        target.liquidity_score,
        candidate.snapshot.liquidity_score,
        100.0,
    );
    push_score_driver(
        &mut drivers,
        "momentum_score",
        target.momentum_score,
        candidate.snapshot.momentum_score,
        100.0,
    );

    if drivers.len() < MIN_COMPARABLE_FIELDS {
        return None;
    }

    let sum_squared = drivers
        .iter()
        .map(|driver| driver.contribution * driver.contribution)
        .sum::<f64>();
    let distance = (sum_squared / drivers.len() as f64).sqrt();
    drivers.sort_by(|left, right| {
        right
            .contribution
            .total_cmp(&left.contribution)
            .then_with(|| left.field.cmp(&right.field))
    });
    drivers.truncate(3);

    Some(AnalogMatch {
        symbol: candidate.symbol.clone(),
        snapshot_ts_ms: candidate.snapshot_ts_ms,
        distance,
        drivers,
    })
}

fn push_optional_driver(
    drivers: &mut Vec<AnalogDriver>,
    field: &str,
    target: Option<f64>,
    candidate: Option<f64>,
    scale: f64,
) {
    let (Some(target), Some(candidate)) = (target, candidate) else {
        return;
    };
    push_driver(drivers, field, target, candidate, scale);
}

fn push_score_driver(
    drivers: &mut Vec<AnalogDriver>,
    field: &str,
    target: f64,
    candidate: f64,
    scale: f64,
) {
    if target.abs() <= f64::EPSILON && candidate.abs() <= f64::EPSILON {
        return;
    }
    push_driver(drivers, field, target, candidate, scale);
}

fn push_driver(
    drivers: &mut Vec<AnalogDriver>,
    field: &str,
    target: f64,
    candidate: f64,
    scale: f64,
) {
    if !target.is_finite() || !candidate.is_finite() {
        return;
    }
    let contribution = ((target - candidate).abs() / scale).max(0.0);
    if contribution.is_finite() {
        drivers.push(AnalogDriver {
            field: field.to_owned(),
            target,
            candidate,
            contribution,
        });
    }
}

fn confidence_inputs_from_gaps(
    registry: &MetadataRegistry,
    run_id: &str,
) -> HlsResult<ConfidenceInputs> {
    let mut inputs = ConfidenceInputs::default();
    for gap in registry.list_gaps(run_id)? {
        for symbol in gap.affected_symbols {
            inputs = inputs.with_gap_symbol(symbol);
        }
    }
    Ok(inputs)
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
