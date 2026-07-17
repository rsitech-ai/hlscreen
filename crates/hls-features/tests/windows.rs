use hls_core::market_state::CandleEvent;
use hls_features::windows::{latest_candle_trade_count_z, latest_candle_volume_z};

#[test]
fn candle_anomaly_baselines_exclude_other_intervals() {
    let candles = vec![
        candle(0, "1m", 100.0, 10),
        candle(60_000, "1m", 110.0, 11),
        candle(90_000, "15m", 100_000.0, 100_000),
        candle(120_000, "1m", 130.0, 13),
    ];

    let volume_z = latest_candle_volume_z(&candles).expect("volume z-score is available");
    let trade_count_z =
        latest_candle_trade_count_z(&candles).expect("trade-count z-score is available");

    assert!((volume_z - 5.0).abs() < 1e-12, "volume z-score: {volume_z}");
    assert!(
        (trade_count_z - 5.0).abs() < 1e-12,
        "trade-count z-score: {trade_count_z}"
    );
}

fn candle(open_ts_ms: i64, interval: &str, volume_base: f64, trade_count: u64) -> CandleEvent {
    CandleEvent {
        recv_ts_ns: open_ts_ms as u64 * 1_000_000,
        open_ts_ms,
        close_ts_ms: open_ts_ms + 59_999,
        hl_coin: "@107".to_owned(),
        interval: interval.to_owned(),
        open: 100.0,
        high: 101.0,
        low: 99.0,
        close: 100.0,
        volume_base,
        trade_count,
        provenance: Default::default(),
        completion: Default::default(),
    }
}
