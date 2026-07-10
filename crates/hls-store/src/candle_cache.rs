use std::{fs, path::Path, time::Duration};

use hls_core::{
    HlsError, HlsResult,
    market_state::{CandleCompletion, CandleEvent, CandleProvenance},
};
use rusqlite::{Connection, params};

const CANDLE_CACHE_SCHEMA_VERSION: i64 = 1;
const MAX_RECENT_ROWS_PER_SYMBOL: usize = 512;

pub struct CandleCache {
    conn: Connection,
}

impl CandleCache {
    pub fn open(path: impl AsRef<Path>) -> HlsResult<Self> {
        let path = path.as_ref();
        if fs::symlink_metadata(path)
            .map(|metadata| metadata.file_type().is_symlink())
            .unwrap_or(false)
        {
            return Err(HlsError::Config(format!(
                "candle cache database must not be a symbolic link: {}",
                path.display()
            )));
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(path).map_err(sqlite_error)?;
        conn.busy_timeout(Duration::from_secs(5))
            .map_err(sqlite_error)?;
        let cache = Self { conn };
        cache.init()?;
        Ok(cache)
    }

    pub fn upsert_batch(&mut self, candles: &[CandleEvent]) -> HlsResult<usize> {
        for candle in candles {
            validate_candle(candle)?;
        }

        let transaction = self.conn.transaction().map_err(sqlite_error)?;
        let mut changed = 0;
        {
            let mut statement = transaction
                .prepare_cached(
                    "INSERT INTO public_candle_cache (
                        symbol, interval, open_ts_ms, close_ts_ms, recv_ts_ns,
                        open, high, low, close, volume_base, trade_count,
                        provenance, completion
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
                    ON CONFLICT(symbol, interval, open_ts_ms) DO UPDATE SET
                        close_ts_ms = excluded.close_ts_ms,
                        recv_ts_ns = excluded.recv_ts_ns,
                        open = excluded.open,
                        high = excluded.high,
                        low = excluded.low,
                        close = excluded.close,
                        volume_base = excluded.volume_base,
                        trade_count = excluded.trade_count,
                        provenance = excluded.provenance,
                        completion = excluded.completion
                    WHERE excluded.recv_ts_ns >= public_candle_cache.recv_ts_ns",
                )
                .map_err(sqlite_error)?;
            for candle in candles {
                changed += statement
                    .execute(params![
                        candle.hl_coin,
                        candle.interval,
                        candle.open_ts_ms,
                        candle.close_ts_ms,
                        i64::try_from(candle.recv_ts_ns).map_err(|_| {
                            HlsError::Config(
                                "candle receive timestamp exceeds SQLite i64".to_owned(),
                            )
                        })?,
                        candle.open,
                        candle.high,
                        candle.low,
                        candle.close,
                        candle.volume_base,
                        i64::try_from(candle.trade_count).map_err(|_| {
                            HlsError::Config("candle trade count exceeds SQLite i64".to_owned())
                        })?,
                        provenance_label(candle.provenance),
                        completion_label(candle.completion),
                    ])
                    .map_err(sqlite_error)?;
            }
        }
        transaction.commit().map_err(sqlite_error)?;
        Ok(changed)
    }

    pub fn load_recent(
        &self,
        symbols: &[String],
        interval: &str,
        per_symbol_limit: usize,
    ) -> HlsResult<Vec<CandleEvent>> {
        if interval.trim().is_empty() {
            return Err(HlsError::Config(
                "candle cache interval must not be empty".to_owned(),
            ));
        }
        if per_symbol_limit == 0 || per_symbol_limit > MAX_RECENT_ROWS_PER_SYMBOL {
            return Err(HlsError::Config(format!(
                "candle cache row limit must be between 1 and {MAX_RECENT_ROWS_PER_SYMBOL}"
            )));
        }
        let limit = i64::try_from(per_symbol_limit)
            .map_err(|_| HlsError::Config("candle cache row limit overflow".to_owned()))?;
        let mut statement = self
            .conn
            .prepare_cached(
                "SELECT symbol, interval, open_ts_ms, close_ts_ms, recv_ts_ns,
                    open, high, low, close, volume_base, trade_count,
                    provenance, completion
                 FROM public_candle_cache
                 WHERE symbol = ?1 AND interval = ?2
                 ORDER BY open_ts_ms DESC
                 LIMIT ?3",
            )
            .map_err(sqlite_error)?;
        let mut candles = Vec::with_capacity(symbols.len().saturating_mul(per_symbol_limit));
        for symbol in symbols {
            let rows = statement
                .query_map(params![symbol, interval, limit], row_to_candle)
                .map_err(sqlite_error)?;
            for row in rows {
                let candle = row.map_err(sqlite_error)?;
                validate_candle(&candle)?;
                candles.push(candle);
            }
        }
        candles.sort_by(|left, right| {
            left.open_ts_ms
                .cmp(&right.open_ts_ms)
                .then_with(|| left.hl_coin.cmp(&right.hl_coin))
                .then_with(|| left.recv_ts_ns.cmp(&right.recv_ts_ns))
        });
        Ok(candles)
    }

    fn init(&self) -> HlsResult<()> {
        self.conn
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS candle_cache_schema (
                    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
                    schema_version INTEGER NOT NULL
                );
                INSERT OR IGNORE INTO candle_cache_schema (singleton, schema_version)
                    VALUES (1, 1);
                CREATE TABLE IF NOT EXISTS public_candle_cache (
                    symbol TEXT NOT NULL,
                    interval TEXT NOT NULL,
                    open_ts_ms INTEGER NOT NULL,
                    close_ts_ms INTEGER NOT NULL,
                    recv_ts_ns INTEGER NOT NULL,
                    open REAL NOT NULL,
                    high REAL NOT NULL,
                    low REAL NOT NULL,
                    close REAL NOT NULL,
                    volume_base REAL NOT NULL,
                    trade_count INTEGER NOT NULL,
                    provenance TEXT NOT NULL,
                    completion TEXT NOT NULL,
                    PRIMARY KEY (symbol, interval, open_ts_ms)
                );",
            )
            .map_err(sqlite_error)?;
        let version = self
            .conn
            .query_row(
                "SELECT schema_version FROM candle_cache_schema WHERE singleton = 1",
                [],
                |row| row.get::<_, i64>(0),
            )
            .map_err(sqlite_error)?;
        if version != CANDLE_CACHE_SCHEMA_VERSION {
            return Err(HlsError::Config(format!(
                "unsupported candle cache schema version {version}; expected {CANDLE_CACHE_SCHEMA_VERSION}"
            )));
        }
        Ok(())
    }
}

