use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

use hls_core::{
    HlsError, HlsResult,
    data_gap::DataGap,
    market_state::{CandleEvent, MarketEvent},
    time::now_millis,
};
use sha2::{Digest, Sha256};

use crate::{
    metadata::{
        BackfillAttemptRecord, BackfillConfidenceImpact, BackfillStatus, FileRegistryEntry,
        MetadataRegistry,
    },
    paths::{prepare_data_file_path, validate_run_id},
};

const SOURCE_CANDLE_SNAPSHOT: &str = "public_rest_candleSnapshot";

#[derive(Clone, Debug)]
pub struct BackfillGapsOptions {
    pub data_dir: PathBuf,
    pub run_id: String,
    pub interval: String,
}

impl BackfillGapsOptions {
    pub fn new(data_dir: impl AsRef<Path>, run_id: impl Into<String>) -> Self {
        Self {
            data_dir: data_dir.as_ref().to_path_buf(),
            run_id: run_id.into(),
            interval: "1m".to_owned(),
        }
    }

    pub fn with_interval(mut self, interval: impl Into<String>) -> Self {
        self.interval = interval.into();
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CandleBackfillRequest<'a> {
    pub symbol: &'a str,
    pub interval: &'a str,
    pub start_time_ms: i64,
    pub end_time_ms: i64,
}

pub trait CandleBackfillSource {
    fn candle_snapshot(&self, request: &CandleBackfillRequest<'_>) -> HlsResult<Vec<CandleEvent>>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackfillGapsSummary {
    pub run_id: String,
    pub gaps_examined: u64,
    pub gaps_repaired: u64,
    pub gaps_partially_repaired: u64,
    pub gaps_unrepaired: u64,
    pub rows_written: u64,
    pub attempts: Vec<BackfillAttemptRecord>,
    pub files: Vec<FileRegistryEntry>,
}

pub fn backfill_public_gaps(
    options: BackfillGapsOptions,
    source: &impl CandleBackfillSource,
) -> HlsResult<BackfillGapsSummary> {
    validate_run_id(&options.run_id)?;
    if !is_supported_candle_interval(&options.interval) {
        return Err(HlsError::Config(
            "backfill interval must be a supported public candle interval".to_owned(),
        ));
    }
    let registry = MetadataRegistry::open(options.data_dir.join("hls.sqlite"))?;
    let Some(run) = registry.get_run(&options.run_id)? else {
        return Err(HlsError::Config(format!(
            "recording run '{}' was not found",
            options.run_id
        )));
    };
    if !run.normalized_enabled {
        return Err(HlsError::Config(format!(
            "recording run '{}' has no normalized dataset to append backfill rows",
            options.run_id
        )));
    }

    let gaps = registry.list_gaps(&options.run_id)?;
    let mut summary = BackfillGapsSummary {
        run_id: options.run_id.clone(),
        gaps_examined: 0,
        gaps_repaired: 0,
        gaps_partially_repaired: 0,
        gaps_unrepaired: 0,
        rows_written: 0,
        attempts: Vec::new(),
        files: Vec::new(),
    };

    for gap in gaps {
        if gap.recovered {
            continue;
        }
        summary.gaps_examined += 1;
        let result = backfill_gap(&options, source, &registry, &gap)?;
        match result.attempt.status {
            BackfillStatus::Repaired => summary.gaps_repaired += 1,
            BackfillStatus::PartiallyRepaired => summary.gaps_partially_repaired += 1,
            BackfillStatus::Unrepaired => summary.gaps_unrepaired += 1,
        }
        summary.rows_written += result.attempt.rows_written;
        summary.files.extend(result.file);
        summary.attempts.push(result.attempt);
    }

    Ok(summary)
}

struct BackfilledGap {
    attempt: BackfillAttemptRecord,
    file: Option<FileRegistryEntry>,
}

fn backfill_gap(
    options: &BackfillGapsOptions,
    source: &impl CandleBackfillSource,
    registry: &MetadataRegistry,
    gap: &DataGap,
) -> HlsResult<BackfilledGap> {
    let start_time_ms = ns_to_ms_i64(gap.started_at_ns)?;
    let end_time_ms = ns_to_ms_i64(gap.ended_at_ns)?;
    let mut candles = Vec::new();

    for symbol in &gap.affected_symbols {
        let mut symbol_candles = source.candle_snapshot(&CandleBackfillRequest {
            symbol,
            interval: &options.interval,
            start_time_ms,
            end_time_ms,
        })?;
        candles.append(&mut symbol_candles);
    }

    candles.sort_by(|left, right| {
        left.open_ts_ms
            .cmp(&right.open_ts_ms)
            .then_with(|| left.hl_coin.cmp(&right.hl_coin))
            .then_with(|| left.interval.cmp(&right.interval))
    });

    let status = match candles.is_empty() {
        true => BackfillStatus::Unrepaired,
        false => BackfillStatus::PartiallyRepaired,
    };
    let confidence_impact = match status {
        BackfillStatus::Repaired => BackfillConfidenceImpact::Restored,
        BackfillStatus::PartiallyRepaired => BackfillConfidenceImpact::Partial,
        BackfillStatus::Unrepaired => BackfillConfidenceImpact::Degraded,
    };
    let attempt_index = registry
        .list_backfill_attempts_for_gap(&gap.run_id, &gap.gap_id)?
        .len();
    let attempted_at_ms = now_ms_i64()?;

    let file = if candles.is_empty() {
        None
    } else {
        Some(write_backfilled_candles(
            options,
            gap,
            attempt_index,
            &candles,
        )?)
    };
    if let Some(file) = &file {
        registry.insert_file(file)?;
    }

    let attempt = BackfillAttemptRecord {
        attempt_id: format!(
            "{}:{}:{}:{attempt_index}",
            gap.gap_id, SOURCE_CANDLE_SNAPSHOT, options.interval,
        ),
        run_id: gap.run_id.clone(),
        gap_id: gap.gap_id.clone(),
        source: SOURCE_CANDLE_SNAPSHOT.to_owned(),
        requested_start_ns: gap.started_at_ns,
        requested_end_ns: gap.ended_at_ns,
        attempted_at_ms,
        status,
        rows_written: candles.len() as u64,
        confidence_impact,
        notes: Some(backfill_notes(
            status,
            &options.interval,
            &gap.affected_symbols,
        )),
    };
    registry.insert_backfill_attempt(&attempt)?;

    Ok(BackfilledGap { attempt, file })
}

fn write_backfilled_candles(
    options: &BackfillGapsOptions,
    gap: &DataGap,
    attempt_index: usize,
    candles: &[CandleEvent],
) -> HlsResult<FileRegistryEntry> {
    let safe_gap_id = stable_path_id(&gap.gap_id);
    let relative_path = format!(
        "normalized/events/run={}/backfill-{}-{}-{attempt_index:06}.ndjson",
        options.run_id, safe_gap_id, options.interval,
    );
    let full_path = prepare_data_file_path(&options.data_dir, &relative_path)?;

    let mut file = File::create(&full_path)?;
    for candle in candles {
        let event = MarketEvent::Candle(candle.clone());
        let line = serde_json::to_string(&event)
            .map_err(|err| HlsError::Parse(format!("serialize backfill candle: {err}")))?;
        writeln!(file, "{line}")?;
    }
    file.flush()?;
    let metadata = fs::metadata(&full_path)?;

    Ok(FileRegistryEntry {
        path: relative_path,
        event_type: "normalized_jsonl".to_owned(),
        symbol: None,
        start_ts_ms: candles.iter().map(|candle| candle.open_ts_ms).min(),
        end_ts_ms: candles.iter().map(|candle| candle.close_ts_ms).max(),
        rows: candles.len() as u64,
        bytes: metadata.len(),
        created_at_ms: now_ms_i64()?,
        run_id: options.run_id.clone(),
    })
}

fn backfill_notes(status: BackfillStatus, interval: &str, symbols: &[String]) -> String {
    format!(
        "{status:?} public candleSnapshot interval={interval} symbols={}",
        symbols.join(",")
    )
}

fn ns_to_ms_i64(value: u64) -> HlsResult<i64> {
    i64::try_from(value / 1_000_000)
        .map_err(|_| HlsError::Time("gap timestamp overflowed i64 milliseconds".to_owned()))
}

fn now_ms_i64() -> HlsResult<i64> {
    i64::try_from(now_millis()?)
        .map_err(|_| HlsError::Time("current time overflowed i64 milliseconds".to_owned()))
}

fn is_supported_candle_interval(value: &str) -> bool {
    matches!(
        value,
        "1m" | "3m"
            | "5m"
            | "15m"
            | "30m"
            | "1h"
            | "2h"
            | "4h"
            | "8h"
            | "12h"
            | "1d"
            | "3d"
            | "1w"
            | "1M"
    )
}

fn stable_path_id(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    digest[..8]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
