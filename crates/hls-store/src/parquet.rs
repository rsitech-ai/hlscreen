use std::{
    fs::{self, File, OpenOptions},
    path::{Path, PathBuf},
    sync::Arc,
};

use hls_core::{
    HlsError, HlsResult,
    market_state::{FeatureSnapshot, MarketEvent},
    time::now_millis,
};
use parquet::{
    column::writer::ColumnWriter,
    data_type::ByteArray,
    file::reader::{FileReader, SerializedFileReader},
    file::writer::SerializedFileWriter,
    record::RowAccessor,
    schema::parser::parse_message_type,
};

use crate::{
    metadata::{FileRegistryEntry, MetadataRegistry},
    normalized::read_normalized_events,
    paths::{prepare_data_file_path, resolve_registered_data_path, validate_run_id},
    replay::{ReplayOptions, replay_run},
    schema::StorageSchemaManifest,
};

const EVENT_PARQUET_SCHEMA: &str = r#"
message hls_normalized_events_v1 {
  REQUIRED INT64 row_index;
  REQUIRED BINARY event_type (UTF8);
  REQUIRED INT64 recv_ts_ns;
  OPTIONAL BINARY hl_coin (UTF8);
  REQUIRED BINARY event_json (UTF8);
}
"#;

const FEATURE_PARQUET_SCHEMA: &str = r#"
message hls_feature_snapshots_v1 {
  REQUIRED INT64 row_index;
  REQUIRED INT64 snapshot_ts_ms;
  REQUIRED BINARY symbol (UTF8);
  REQUIRED INT64 confidence_score;
  REQUIRED BINARY confidence_level (UTF8);
  REQUIRED BINARY confidence_reasons_json (UTF8);
  OPTIONAL DOUBLE price;
  OPTIONAL DOUBLE mid_px;
  OPTIONAL DOUBLE spread_bps;
  OPTIONAL DOUBLE tob_depth_usd;
  OPTIONAL DOUBLE tob_imbalance;
  REQUIRED DOUBLE liquidity_score;
  REQUIRED DOUBLE momentum_score;
  REQUIRED DOUBLE mean_reversion_score;
  REQUIRED BINARY tradeability_state (UTF8);
  REQUIRED BINARY resilience_state (UTF8);
  REQUIRED BINARY snapshot_json (UTF8);
}
"#;

pub fn export_normalized_events_to_parquet(
    data_dir: impl AsRef<Path>,
    run_id: &str,
) -> HlsResult<FileRegistryEntry> {
    let data_dir = data_dir.as_ref();
    let prepared = prepare_normalized_events_export(data_dir, run_id)?;
    let mut entries = commit_prepared_exports(data_dir, vec![prepared])?;
    Ok(entries.remove(0))
}

pub fn export_feature_snapshots_to_parquet(
    data_dir: impl AsRef<Path>,
    run_id: &str,
) -> HlsResult<FileRegistryEntry> {
    let data_dir = data_dir.as_ref();
    let prepared = prepare_feature_snapshots_export(data_dir, run_id)?;
    let mut entries = commit_prepared_exports(data_dir, vec![prepared])?;
    Ok(entries.remove(0))
}

pub fn export_all_to_parquet(
    data_dir: impl AsRef<Path>,
    run_id: &str,
) -> HlsResult<Vec<FileRegistryEntry>> {
    let data_dir = data_dir.as_ref();
    let events = prepare_normalized_events_export(data_dir, run_id)?;
    let features = prepare_feature_snapshots_export(data_dir, run_id)?;
    commit_prepared_exports(data_dir, vec![events, features])
}

