use std::{io, path::Path};

use hls_core::{
    HlsError, HlsResult, confidence::DataConfidenceSnapshot, data_gap::DataGap,
    market_state::FeatureSnapshot, metadata::MetadataEnrichment,
};
use rusqlite::{Connection, OptionalExtension, params, types::Type};
use serde::{Deserialize, Serialize};

use crate::paths::{validate_registered_data_path, validate_run_id};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RecordingRun {
    pub run_id: String,
    pub started_at_ms: i64,
    pub ended_at_ms: Option<i64>,
    pub config_hash: String,
    pub app_version: String,
    pub git_sha: String,
    pub raw_enabled: bool,
    pub normalized_enabled: bool,
    pub clean_shutdown: Option<bool>,
    pub gap_count: u64,
}

impl RecordingRun {
    pub fn new(
        run_id: impl Into<String>,
        started_at_ms: i64,
        raw_enabled: bool,
        normalized_enabled: bool,
    ) -> Self {
        Self {
            run_id: run_id.into(),
            started_at_ms,
            ended_at_ms: None,
            config_hash: "fixture".to_owned(),
            app_version: env!("CARGO_PKG_VERSION").to_owned(),
            git_sha: "unknown".to_owned(),
            raw_enabled,
            normalized_enabled,
            clean_shutdown: None,
            gap_count: 0,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FileRegistryEntry {
    pub path: String,
    pub event_type: String,
    pub symbol: Option<String>,
    pub start_ts_ms: Option<i64>,
    pub end_ts_ms: Option<i64>,
    pub rows: u64,
    pub bytes: u64,
    pub created_at_ms: i64,
    pub run_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SymbolRegistryEntry {
    pub hl_coin: String,
    pub display_name: String,
    pub first_seen_ms: i64,
    pub last_seen_ms: i64,
}

impl SymbolRegistryEntry {
    pub fn new(hl_coin: impl Into<String>, first_seen_ms: i64, last_seen_ms: i64) -> Self {
        let hl_coin = hl_coin.into();
        Self {
            display_name: hl_coin.clone(),
            hl_coin,
            first_seen_ms,
            last_seen_ms,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ConfidenceSnapshotRecord {
    pub run_id: String,
    pub snapshot_ts_ms: i64,
    pub symbol: String,
    pub confidence: DataConfidenceSnapshot,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MetadataCacheRecord {
    pub symbol: String,
    pub metadata_fetched_at_ms: i64,
    pub metadata: MetadataEnrichment,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum BackfillStatus {
    Repaired,
    PartiallyRepaired,
    Unrepaired,
}

impl BackfillStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Repaired => "repaired",
            Self::PartiallyRepaired => "partially_repaired",
            Self::Unrepaired => "unrepaired",
        }
    }

    fn parse(value: &str, column: usize) -> rusqlite::Result<Self> {
        match value {
            "repaired" => Ok(Self::Repaired),
            "partially_repaired" => Ok(Self::PartiallyRepaired),
            "unrepaired" => Ok(Self::Unrepaired),
            value => Err(invalid_text_enum(column, "backfill status", value)),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum BackfillConfidenceImpact {
    Restored,
    Partial,
    Degraded,
}

impl BackfillConfidenceImpact {
    fn as_str(self) -> &'static str {
        match self {
            Self::Restored => "restored",
            Self::Partial => "partial",
            Self::Degraded => "degraded",
        }
    }

    fn parse(value: &str, column: usize) -> rusqlite::Result<Self> {
        match value {
            "restored" => Ok(Self::Restored),
            "partial" => Ok(Self::Partial),
            "degraded" => Ok(Self::Degraded),
            value => Err(invalid_text_enum(
                column,
                "backfill confidence impact",
                value,
            )),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BackfillAttemptRecord {
    pub attempt_id: String,
    pub run_id: String,
    pub gap_id: String,
    pub source: String,
    pub requested_start_ns: u64,
    pub requested_end_ns: u64,
    pub attempted_at_ms: i64,
    pub status: BackfillStatus,
    pub rows_written: u64,
    pub confidence_impact: BackfillConfidenceImpact,
    pub notes: Option<String>,
}

pub struct MetadataRegistry {
    conn: Connection,
}

impl MetadataRegistry {
    pub fn open(path: impl AsRef<Path>) -> HlsResult<Self> {
        let path = path.as_ref();
        match std::fs::symlink_metadata(path) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(HlsError::Config(format!(
                    "refusing symbolic link at metadata registry '{}'",
                    path.display()
                )));
            }
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(path).map_err(sqlite_error)?;
        let registry = Self { conn };
        registry.init()?;
        Ok(registry)
    }

    pub fn insert_run(&self, run: &RecordingRun) -> HlsResult<()> {
        validate_run_id(&run.run_id)?;
        if self.get_run(&run.run_id)?.is_some() {
            return Err(HlsError::Config(format!(
                "recording run '{}' already exists; choose a unique --run-id to preserve existing evidence",
                run.run_id
            )));
        }
        self.conn
            .execute(
                "INSERT INTO runs (
                    run_id, started_at_ms, ended_at_ms, config_hash, app_version, git_sha,
                    raw_enabled, normalized_enabled, clean_shutdown, gap_count
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    run.run_id,
                    run.started_at_ms,
                    run.ended_at_ms,
                    run.config_hash,
                    run.app_version,
                    run.git_sha,
                    run.raw_enabled,
                    run.normalized_enabled,
                    run.clean_shutdown,
                    run.gap_count
                ],
            )
            .map_err(sqlite_error)?;
        Ok(())
    }

    pub fn finish_run(
        &self,
        run_id: &str,
        ended_at_ms: i64,
        clean_shutdown: bool,
    ) -> HlsResult<()> {
        self.conn
            .execute(
                "UPDATE runs SET ended_at_ms = ?2, clean_shutdown = ?3 WHERE run_id = ?1",
                params![run_id, ended_at_ms, clean_shutdown],
            )
            .map_err(sqlite_error)?;
        Ok(())
    }

    pub fn get_run(&self, run_id: &str) -> HlsResult<Option<RecordingRun>> {
        self.conn
            .query_row(
                "SELECT run_id, started_at_ms, ended_at_ms, config_hash, app_version, git_sha,
                    raw_enabled, normalized_enabled, clean_shutdown, gap_count
                 FROM runs WHERE run_id = ?1",
                [run_id],
                row_to_run,
            )
            .optional()
            .map_err(sqlite_error)
    }

    pub fn insert_file(&self, file: &FileRegistryEntry) -> HlsResult<()> {
        self.validate_file_entry(file)?;
        self.conn
            .execute(
                "INSERT INTO files (
                    path, event_type, symbol, start_ts_ms, end_ts_ms, rows, bytes, created_at_ms, run_id
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    file.path,
                    file.event_type,
                    file.symbol,
                    file.start_ts_ms,
                    file.end_ts_ms,
                    file.rows,
                    file.bytes,
                    file.created_at_ms,
                    file.run_id
                ],
            )
            .map_err(sqlite_error)?;
        Ok(())
    }

    pub fn insert_files_atomic(&mut self, files: &[FileRegistryEntry]) -> HlsResult<()> {
        for file in files {
            self.validate_file_entry(file)?;
        }
        let transaction = self.conn.transaction().map_err(sqlite_error)?;
        for file in files {
            transaction
                .execute(
                    "INSERT INTO files (
                        path, event_type, symbol, start_ts_ms, end_ts_ms, rows, bytes, created_at_ms, run_id
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    params![
                        file.path,
                        file.event_type,
                        file.symbol,
                        file.start_ts_ms,
                        file.end_ts_ms,
                        file.rows,
                        file.bytes,
                        file.created_at_ms,
                        file.run_id
                    ],
                )
                .map_err(sqlite_error)?;
        }
        transaction.commit().map_err(sqlite_error)?;
        Ok(())
    }

    fn validate_file_entry(&self, file: &FileRegistryEntry) -> HlsResult<()> {
        validate_run_id(&file.run_id)?;
        validate_registered_data_path(&file.path)?;
        if self.get_run(&file.run_id)?.is_none() {
            return Err(HlsError::Config(format!(
                "recording run '{}' was not found; file evidence cannot be registered without its run",
                file.run_id
            )));
        }
        Ok(())
    }

    pub fn insert_symbol(&self, symbol: &SymbolRegistryEntry) -> HlsResult<()> {
        self.conn
            .execute(
                "INSERT INTO symbols (hl_coin, display_name, first_seen_ms, last_seen_ms)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(hl_coin) DO UPDATE SET
                    display_name = excluded.display_name,
                    last_seen_ms = excluded.last_seen_ms",
                params![
                    symbol.hl_coin,
                    symbol.display_name,
                    symbol.first_seen_ms,
                    symbol.last_seen_ms
                ],
            )
            .map_err(sqlite_error)?;
        Ok(())
    }

    pub fn list_symbols(&self) -> HlsResult<Vec<SymbolRegistryEntry>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT hl_coin, display_name, first_seen_ms, last_seen_ms
                 FROM symbols ORDER BY hl_coin",
            )
            .map_err(sqlite_error)?;

        let rows = stmt
            .query_map([], row_to_symbol)
            .map_err(sqlite_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(sqlite_error)?;

        Ok(rows)
    }

    pub fn list_files(&self, run_id: &str) -> HlsResult<Vec<FileRegistryEntry>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT path, event_type, symbol, start_ts_ms, end_ts_ms, rows, bytes, created_at_ms, run_id
                 FROM files WHERE run_id = ?1 ORDER BY path",
            )
            .map_err(sqlite_error)?;

        let rows = stmt
            .query_map([run_id], row_to_file)
            .map_err(sqlite_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(sqlite_error)?;

        Ok(rows)
    }

    pub fn insert_gap(&self, gap: &DataGap) -> HlsResult<()> {
        let changed = self
            .conn
            .execute(
                "INSERT OR IGNORE INTO data_gaps (
                    gap_id, run_id, conn_id, started_at_ns, ended_at_ns, reason, affected_symbols, recovered
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    gap.gap_id,
                    gap.run_id,
                    gap.conn_id,
                    gap.started_at_ns,
                    gap.ended_at_ns,
                    gap.reason,
                    serde_json::to_string(&gap.affected_symbols)
                        .map_err(|err| HlsError::Parse(format!("serialize data gap symbols: {err}")))?,
                    gap.recovered
                ],
            )
            .map_err(sqlite_error)?;
        if changed > 0 {
            self.conn
                .execute(
                    "UPDATE runs SET gap_count = gap_count + 1 WHERE run_id = ?1",
                    params![gap.run_id],
                )
                .map_err(sqlite_error)?;
        }
        Ok(())
    }

    pub fn list_gaps(&self, run_id: &str) -> HlsResult<Vec<DataGap>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT gap_id, run_id, conn_id, started_at_ns, ended_at_ns, reason, affected_symbols, recovered
                 FROM data_gaps WHERE run_id = ?1 ORDER BY started_at_ns",
            )
            .map_err(sqlite_error)?;

        let rows = stmt
            .query_map([run_id], row_to_gap)
            .map_err(sqlite_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(sqlite_error)?;

        Ok(rows)
    }

    pub fn mark_gap_recovered(&self, run_id: &str, gap_id: &str, recovered: bool) -> HlsResult<()> {
        let changed = self
            .conn
            .execute(
                "UPDATE data_gaps SET recovered = ?3 WHERE run_id = ?1 AND gap_id = ?2",
                params![run_id, gap_id, recovered],
            )
            .map_err(sqlite_error)?;
        if changed == 0 {
            return Err(HlsError::Config(format!(
                "data gap '{gap_id}' was not found in run '{run_id}'"
            )));
        }
        Ok(())
    }

    pub fn insert_backfill_attempt(&self, attempt: &BackfillAttemptRecord) -> HlsResult<()> {
        self.conn
            .execute(
                "INSERT INTO backfill_attempts (
                    attempt_id, run_id, gap_id, source, requested_start_ns, requested_end_ns,
                    attempted_at_ms, status, rows_written, confidence_impact, notes
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    attempt.attempt_id,
                    attempt.run_id,
                    attempt.gap_id,
                    attempt.source,
                    attempt.requested_start_ns,
                    attempt.requested_end_ns,
                    attempt.attempted_at_ms,
                    attempt.status.as_str(),
                    attempt.rows_written,
                    attempt.confidence_impact.as_str(),
                    attempt.notes
                ],
            )
            .map_err(sqlite_error)?;
        Ok(())
    }

    pub fn insert_backfill_result_atomic(
        &mut self,
        file: Option<&FileRegistryEntry>,
        attempt: &BackfillAttemptRecord,
    ) -> HlsResult<()> {
        if let Some(file) = file {
            self.validate_file_entry(file)?;
        }
        let transaction = self.conn.transaction().map_err(sqlite_error)?;
        if let Some(file) = file {
            transaction
                .execute(
                    "INSERT INTO files (
                        path, event_type, symbol, start_ts_ms, end_ts_ms, rows, bytes, created_at_ms, run_id
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    params![
                        file.path,
                        file.event_type,
                        file.symbol,
                        file.start_ts_ms,
                        file.end_ts_ms,
                        file.rows,
                        file.bytes,
                        file.created_at_ms,
                        file.run_id
                    ],
                )
                .map_err(sqlite_error)?;
        }
        transaction
            .execute(
                "INSERT INTO backfill_attempts (
                    attempt_id, run_id, gap_id, source, requested_start_ns, requested_end_ns,
                    attempted_at_ms, status, rows_written, confidence_impact, notes
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    attempt.attempt_id,
                    attempt.run_id,
                    attempt.gap_id,
                    attempt.source,
                    attempt.requested_start_ns,
                    attempt.requested_end_ns,
                    attempt.attempted_at_ms,
                    attempt.status.as_str(),
                    attempt.rows_written,
                    attempt.confidence_impact.as_str(),
                    attempt.notes
                ],
            )
            .map_err(sqlite_error)?;
        transaction.commit().map_err(sqlite_error)?;
        Ok(())
    }

    pub fn list_backfill_attempts(&self, run_id: &str) -> HlsResult<Vec<BackfillAttemptRecord>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT attempt_id, run_id, gap_id, source, requested_start_ns, requested_end_ns,
                    attempted_at_ms, status, rows_written, confidence_impact, notes
                 FROM backfill_attempts WHERE run_id = ?1
                 ORDER BY attempted_at_ms, attempt_id",
            )
            .map_err(sqlite_error)?;

        let rows = stmt
            .query_map([run_id], row_to_backfill_attempt)
            .map_err(sqlite_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(sqlite_error)?;

        Ok(rows)
    }

    pub fn list_backfill_attempts_for_gap(
        &self,
        run_id: &str,
        gap_id: &str,
    ) -> HlsResult<Vec<BackfillAttemptRecord>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT attempt_id, run_id, gap_id, source, requested_start_ns, requested_end_ns,
                    attempted_at_ms, status, rows_written, confidence_impact, notes
                 FROM backfill_attempts WHERE run_id = ?1 AND gap_id = ?2
                 ORDER BY attempted_at_ms, attempt_id",
            )
            .map_err(sqlite_error)?;