fn validate_candle(candle: &CandleEvent) -> HlsResult<()> {
    if candle.hl_coin.trim().is_empty() || candle.interval.trim().is_empty() {
        return Err(HlsError::Config(
            "cached candle symbol and interval must not be empty".to_owned(),
        ));
    }
    if candle.open_ts_ms < 0
        || candle.close_ts_ms < candle.open_ts_ms
        || candle.recv_ts_ns > i64::MAX as u64
        || candle.trade_count > i64::MAX as u64
    {
        return Err(HlsError::Config(
            "cached candle timestamps or trade count are out of range".to_owned(),
        ));
    }
    let prices = [candle.open, candle.high, candle.low, candle.close];
    if prices
        .iter()
        .any(|value| !value.is_finite() || *value <= 0.0)
        || !candle.volume_base.is_finite()
        || candle.volume_base < 0.0
        || candle.high < candle.low
        || candle.high < candle.open
        || candle.high < candle.close
        || candle.low > candle.open
        || candle.low > candle.close
    {
        return Err(HlsError::Config(
            "cached candle contains invalid OHLCV values".to_owned(),
        ));
    }
    Ok(())
}

fn row_to_candle(row: &rusqlite::Row<'_>) -> rusqlite::Result<CandleEvent> {
    let recv_ts_ns = row.get::<_, i64>(4)?;
    let trade_count = row.get::<_, i64>(10)?;
    if recv_ts_ns < 0 || trade_count < 0 {
        return Err(rusqlite::Error::IntegralValueOutOfRange(4, recv_ts_ns));
    }
    Ok(CandleEvent {
        hl_coin: row.get(0)?,
        interval: row.get(1)?,
        open_ts_ms: row.get(2)?,
        close_ts_ms: row.get(3)?,
        recv_ts_ns: recv_ts_ns as u64,
        open: row.get(5)?,
        high: row.get(6)?,
        low: row.get(7)?,
        close: row.get(8)?,
        volume_base: row.get(9)?,
        trade_count: trade_count as u64,
        provenance: parse_provenance(row.get::<_, String>(11)?.as_str(), 11)?,
        completion: parse_completion(row.get::<_, String>(12)?.as_str(), 12)?,
    })
}

fn provenance_label(provenance: CandleProvenance) -> &'static str {
    match provenance {
        CandleProvenance::WebSocket => "websocket",
        CandleProvenance::RestBootstrap => "rest_bootstrap",
    }
}

fn completion_label(completion: CandleCompletion) -> &'static str {
    match completion {
        CandleCompletion::Open => "open",
        CandleCompletion::Closed => "closed",
    }
}

fn parse_provenance(value: &str, column: usize) -> rusqlite::Result<CandleProvenance> {
    match value {
        "websocket" => Ok(CandleProvenance::WebSocket),
        "rest_bootstrap" => Ok(CandleProvenance::RestBootstrap),
        _ => Err(invalid_enum(column, "candle provenance", value)),
    }
}

fn parse_completion(value: &str, column: usize) -> rusqlite::Result<CandleCompletion> {
    match value {
        "open" => Ok(CandleCompletion::Open),
        "closed" => Ok(CandleCompletion::Closed),
        _ => Err(invalid_enum(column, "candle completion", value)),
    }
}

fn invalid_enum(column: usize, label: &str, value: &str) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        column,
        rusqlite::types::Type::Text,
        Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("invalid {label} '{value}'"),
        )),
    )
}

fn sqlite_error(error: rusqlite::Error) -> HlsError {
    HlsError::External(format!("SQLite candle cache error: {error}"))
}