fn prepare_normalized_events_export(data_dir: &Path, run_id: &str) -> HlsResult<PreparedExport> {
    validate_run_id(run_id)?;
    let registry = MetadataRegistry::open(data_dir.join("hls.sqlite"))?;
    require_registered_run(&registry, run_id)?;
    let registered_files = registry.list_files(run_id)?;
    let source_files = registered_files
        .iter()
        .filter(|file| file.event_type == "normalized_jsonl")
        .collect::<Vec<_>>();
    if source_files.is_empty() {
        return Err(HlsError::Config(format!(
            "recording run '{run_id}' has no registered normalized_jsonl files"
        )));
    }
    let mut events = Vec::new();
    for source in source_files {
        let path = resolve_registered_data_path(data_dir, &source.path)?;
        events.extend(read_normalized_events(path)?);
    }

    let relative_path = format!("parquet/events/run={run_id}/part-000000.parquet");
    reject_registered_export(&registered_files, &relative_path)?;
    let full_path = prepare_data_file_path(data_dir, &relative_path)?;
    let schema_path = prepare_data_file_path(
        data_dir,
        &format!("parquet/events/run={run_id}/schema.json"),
    )?;
    reject_existing_export_paths(&full_path, &schema_path)?;

    let pending_parquet = write_events_to_parquet_file(&events, &full_path)?;
    StorageSchemaManifest::current_for_normalized_events().write_to_path(&schema_path)?;
    let pending_schema = PendingEvidenceFile::track(&schema_path);

    let metadata = fs::metadata(&full_path)?;
    Ok(PreparedExport {
        entry: FileRegistryEntry {
            path: relative_path,
            event_type: "normalized_parquet".to_owned(),
            symbol: None,
            start_ts_ms: None,
            end_ts_ms: None,
            rows: events.len() as u64,
            bytes: metadata.len(),
            created_at_ms: now_ms_i64()?,
            run_id: run_id.to_owned(),
        },
        parquet: pending_parquet,
        schema: pending_schema,
    })
}

fn prepare_feature_snapshots_export(data_dir: &Path, run_id: &str) -> HlsResult<PreparedExport> {
    validate_run_id(run_id)?;
    let registry = MetadataRegistry::open(data_dir.join("hls.sqlite"))?;
    require_registered_run(&registry, run_id)?;
    let summary = replay_run(ReplayOptions::new(data_dir, run_id, Vec::new()))?;

    let relative_path = format!("parquet/features/run={run_id}/part-000000.parquet");
    let registered_files = registry.list_files(run_id)?;
    reject_registered_export(&registered_files, &relative_path)?;
    let full_path = prepare_data_file_path(data_dir, &relative_path)?;
    let schema_path = prepare_data_file_path(
        data_dir,
        &format!("parquet/features/run={run_id}/schema.json"),
    )?;
    reject_existing_export_paths(&full_path, &schema_path)?;

    let pending_parquet = write_feature_snapshots_to_parquet_file(
        &summary.snapshots,
        summary.snapshot_ts_ms,
        &full_path,
    )?;
    StorageSchemaManifest::current_for_feature_snapshots().write_to_path(&schema_path)?;
    let pending_schema = PendingEvidenceFile::track(&schema_path);

    let metadata = fs::metadata(&full_path)?;
    Ok(PreparedExport {
        entry: FileRegistryEntry {
            path: relative_path,
            event_type: "feature_snapshot_parquet".to_owned(),
            symbol: None,
            start_ts_ms: Some(summary.snapshot_ts_ms),
            end_ts_ms: Some(summary.snapshot_ts_ms),
            rows: summary.snapshots.len() as u64,
            bytes: metadata.len(),
            created_at_ms: now_ms_i64()?,
            run_id: run_id.to_owned(),
        },
        parquet: pending_parquet,
        schema: pending_schema,
    })
}

fn require_registered_run(registry: &MetadataRegistry, run_id: &str) -> HlsResult<()> {
    if registry.get_run(run_id)?.is_none() {
        return Err(HlsError::Config(format!(
            "recording run '{run_id}' was not found"
        )));
    }
    Ok(())
}

fn reject_registered_export(files: &[FileRegistryEntry], path: &str) -> HlsResult<()> {
    if files.iter().any(|file| file.path == path) {
        return Err(HlsError::Config(format!(
            "Parquet export '{path}' is already registered; existing evidence is append-only"
        )));
    }
    Ok(())
}

fn reject_existing_export_paths(parquet_path: &Path, schema_path: &Path) -> HlsResult<()> {
    for path in [parquet_path, schema_path] {
        if fs::symlink_metadata(path).is_ok() {
            return Err(HlsError::Config(format!(
                "refusing to replace existing export evidence '{}'",
                path.display()
            )));
        }
    }
    Ok(())
}

fn commit_prepared_exports(
    data_dir: &Path,
    prepared: Vec<PreparedExport>,
) -> HlsResult<Vec<FileRegistryEntry>> {
    let entries = prepared
        .iter()
        .map(|export| export.entry.clone())
        .collect::<Vec<_>>();
    let mut registry = MetadataRegistry::open(data_dir.join("hls.sqlite"))?;
    registry.insert_files_atomic(&entries)?;
    Ok(prepared.into_iter().map(PreparedExport::commit).collect())
}

struct PreparedExport {
    entry: FileRegistryEntry,
    parquet: PendingEvidenceFile,
    schema: PendingEvidenceFile,
}