        let rows = stmt
            .query_map(params![run_id, gap_id], row_to_backfill_attempt)
            .map_err(sqlite_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(sqlite_error)?;

        Ok(rows)
    }

    pub fn insert_confidence_snapshot(
        &self,
        run_id: &str,
        snapshot_ts_ms: i64,
        confidence: &DataConfidenceSnapshot,
    ) -> HlsResult<()> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO confidence_snapshots (
                    run_id, snapshot_ts_ms, symbol, score, level, confidence_json
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    run_id,
                    snapshot_ts_ms,
                    &confidence.symbol,
                    confidence.score,
                    format!("{:?}", confidence.level),
                    serde_json::to_string(confidence).map_err(|err| {
                        HlsError::Parse(format!("serialize confidence snapshot: {err}"))
                    })?
                ],
            )
            .map_err(sqlite_error)?;
        Ok(())
    }

    pub fn insert_confidence_snapshots(
        &self,
        run_id: &str,
        snapshot_ts_ms: i64,
        snapshots: &[FeatureSnapshot],
    ) -> HlsResult<()> {
        for snapshot in snapshots {
            self.insert_confidence_snapshot(run_id, snapshot_ts_ms, &snapshot.confidence)?;
        }
        Ok(())
    }

    pub fn list_confidence_snapshots(
        &self,
        run_id: &str,
    ) -> HlsResult<Vec<ConfidenceSnapshotRecord>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT run_id, snapshot_ts_ms, symbol, confidence_json
                 FROM confidence_snapshots WHERE run_id = ?1 ORDER BY snapshot_ts_ms, symbol",
            )
            .map_err(sqlite_error)?;

        let rows = stmt
            .query_map([run_id], row_to_confidence_snapshot)
            .map_err(sqlite_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(sqlite_error)?;

        Ok(rows)
    }

    pub fn list_confidence_snapshots_at(
        &self,
        run_id: &str,
        snapshot_ts_ms: i64,
    ) -> HlsResult<Vec<ConfidenceSnapshotRecord>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT run_id, snapshot_ts_ms, symbol, confidence_json
                 FROM confidence_snapshots
                 WHERE run_id = ?1 AND snapshot_ts_ms = ?2
                 ORDER BY symbol",
            )
            .map_err(sqlite_error)?;

        let rows = stmt
            .query_map(params![run_id, snapshot_ts_ms], row_to_confidence_snapshot)
            .map_err(sqlite_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(sqlite_error)?;

        Ok(rows)
    }

    pub fn upsert_metadata_enrichment(&self, metadata: &MetadataEnrichment) -> HlsResult<()> {
        self.conn
            .execute(
                "INSERT INTO metadata_cache (symbol, metadata_fetched_at_ms, metadata_json)
                 VALUES (?1, ?2, ?3)
                 ON CONFLICT(symbol) DO UPDATE SET
                    metadata_fetched_at_ms = excluded.metadata_fetched_at_ms,
                    metadata_json = excluded.metadata_json",
                params![
                    metadata.feed_identifier,
                    metadata.metadata_fetched_at_ms,
                    serde_json::to_string(metadata).map_err(|err| {
                        HlsError::Parse(format!("serialize metadata enrichment: {err}"))
                    })?
                ],
            )
            .map_err(sqlite_error)?;
        Ok(())
    }

    pub fn get_metadata_enrichment(&self, symbol: &str) -> HlsResult<Option<MetadataCacheRecord>> {
        self.conn
            .query_row(
                "SELECT symbol, metadata_fetched_at_ms, metadata_json
                 FROM metadata_cache WHERE symbol = ?1",
                [symbol],
                row_to_metadata_cache,
            )
            .optional()
            .map_err(sqlite_error)
    }

    pub fn list_metadata_enrichments(&self) -> HlsResult<Vec<MetadataCacheRecord>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT symbol, metadata_fetched_at_ms, metadata_json
                 FROM metadata_cache ORDER BY symbol",
            )
            .map_err(sqlite_error)?;

        let rows = stmt
            .query_map([], row_to_metadata_cache)
            .map_err(sqlite_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(sqlite_error)?;

        Ok(rows)
    }

    fn init(&self) -> HlsResult<()> {
        self.conn
            .execute_batch(
                "
                CREATE TABLE IF NOT EXISTS runs (
                    run_id TEXT PRIMARY KEY,
                    started_at_ms INTEGER NOT NULL,
                    ended_at_ms INTEGER,
                    config_hash TEXT NOT NULL,
                    app_version TEXT NOT NULL,
                    git_sha TEXT NOT NULL,
                    raw_enabled INTEGER NOT NULL,
                    normalized_enabled INTEGER NOT NULL,
                    clean_shutdown INTEGER,
                    gap_count INTEGER NOT NULL
                );
                CREATE TABLE IF NOT EXISTS files (
                    path TEXT PRIMARY KEY,
                    event_type TEXT NOT NULL,
                    symbol TEXT,
                    start_ts_ms INTEGER,
                    end_ts_ms INTEGER,
                    rows INTEGER NOT NULL,
                    bytes INTEGER NOT NULL,
                    created_at_ms INTEGER NOT NULL,
                    run_id TEXT NOT NULL
                );
                CREATE TABLE IF NOT EXISTS symbols (
                    hl_coin TEXT PRIMARY KEY,
                    display_name TEXT,
                    first_seen_ms INTEGER,
                    last_seen_ms INTEGER
                );
                CREATE TABLE IF NOT EXISTS data_gaps (
                    gap_id TEXT PRIMARY KEY,
                    run_id TEXT NOT NULL,
                    conn_id INTEGER NOT NULL,
                    started_at_ns INTEGER NOT NULL,
                    ended_at_ns INTEGER NOT NULL,
                    reason TEXT NOT NULL,
                    affected_symbols TEXT NOT NULL,
                    recovered INTEGER NOT NULL
                );
                CREATE TABLE IF NOT EXISTS backfill_attempts (
                    attempt_id TEXT PRIMARY KEY,
                    run_id TEXT NOT NULL,
                    gap_id TEXT NOT NULL,
                    source TEXT NOT NULL,
                    requested_start_ns INTEGER NOT NULL,
                    requested_end_ns INTEGER NOT NULL,
                    attempted_at_ms INTEGER NOT NULL,
                    status TEXT NOT NULL,
                    rows_written INTEGER NOT NULL,
                    confidence_impact TEXT NOT NULL,
                    notes TEXT
                );
                CREATE INDEX IF NOT EXISTS idx_backfill_attempts_run_gap
                    ON backfill_attempts(run_id, gap_id, attempted_at_ms);
                CREATE TABLE IF NOT EXISTS confidence_snapshots (
                    run_id TEXT NOT NULL,
                    snapshot_ts_ms INTEGER NOT NULL,
                    symbol TEXT NOT NULL,
                    score INTEGER NOT NULL,
                    level TEXT NOT NULL,
                    confidence_json TEXT NOT NULL,
                    PRIMARY KEY(run_id, snapshot_ts_ms, symbol)
                );
                CREATE TABLE IF NOT EXISTS metadata_cache (
                    symbol TEXT PRIMARY KEY,
                    metadata_fetched_at_ms INTEGER NOT NULL,
                    metadata_json TEXT NOT NULL
                );
                ",
            )
            .map_err(sqlite_error)?;
        Ok(())
    }
}

