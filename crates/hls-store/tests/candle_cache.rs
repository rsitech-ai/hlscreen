use hls_core::market_state::{CandleCompletion, CandleEvent, CandleProvenance};
use hls_store::candle_cache::CandleCache;

#[test]
fn cache_roundtrip_keeps_the_latest_receive_ordered_candle_and_provenance() {
    let temp = tempfile::tempdir().expect("tempdir");
    let path = temp.path().join("hls.sqlite");
    let mut cache = CandleCache::open(&path).expect("open cache");

    cache
        .upsert_batch(&[
            candle(200, 101.0, CandleCompletion::Open),
            candle(100, 99.0, CandleCompletion::Open),
            candle(300, 102.5, CandleCompletion::Closed),
        ])
        .expect("upsert updates");
    drop(cache);

    let cache = CandleCache::open(&path).expect("reopen cache");
    let loaded = cache
        .load_recent(&["@107".to_owned()], "1m", 10)
        .expect("load recent");

    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].recv_ts_ns, 300);
    assert_eq!(loaded[0].close, 102.5);
    assert_eq!(loaded[0].provenance, CandleProvenance::RestBootstrap);
    assert_eq!(loaded[0].completion, CandleCompletion::Closed);
}

#[test]
fn cache_rejects_invalid_candles_without_partial_writes() {
    let temp = tempfile::tempdir().expect("tempdir");
    let mut cache = CandleCache::open(temp.path().join("hls.sqlite")).expect("open cache");
    let mut invalid = candle(400, 100.0, CandleCompletion::Closed);
    invalid.high = 98.0;

    assert!(cache.upsert_batch(&[invalid]).is_err());
    assert!(
        cache
            .load_recent(&["@107".to_owned()], "1m", 10)
            .expect("load after rejected write")
            .is_empty()
    );
}

#[test]
fn cache_rejects_corrupt_rows_when_loading() {
    let temp = tempfile::tempdir().expect("tempdir");
    let path = temp.path().join("hls.sqlite");
    let mut cache = CandleCache::open(&path).expect("open cache");
    cache
        .upsert_batch(&[candle(500, 100.0, CandleCompletion::Closed)])
        .expect("insert valid row");
    drop(cache);

    let connection = rusqlite::Connection::open(&path).expect("open raw database");
    connection
        .execute("UPDATE public_candle_cache SET high = 1.0", [])
        .expect("corrupt cached OHLC");
    drop(connection);

    let cache = CandleCache::open(&path).expect("reopen cache");
    let error = cache
        .load_recent(&["@107".to_owned()], "1m", 10)
        .expect_err("corrupt row fails closed");
    assert!(error.to_string().contains("invalid OHLCV"));
}

fn candle(recv_ts_ns: u64, close: f64, completion: CandleCompletion) -> CandleEvent {
    CandleEvent {
        recv_ts_ns,
        open_ts_ms: 1_710_000_000_000,
        close_ts_ms: 1_710_000_059_999,
        hl_coin: "@107".to_owned(),
        interval: "1m".to_owned(),
        open: 100.0,
        high: close.max(101.0),
        low: 99.0,
        close,
        volume_base: 25.0,
        trade_count: 12,
        provenance: CandleProvenance::RestBootstrap,
        completion,
    }
}
