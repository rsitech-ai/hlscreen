use std::{
    collections::{BTreeMap, HashSet},
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
// A five-minute sample preserves a full day of local analog history at 288
// candidates per symbol while avoiding one expensive all-symbol feature sweep
// per inbound event. At the observed 310-symbol production scale, retention is
// bounded to 89,280 FeatureSnapshots. FeatureSnapshot owns nested strings and
// vectors, so cardinality is the stable contract; the tradeoff is that
// sub-five-minute historical states are omitted.
const ANALOG_SAMPLE_CADENCE_MS: i64 = 5 * 60 * 1_000;
const ANALOG_MAX_CANDIDATES_PER_SYMBOL: usize = 288;
pub const ANALOG_INDEX_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug)]
struct AnalogReplayPolicy {
    sample_cadence_ms: i64,
    max_candidates_per_symbol: usize,
}

impl Default for AnalogReplayPolicy {
    fn default() -> Self {
        Self {
            sample_cadence_ms: ANALOG_SAMPLE_CADENCE_MS,
            max_candidates_per_symbol: ANALOG_MAX_CANDIDATES_PER_SYMBOL,
        }
    }
}

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
    replay_window_candidates_with_policy(
        events,
        symbols,
        confidence_inputs,
        AnalogReplayPolicy::default(),
    )
}

fn replay_window_candidates_with_policy(
    events: &[MarketEvent],
    symbols: Vec<String>,
    confidence_inputs: &ConfidenceInputs,
    policy: AnalogReplayPolicy,
) -> HlsResult<Vec<AnalogCandidate>> {
    let selected_symbols: HashSet<_> = symbols.iter().cloned().collect();
    let mut state = LiveMarketState::new(symbols);
    let engine = FeatureEngine::default();
    let mut candidates_by_symbol = BTreeMap::new();
    let mut last_sample_ts_ms = None;
    let mut replay_ts_ms = 0;
    let sample_cadence_ms = policy.sample_cadence_ms.max(1);
    let max_candidates_per_symbol = policy.max_candidates_per_symbol.max(1);

    for event in events {
        let affected_symbol = replay_clock_symbol(event, &selected_symbols);
        state.apply(event.clone())?;
        let applied_update_ms = affected_symbol
            .and_then(|symbol| state.symbol_state(symbol))
            .and_then(|symbol_state| symbol_state.last_update_ms)
            .unwrap_or_default();
        replay_ts_ms = replay_ts_ms.max(applied_update_ms);
        let now_ms = replay_ts_ms;
        let sample_is_due = now_ms > 0
            && last_sample_ts_ms.is_none_or(|last_sample_ts_ms| {
                now_ms > last_sample_ts_ms
                    && now_ms.saturating_sub(last_sample_ts_ms) >= sample_cadence_ms
            });
        if sample_is_due {
            capture_candidates(
                &engine,
                &state,
                now_ms,
                confidence_inputs,
                max_candidates_per_symbol,
                &mut candidates_by_symbol,
            );
            last_sample_ts_ms = Some(now_ms);
        }
    }

    let final_ts_ms = replay_ts_ms;
    if final_ts_ms > 0 {
        capture_candidates(
            &engine,
            &state,
            final_ts_ms,
            confidence_inputs,
            max_candidates_per_symbol,
            &mut candidates_by_symbol,
        );
    }

    Ok(candidates_by_symbol
        .into_values()
        .flat_map(BTreeMap::into_values)
        .collect())
}

fn replay_clock_symbol<'a>(
    event: &'a MarketEvent,
    selected_symbols: &HashSet<String>,
) -> Option<&'a str> {
    match event {
        MarketEvent::AllMids(event) => event
            .mids_by_hl_coin
            .keys()
            .find(|symbol| selected_symbols.contains(*symbol))
            .map(String::as_str),
        _ => event
            .hl_coin()
            .filter(|symbol| selected_symbols.contains(*symbol)),
    }
}