impl PreparedExport {
    fn commit(self) -> FileRegistryEntry {
        self.parquet.commit();
        self.schema.commit();
        self.entry
    }
}

struct PendingEvidenceFile {
    path: PathBuf,
    committed: bool,
}

impl PendingEvidenceFile {
    fn create(path: &Path) -> HlsResult<(Self, File)> {
        let file = OpenOptions::new().write(true).create_new(true).open(path)?;
        Ok((
            Self {
                path: path.to_owned(),
                committed: false,
            },
            file,
        ))
    }

    fn track(path: &Path) -> Self {
        Self {
            path: path.to_owned(),
            committed: false,
        }
    }

    fn commit(mut self) {
        self.committed = true;
    }
}

impl Drop for PendingEvidenceFile {
    fn drop(&mut self) {
        if !self.committed {
            let _ = fs::remove_file(&self.path);
        }
    }
}

pub fn read_normalized_events_from_parquet(path: impl AsRef<Path>) -> HlsResult<Vec<MarketEvent>> {
    let file = File::open(path)?;
    let reader = SerializedFileReader::new(file).map_err(parquet_error)?;
    let mut events = Vec::new();
    for row in reader.get_row_iter(None).map_err(parquet_error)? {
        let row = row.map_err(parquet_error)?;
        let event_json = row
            .get_string(4)
            .map_err(parquet_error)?
            .as_str()
            .to_owned();
        let event = serde_json::from_str::<MarketEvent>(&event_json)
            .map_err(|err| HlsError::Parse(format!("parse parquet event_json: {err}")))?;
        events.push(event);
    }
    Ok(events)
}

fn write_events_to_parquet_file(
    events: &[MarketEvent],
    path: &Path,
) -> HlsResult<PendingEvidenceFile> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let schema = Arc::new(parse_message_type(EVENT_PARQUET_SCHEMA).map_err(parquet_error)?);
    let (pending, file) = PendingEvidenceFile::create(path)?;
    let mut writer =
        SerializedFileWriter::new(file, schema, Default::default()).map_err(parquet_error)?;
    let mut row_group = writer.next_row_group().map_err(parquet_error)?;

    let row_indices = (0..events.len())
        .map(|index| {
            i64::try_from(index)
                .map_err(|_| HlsError::Parse("parquet row index overflowed i64".to_owned()))
        })
        .collect::<HlsResult<Vec<_>>>()?;
    write_i64_column(&mut row_group, &row_indices)?;

    let event_types = events
        .iter()
        .map(|event| ByteArray::from(event_type(event)))
        .collect::<Vec<_>>();
    write_byte_array_column(&mut row_group, &event_types)?;

    let recv_ts_ns = events
        .iter()
        .map(|event| {
            i64::try_from(event.recv_ts_ns())
                .map_err(|_| HlsError::Parse("event recv_ts_ns overflowed parquet i64".to_owned()))
        })
        .collect::<HlsResult<Vec<_>>>()?;
    write_i64_column(&mut row_group, &recv_ts_ns)?;

    let mut hl_coin_levels = Vec::with_capacity(events.len());
    let mut hl_coin_values = Vec::new();
    for event in events {
        if let Some(hl_coin) = event.hl_coin() {
            hl_coin_levels.push(1);
            hl_coin_values.push(ByteArray::from(hl_coin));
        } else {
            hl_coin_levels.push(0);
        }
    }
    write_optional_byte_array_column(&mut row_group, &hl_coin_values, &hl_coin_levels)?;

    let event_json = events
        .iter()
        .map(|event| {
            serde_json::to_string(event)
                .map(|event_json| ByteArray::from(event_json.into_bytes()))
                .map_err(|err| HlsError::Parse(format!("serialize parquet event: {err}")))
        })
        .collect::<HlsResult<Vec<_>>>()?;
    write_byte_array_column(&mut row_group, &event_json)?;

    row_group.close().map_err(parquet_error)?;
    writer.finish().map_err(parquet_error)?;
    Ok(pending)
}