fn row_to_run(row: &rusqlite::Row<'_>) -> rusqlite::Result<RecordingRun> {
    Ok(RecordingRun {
        run_id: row.get(0)?,
        started_at_ms: row.get(1)?,
        ended_at_ms: row.get(2)?,
        config_hash: row.get(3)?,
        app_version: row.get(4)?,
        git_sha: row.get(5)?,
        raw_enabled: row.get(6)?,
        normalized_enabled: row.get(7)?,
        clean_shutdown: row.get(8)?,
        gap_count: row.get(9)?,
    })
}

fn row_to_file(row: &rusqlite::Row<'_>) -> rusqlite::Result<FileRegistryEntry> {
    Ok(FileRegistryEntry {
        path: row.get(0)?,
        event_type: row.get(1)?,
        symbol: row.get(2)?,
        start_ts_ms: row.get(3)?,
        end_ts_ms: row.get(4)?,
        rows: row.get(5)?,
        bytes: row.get(6)?,
        created_at_ms: row.get(7)?,
        run_id: row.get(8)?,
    })
}

fn row_to_symbol(row: &rusqlite::Row<'_>) -> rusqlite::Result<SymbolRegistryEntry> {
    Ok(SymbolRegistryEntry {
        hl_coin: row.get(0)?,
        display_name: row.get(1)?,
        first_seen_ms: row.get(2)?,
        last_seen_ms: row.get(3)?,
    })
}

