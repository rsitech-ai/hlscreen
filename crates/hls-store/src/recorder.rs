use std::{
    path::{Path, PathBuf},
    sync::mpsc::sync_channel,
    thread,
};

use hls_core::{HlsError, HlsResult, time::now_millis};
use hls_hyperliquid::ws::parser::parse_ws_ndjson;

use crate::{
    metadata::{FileRegistryEntry, MetadataRegistry, RecordingRun, SymbolRegistryEntry},
    normalized::NormalizedWriter,
    raw::{RawMarketMessage, RawWriter},
};

#[derive(Clone, Debug)]
pub struct RecordOptions {
    pub data_dir: PathBuf,
    pub run_id: String,
    pub symbols: Vec<String>,
    pub raw_enabled: bool,
    pub normalized_enabled: bool,
    pub max_raw_uncompressed_bytes: usize,
    pub raw_channel_capacity: usize,
}

impl RecordOptions {
    pub fn new(
        data_dir: impl AsRef<Path>,
        run_id: impl Into<String>,
        symbols: Vec<String>,
        raw_enabled: bool,
        normalized_enabled: bool,
    ) -> Self {
        Self {
            data_dir: data_dir.as_ref().to_path_buf(),
            run_id: run_id.into(),
            symbols,
            raw_enabled,
            normalized_enabled,
            max_raw_uncompressed_bytes: 8 * 1024 * 1024,
            raw_channel_capacity: 1024,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecordSummary {
    pub run_id: String,
    pub raw_files: Vec<FileRegistryEntry>,
    pub normalized_files: Vec<FileRegistryEntry>,
    pub raw_messages: u64,
    pub normalized_events: u64,
    pub clean_shutdown: bool,
}

pub fn record_fixture_ndjson(raw_ndjson: &str, options: RecordOptions) -> HlsResult<RecordSummary> {
    if !options.raw_enabled && !options.normalized_enabled {
        return Err(HlsError::Config(
            "recording requires --raw, --normalized, or both".to_owned(),
        ));
    }

    let registry = MetadataRegistry::open(options.data_dir.join("hls.sqlite"))?;
    let started_at_ms = now_ms_i64()?;
    registry.insert_run(&RecordingRun::new(
        &options.run_id,
        started_at_ms,
        options.raw_enabled,
        options.normalized_enabled,
    ))?;

    let result = record_fixture_after_run_inserted(raw_ndjson, &options, &registry, started_at_ms);
    match result {
        Ok(mut summary) => {
            registry.finish_run(&options.run_id, now_ms_i64()?, true)?;
            summary.clean_shutdown = true;
            Ok(summary)
        }
        Err(err) => {
            let _ = registry.finish_run(
                &options.run_id,
                now_ms_i64().unwrap_or(started_at_ms),
                false,
            );
            Err(err)
        }
    }
}

fn record_fixture_after_run_inserted(
    raw_ndjson: &str,
    options: &RecordOptions,
    registry: &MetadataRegistry,
    started_at_ms: i64,
) -> HlsResult<RecordSummary> {
    for symbol in &options.symbols {
        registry.insert_symbol(&SymbolRegistryEntry::new(
            symbol,
            started_at_ms,
            started_at_ms,
        ))?;
    }

    let mut raw_files = Vec::new();
    let mut raw_messages = 0;
    if options.raw_enabled {
        let raw_write = write_raw_messages_bounded(raw_ndjson, options, started_at_ms)?;
        raw_messages = raw_write.message_count;
        for file in &raw_write.files {
            registry.insert_file(file)?;
        }
        raw_files = raw_write.files;
    }

    let mut normalized_files = Vec::new();
    let mut normalized_events = 0;
    if options.normalized_enabled {
        let events = parse_ws_ndjson(raw_ndjson)?;
        normalized_events = events.len() as u64;
        let mut writer = NormalizedWriter::new(&options.data_dir, &options.run_id)?;
        let file = writer.write_events(&events)?;
        registry.insert_file(&file)?;
        normalized_files = writer.finish()?;
    }

    Ok(RecordSummary {
        run_id: options.run_id.clone(),
        raw_files,
        normalized_files,
        raw_messages,
        normalized_events,
        clean_shutdown: false,
    })
}

struct RawWriteResult {
    files: Vec<FileRegistryEntry>,
    message_count: u64,
}

fn write_raw_messages_bounded(
    raw_ndjson: &str,
    options: &RecordOptions,
    started_at_ms: i64,
) -> HlsResult<RawWriteResult> {
    let capacity = options.raw_channel_capacity.max(1);
    let (sender, receiver) = sync_channel(capacity);
    let data_dir = options.data_dir.clone();
    let run_id = options.run_id.clone();
    let max_raw_uncompressed_bytes = options.max_raw_uncompressed_bytes;

    let writer = thread::spawn(move || -> HlsResult<Vec<FileRegistryEntry>> {
        let mut writer = RawWriter::new(data_dir, run_id, max_raw_uncompressed_bytes)?;
        for message in receiver {
            writer.write(&message)?;
        }
        writer.finish()
    });

    let send_result = (|| -> HlsResult<u64> {
        let mut message_count = 0_u64;
        for (line_number, line) in raw_ndjson.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let seq = u64::try_from(line_number + 1)
                .map_err(|_| HlsError::Parse("fixture line number overflowed u64".to_owned()))?;
            let recv_ts_ns = (started_at_ms as u64)
                .saturating_mul(1_000_000)
                .saturating_add(seq);
            let message = RawMarketMessage::from_ws_line(recv_ts_ns, 0, seq, line)?;
            sender.send(message).map_err(|err| {
                HlsError::External(format!(
                    "raw recorder channel closed before clean shutdown at seq {seq}: {err}"
                ))
            })?;
            message_count += 1;
        }
        Ok(message_count)
    })();

    drop(sender);

    let files = writer
        .join()
        .map_err(|_| HlsError::External("raw writer thread panicked".to_owned()))??;
    let message_count = send_result?;
    Ok(RawWriteResult {
        files,
        message_count,
    })
}

fn now_ms_i64() -> HlsResult<i64> {
    i64::try_from(now_millis()?)
        .map_err(|_| HlsError::Time("current time overflowed i64 milliseconds".to_owned()))
}