fn write_feature_snapshots_to_parquet_file(
    snapshots: &[FeatureSnapshot],
    snapshot_ts_ms: i64,
    path: &Path,
) -> HlsResult<PendingEvidenceFile> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let schema = Arc::new(parse_message_type(FEATURE_PARQUET_SCHEMA).map_err(parquet_error)?);
    let (pending, file) = PendingEvidenceFile::create(path)?;
    let mut writer =
        SerializedFileWriter::new(file, schema, Default::default()).map_err(parquet_error)?;
    let mut row_group = writer.next_row_group().map_err(parquet_error)?;

    let row_indices = (0..snapshots.len())
        .map(|index| {
            i64::try_from(index)
                .map_err(|_| HlsError::Parse("parquet row index overflowed i64".to_owned()))
        })
        .collect::<HlsResult<Vec<_>>>()?;
    write_i64_column(&mut row_group, &row_indices)?;

    let snapshot_times = vec![snapshot_ts_ms; snapshots.len()];
    write_i64_column(&mut row_group, &snapshot_times)?;

    let symbols = snapshots
        .iter()
        .map(|snapshot| ByteArray::from(snapshot.symbol.as_str()))
        .collect::<Vec<_>>();
    write_byte_array_column(&mut row_group, &symbols)?;

    let confidence_scores = snapshots
        .iter()
        .map(|snapshot| i64::from(snapshot.confidence.score))
        .collect::<Vec<_>>();
    write_i64_column(&mut row_group, &confidence_scores)?;

    let confidence_levels = snapshots
        .iter()
        .map(|snapshot| ByteArray::from(snapshot.confidence.level.as_str()))
        .collect::<Vec<_>>();
    write_byte_array_column(&mut row_group, &confidence_levels)?;

    let confidence_reasons = snapshots
        .iter()
        .map(|snapshot| {
            serde_json::to_string(&snapshot.confidence.reasons)
                .map(|reasons| ByteArray::from(reasons.into_bytes()))
                .map_err(|err| HlsError::Parse(format!("serialize confidence reasons: {err}")))
        })
        .collect::<HlsResult<Vec<_>>>()?;
    write_byte_array_column(&mut row_group, &confidence_reasons)?;

    write_optional_f64_column(
        &mut row_group,
        snapshots.iter().map(|snapshot| snapshot.price),
    )?;
    write_optional_f64_column(
        &mut row_group,
        snapshots.iter().map(|snapshot| snapshot.mid_px),
    )?;
    write_optional_f64_column(
        &mut row_group,
        snapshots.iter().map(|snapshot| snapshot.spread_bps),
    )?;
    write_optional_f64_column(
        &mut row_group,
        snapshots.iter().map(|snapshot| snapshot.tob_depth_usd),
    )?;
    write_optional_f64_column(
        &mut row_group,
        snapshots.iter().map(|snapshot| snapshot.tob_imbalance),
    )?;

    let liquidity_scores = snapshots
        .iter()
        .map(|snapshot| snapshot.liquidity_score)
        .collect::<Vec<_>>();
    write_f64_column(&mut row_group, &liquidity_scores)?;

    let momentum_scores = snapshots
        .iter()
        .map(|snapshot| snapshot.momentum_score)
        .collect::<Vec<_>>();
    write_f64_column(&mut row_group, &momentum_scores)?;

    let mean_reversion_scores = snapshots
        .iter()
        .map(|snapshot| snapshot.mean_reversion_score)
        .collect::<Vec<_>>();
    write_f64_column(&mut row_group, &mean_reversion_scores)?;

    let tradeability_states = snapshots
        .iter()
        .map(|snapshot| ByteArray::from(snapshot.tradeability_state.as_str()))
        .collect::<Vec<_>>();
    write_byte_array_column(&mut row_group, &tradeability_states)?;

    let resilience_states = snapshots
        .iter()
        .map(|snapshot| ByteArray::from(snapshot.resilience_state.as_str()))
        .collect::<Vec<_>>();
    write_byte_array_column(&mut row_group, &resilience_states)?;

    let snapshot_json = snapshots
        .iter()
        .map(|snapshot| {
            serde_json::to_string(snapshot)
                .map(|snapshot_json| ByteArray::from(snapshot_json.into_bytes()))
                .map_err(|err| HlsError::Parse(format!("serialize parquet snapshot: {err}")))
        })
        .collect::<HlsResult<Vec<_>>>()?;
    write_byte_array_column(&mut row_group, &snapshot_json)?;

    row_group.close().map_err(parquet_error)?;
    writer.finish().map_err(parquet_error)?;
    Ok(pending)
}