fn capture_candidates(
    engine: &FeatureEngine,
    state: &LiveMarketState,
    now_ms: i64,
    confidence_inputs: &ConfidenceInputs,
    max_candidates_per_symbol: usize,
    candidates_by_symbol: &mut BTreeMap<String, BTreeMap<i64, AnalogCandidate>>,
) {
    for snapshot in engine.snapshots_with_confidence_inputs(state, now_ms, confidence_inputs) {
        let symbol = snapshot.symbol.clone();
        let candidates = candidates_by_symbol.entry(symbol.clone()).or_default();
        candidates.insert(
            now_ms,
            AnalogCandidate {
                symbol,
                snapshot_ts_ms: now_ms,
                snapshot,
            },
        );
        while candidates.len() > max_candidates_per_symbol {
            let Some(oldest_ts_ms) = candidates.first_key_value().map(|(ts_ms, _)| *ts_ms) else {
                break;
            };
            candidates.remove(&oldest_ts_ms);
        }
    }
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

#[cfg(test)]
mod tests {
    use hls_core::confidence::ConfidenceReason;
    use hls_core::market_state::{TopOfBookEvent, TradeEvent, TradeSide};

    use super::*;

    #[test]
    fn dense_events_do_not_generate_a_candidate_at_every_event_timestamp() {
        let events: Vec<_> = (1..=20)
            .map(|second| top_of_book("@107", second * 1_000, second as f64))
            .collect();

        let candidates = replay_window_candidates(
            &events,
            vec!["@107".to_owned()],
            &ConfidenceInputs::default(),
        )
        .expect("replay succeeds");

        assert_eq!(
            candidates.len(),
            2,
            "the production cadence should retain the first sample and final state"
        );
        assert_eq!(candidates[0].snapshot_ts_ms, 1_000);
        assert_eq!(candidates[1].snapshot_ts_ms, 20_000);
    }

    #[test]
    fn production_policy_caps_each_symbol_and_retains_the_newest_final_state() {
        let events: Vec<_> = (0..290)
            .map(|sample| {
                let ts_ms = 1_000 + sample * 300_000;
                top_of_book("@107", ts_ms, 100.0 + sample as f64)
            })
            .collect();

        let candidates = replay_window_candidates(
            &events,
            vec!["@107".to_owned()],
            &ConfidenceInputs::default(),
        )
        .expect("replay succeeds");

        assert_eq!(candidates.len(), 288);
        assert_eq!(
            candidates.last().map(|candidate| candidate.snapshot_ts_ms),
            Some(1_000 + 289 * 300_000)
        );
        assert_eq!(
            candidates
                .last()
                .and_then(|candidate| candidate.snapshot.bid_px),
            Some(389.0),
            "the final state must survive eviction"
        );
    }

    #[test]
    fn production_policy_is_one_day_at_five_minute_resolution() {
        let policy = AnalogReplayPolicy::default();

        assert_eq!(policy.sample_cadence_ms, 5 * 60 * 1_000);
        assert_eq!(policy.max_candidates_per_symbol, 288);
        assert_eq!(310 * policy.max_candidates_per_symbol, 89_280);
    }

    #[test]
    fn injected_policy_samples_deterministically_and_replaces_the_final_state() {
        let events = vec![
            top_of_book("@107", 1_000, 100.0),
            top_of_book("@107", 1_050, 101.0),
            top_of_book("@107", 1_100, 102.0),
            top_of_book("@107", 1_100, 103.0),
            top_of_book("@107", 1_090, 999.0),
            top_of_book("@107", 1_150, 104.0),
        ];
        let policy = AnalogReplayPolicy {
            sample_cadence_ms: 100,
            max_candidates_per_symbol: 10,
        };

        let candidates = replay_window_candidates_with_policy(
            &events,
            vec!["@107".to_owned()],
            &ConfidenceInputs::default(),
            policy,
        )
        .expect("replay succeeds");

        assert_eq!(
            candidates
                .iter()
                .map(|candidate| candidate.snapshot_ts_ms)
                .collect::<Vec<_>>(),
            vec![1_000, 1_100, 1_150]
        );
        assert_eq!(
            candidates
                .last()
                .and_then(|candidate| candidate.snapshot.bid_px),
            Some(104.0),
            "the replay's final state replaces or appends the final timestamp"
        );
    }

    #[test]
    fn multi_symbol_policy_bounds_cardinality_and_orders_output_stably() {
        let events = vec![
            top_of_book("B", 1_000, 10.0),
            top_of_book("A", 1_100, 20.0),
            top_of_book("B", 1_200, 30.0),
            top_of_book("A", 1_300, 40.0),
        ];
        let symbols = vec!["B".to_owned(), "A".to_owned()];
        let confidence_inputs = ConfidenceInputs::default().with_gap_symbol("A");
        let policy = AnalogReplayPolicy {
            sample_cadence_ms: 100,
            max_candidates_per_symbol: 2,
        };

        let first = replay_window_candidates_with_policy(
            &events,
            symbols.clone(),
            &confidence_inputs,
            policy,
        )
        .expect("first replay succeeds");
        let second =
            replay_window_candidates_with_policy(&events, symbols, &confidence_inputs, policy)
                .expect("second replay succeeds");

        assert_eq!(first, second);
        assert_eq!(first.len(), 4);
        assert_eq!(
            first
                .iter()
                .map(|candidate| (candidate.symbol.as_str(), candidate.snapshot_ts_ms))
                .collect::<Vec<_>>(),
            vec![("A", 1_200), ("A", 1_300), ("B", 1_200), ("B", 1_300)]
        );
        assert!(
            first
                .iter()
                .filter(|candidate| candidate.symbol == "A")
                .all(|candidate| candidate
                    .snapshot
                    .confidence
                    .has_reason(ConfidenceReason::ReconnectGap))
        );
    }

    #[test]
    fn ignored_and_duplicate_events_do_not_advance_the_replay_clock() {
        let events = vec![
            top_of_book("outside", 999_999, 999.0),
            trade("A", 1_000, "duplicate", 100.0),
            trade("A", 999_999, "duplicate", 999.0),
        ];

        let candidates = replay_window_candidates_with_policy(
            &events,
            vec!["A".to_owned()],
            &ConfidenceInputs::default(),
            AnalogReplayPolicy {
                sample_cadence_ms: 100,
                max_candidates_per_symbol: 10,
            },
        )
        .expect("replay succeeds");

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].snapshot_ts_ms, 1_000);
        assert_eq!(candidates[0].snapshot.price, Some(100.0));
    }

    fn top_of_book(symbol: &str, exchange_ts_ms: i64, bid_price: f64) -> MarketEvent {
        MarketEvent::TopOfBook(TopOfBookEvent {
            recv_ts_ns: u64::try_from(exchange_ts_ms)
                .unwrap_or_default()
                .saturating_mul(1_000_000),
            exchange_ts_ms,
            hl_coin: symbol.to_owned(),
            bid_price: Some(bid_price),
            bid_size: Some(1.0),
            bid_order_count: Some(1),
            ask_price: Some(bid_price + 1.0),
            ask_size: Some(1.0),
            ask_order_count: Some(1),
        })
    }

    fn trade(symbol: &str, exchange_ts_ms: i64, id: &str, price: f64) -> MarketEvent {
        MarketEvent::Trade(TradeEvent {
            recv_ts_ns: u64::try_from(exchange_ts_ms)
                .unwrap_or_default()
                .saturating_mul(1_000_000),
            exchange_ts_ms,
            hl_coin: symbol.to_owned(),
            side: TradeSide::Buy,
            price,
            size: 1.0,
            notional: price,
            hash: id.to_owned(),
            tid: 1,
            unique_trade_id: id.to_owned(),
        })
    }
}