fn row_to_gap(row: &rusqlite::Row<'_>) -> rusqlite::Result<DataGap> {
    let affected_symbols: String = row.get(6)?;
    let affected_symbols = serde_json::from_str(&affected_symbols)
        .map_err(|err| rusqlite::Error::FromSqlConversionFailure(6, Type::Text, Box::new(err)))?;
    Ok(DataGap {
        gap_id: row.get(0)?,
        run_id: row.get(1)?,
        conn_id: row.get(2)?,
        started_at_ns: row.get(3)?,
        ended_at_ns: row.get(4)?,
        reason: row.get(5)?,
        affected_symbols,
        recovered: row.get(7)?,
    })
}

fn row_to_backfill_attempt(row: &rusqlite::Row<'_>) -> rusqlite::Result<BackfillAttemptRecord> {
    let status: String = row.get(7)?;
    let confidence_impact: String = row.get(9)?;
    Ok(BackfillAttemptRecord {
        attempt_id: row.get(0)?,
        run_id: row.get(1)?,
        gap_id: row.get(2)?,
        source: row.get(3)?,
        requested_start_ns: row.get(4)?,
        requested_end_ns: row.get(5)?,
        attempted_at_ms: row.get(6)?,
        status: BackfillStatus::parse(&status, 7)?,
        rows_written: row.get(8)?,
        confidence_impact: BackfillConfidenceImpact::parse(&confidence_impact, 9)?,
        notes: row.get(10)?,
    })
}

