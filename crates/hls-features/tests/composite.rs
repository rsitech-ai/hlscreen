use std::collections::HashMap;

use hls_core::market_state::{CandleEvent, CompositeCoverageState, CompositeVolumeSource};
use hls_features::composite::{build_market_composite, build_market_composite_with_exact_volume};

#[test]
fn composite_uses_exact_public_trade_notional_when_available() {
    let candles = vec![candle("A", 60_000, 10.0, 10.5, 9.8, 10.2, 100.0)];
    let liquidity = HashMap::from([("A".to_owned(), 1_000_000.0)]);
    let exact_quote_volume = HashMap::from([(60_000, 987.5)]);

    let composite =
        build_market_composite_with_exact_volume(&candles, &liquidity, &exact_quote_volume, 1)
            .expect("composite builds");

    assert_eq!(composite[0].quote_volume, 987.5);
    assert_eq!(
        composite[0].volume_source,
        CompositeVolumeSource::ExactTrades
    );
}

#[test]
fn composite_chains_normalized_constituent_returns() {
    let candles = vec![
        candle("A", 0, 10.0, 11.0, 9.5, 10.5, 100.0),
        candle("B", 0, 1_000.0, 1_020.0, 990.0, 1_010.0, 2.0),
        candle("A", 60_000, 10.5, 11.2, 10.4, 11.0, 120.0),
        candle("B", 60_000, 1_010.0, 1_015.0, 980.0, 995.0, 3.0),
    ];
    let liquidity = HashMap::from([("A".to_owned(), 1_000_000.0), ("B".to_owned(), 1_000_000.0)]);

    let composite = build_market_composite(&candles, &liquidity, 2).expect("composite builds");

    assert_eq!(composite.len(), 2);
    assert_eq!(composite[0].open, 100.0);
    assert!((composite[0].close - 103.0).abs() < 1e-9);
    assert!((composite[1].open - composite[0].close).abs() < 1e-9);
    assert!(composite[1].high >= composite[1].open.max(composite[1].close));
    assert!(composite[1].low <= composite[1].open.min(composite[1].close));
    assert_eq!(composite[0].contributing_symbols, 2);
    assert_eq!(composite[0].requested_symbols, 2);
    assert_eq!(composite[0].coverage_state, CompositeCoverageState::Healthy);
    assert_eq!(
        composite[0].volume_source,
        CompositeVolumeSource::CloseApproximation
    );
}

#[test]
fn composite_reports_partial_weight_coverage_without_fabricating_constituents() {
    let candles = vec![
        candle("A", 0, 10.0, 10.5, 9.9, 10.2, 100.0),
        candle("B", 0, 20.0, 20.5, 19.8, 20.2, 100.0),
        candle("A", 60_000, 10.2, 10.4, 10.0, 10.1, 50.0),
    ];
    let liquidity = HashMap::from([("A".to_owned(), 1_000_000.0), ("B".to_owned(), 1_000_000.0)]);

    let composite = build_market_composite(&candles, &liquidity, 2).expect("composite builds");

    assert_eq!(composite[1].contributing_symbols, 1);
    assert_eq!(composite[1].stale_symbols, 1);
    assert_eq!(composite[1].liquidity_weight_coverage, 0.5);
    assert_eq!(composite[1].coverage_state, CompositeCoverageState::Partial);
}

#[test]
fn dominant_liquidity_is_capped_before_composite_returns_are_aggregated() {
    let mut candles = Vec::new();
    let mut liquidity = HashMap::new();
    candles.push(candle("DOM", 0, 100.0, 100.0, 90.0, 90.0, 10.0));
    liquidity.insert("DOM".to_owned(), 1_000_000_000_000.0);
    for index in 0..10 {
        let symbol = format!("S{index}");
        candles.push(candle(&symbol, 0, 100.0, 110.0, 100.0, 110.0, 10.0));
        liquidity.insert(symbol, 1_000_000.0);
    }

    let composite = build_market_composite(&candles, &liquidity, 11).expect("composite builds");

    assert!((composite[0].close - 108.0).abs() < 1e-9);
}

#[test]
fn latest_receive_ordered_candle_wins_deterministically() {
    let mut older = candle("A", 0, 10.0, 10.2, 9.8, 10.1, 10.0);
    older.recv_ts_ns = 1;
    let mut newer = candle("A", 0, 10.0, 11.0, 9.8, 10.8, 12.0);
    newer.recv_ts_ns = 2;
    let liquidity = HashMap::from([("A".to_owned(), 1_000_000.0)]);

    let forward = build_market_composite(&[older.clone(), newer.clone()], &liquidity, 1)
        .expect("forward composite builds");
    let reversed =
        build_market_composite(&[newer, older], &liquidity, 1).expect("reversed composite builds");

    assert_eq!(forward, reversed);
    assert!((forward[0].close - 108.0).abs() < 1e-9);
}

fn candle(
    symbol: &str,
    open_ts_ms: i64,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume_base: f64,
) -> CandleEvent {
    CandleEvent {
        recv_ts_ns: u64::try_from(open_ts_ms.max(0)).unwrap_or_default() * 1_000_000 + 1,
        open_ts_ms,
        close_ts_ms: open_ts_ms + 59_999,
        hl_coin: symbol.to_owned(),
        interval: "1m".to_owned(),
        open,
        high,
        low,
        close,
        volume_base,
        trade_count: 10,
        provenance: Default::default(),
        completion: Default::default(),
    }
}