fn write_i64_column<W: std::io::Write + Send>(
    row_group: &mut parquet::file::writer::SerializedRowGroupWriter<'_, W>,
    values: &[i64],
) -> HlsResult<()> {
    let mut column = next_column(row_group)?;
    match column.untyped() {
        ColumnWriter::Int64ColumnWriter(typed) => {
            typed
                .write_batch(values, None, None)
                .map_err(parquet_error)?;
        }
        _ => {
            return Err(HlsError::Parse(
                "unexpected parquet column type for i64 column".to_owned(),
            ));
        }
    }
    column.close().map_err(parquet_error)?;
    Ok(())
}

fn write_f64_column<W: std::io::Write + Send>(
    row_group: &mut parquet::file::writer::SerializedRowGroupWriter<'_, W>,
    values: &[f64],
) -> HlsResult<()> {
    let mut column = next_column(row_group)?;
    match column.untyped() {
        ColumnWriter::DoubleColumnWriter(typed) => {
            typed
                .write_batch(values, None, None)
                .map_err(parquet_error)?;
        }
        _ => {
            return Err(HlsError::Parse(
                "unexpected parquet column type for f64 column".to_owned(),
            ));
        }
    }
    column.close().map_err(parquet_error)?;
    Ok(())
}

fn write_byte_array_column<W: std::io::Write + Send>(
    row_group: &mut parquet::file::writer::SerializedRowGroupWriter<'_, W>,
    values: &[ByteArray],
) -> HlsResult<()> {
    let mut column = next_column(row_group)?;
    match column.untyped() {
        ColumnWriter::ByteArrayColumnWriter(typed) => {
            typed
                .write_batch(values, None, None)
                .map_err(parquet_error)?;
        }
        _ => {
            return Err(HlsError::Parse(
                "unexpected parquet column type for string column".to_owned(),
            ));
        }
    }
    column.close().map_err(parquet_error)?;
    Ok(())
}

fn write_optional_byte_array_column<W: std::io::Write + Send>(
    row_group: &mut parquet::file::writer::SerializedRowGroupWriter<'_, W>,
    values: &[ByteArray],
    def_levels: &[i16],
) -> HlsResult<()> {
    let mut column = next_column(row_group)?;
    match column.untyped() {
        ColumnWriter::ByteArrayColumnWriter(typed) => {
            typed
                .write_batch(values, Some(def_levels), None)
                .map_err(parquet_error)?;
        }
        _ => {
            return Err(HlsError::Parse(
                "unexpected parquet column type for optional string column".to_owned(),
            ));
        }
    }
    column.close().map_err(parquet_error)?;
    Ok(())
}

fn write_optional_f64_column<W: std::io::Write + Send>(
    row_group: &mut parquet::file::writer::SerializedRowGroupWriter<'_, W>,
    values: impl Iterator<Item = Option<f64>>,
) -> HlsResult<()> {
    let mut present_values = Vec::new();
    let mut def_levels = Vec::new();
    for value in values {
        if let Some(value) = value {
            def_levels.push(1);
            present_values.push(value);
        } else {
            def_levels.push(0);
        }
    }

    let mut column = next_column(row_group)?;
    match column.untyped() {
        ColumnWriter::DoubleColumnWriter(typed) => {
            typed
                .write_batch(&present_values, Some(&def_levels), None)
                .map_err(parquet_error)?;
        }
        _ => {
            return Err(HlsError::Parse(
                "unexpected parquet column type for optional f64 column".to_owned(),
            ));
        }
    }
    column.close().map_err(parquet_error)?;
    Ok(())
}

fn next_column<'a, W: std::io::Write + Send>(
    row_group: &'a mut parquet::file::writer::SerializedRowGroupWriter<'_, W>,
) -> HlsResult<parquet::file::writer::SerializedColumnWriter<'a>> {
    row_group
        .next_column()
        .map_err(parquet_error)?
        .ok_or_else(|| HlsError::Parse("parquet schema has fewer columns than expected".to_owned()))
}

fn event_type(event: &MarketEvent) -> &'static str {
    match event {
        MarketEvent::Trade(_) => "trade",
        MarketEvent::TopOfBook(_) => "top_of_book",
        MarketEvent::OrderBook(_) => "order_book",
        MarketEvent::AssetContext(_) => "asset_context",
        MarketEvent::AllMids(_) => "all_mids",
        MarketEvent::Candle(_) => "candle",
    }
}

fn now_ms_i64() -> HlsResult<i64> {
    i64::try_from(now_millis()?)
        .map_err(|_| HlsError::Time("current time overflowed i64 milliseconds".to_owned()))
}

fn parquet_error(err: parquet::errors::ParquetError) -> HlsError {
    HlsError::External(format!("parquet error: {err}"))
}