fn row_to_confidence_snapshot(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<ConfidenceSnapshotRecord> {
    let confidence_json: String = row.get(3)?;
    let confidence: DataConfidenceSnapshot = serde_json::from_str(&confidence_json)
        .map_err(|err| rusqlite::Error::FromSqlConversionFailure(3, Type::Text, Box::new(err)))?;
    Ok(ConfidenceSnapshotRecord {
        run_id: row.get(0)?,
        snapshot_ts_ms: row.get(1)?,
        symbol: row.get(2)?,
        confidence,
    })
}

fn row_to_metadata_cache(row: &rusqlite::Row<'_>) -> rusqlite::Result<MetadataCacheRecord> {
    let metadata_json: String = row.get(2)?;
    let metadata: MetadataEnrichment = serde_json::from_str(&metadata_json)
        .map_err(|err| rusqlite::Error::FromSqlConversionFailure(2, Type::Text, Box::new(err)))?;
    Ok(MetadataCacheRecord {
        symbol: row.get(0)?,
        metadata_fetched_at_ms: row.get(1)?,
        metadata,
    })
}

fn sqlite_error(err: rusqlite::Error) -> HlsError {
    HlsError::External(format!("sqlite metadata error: {err}"))
}

fn invalid_text_enum(column: usize, name: &'static str, value: &str) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        column,
        Type::Text,
        Box::new(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid {name}: {value}"),
        )),
    )
}
