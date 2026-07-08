use hls_core::market_state::{LiveMarketState, TradeabilityState};
use hls_features::engine::FeatureEngine;
use hls_hyperliquid::ws::parser::parse_ws_ndjson;

#[test]
fn resilient_recovered_book_is_tradeable_after_recovery() {
    let snapshot = snapshot_from_fixture(
        include_str!("../../../tests/fixtures/microstructure/resilience_shock.ndjson"),
        1_710_001_008_000,
    );

    assert_eq!(snapshot.tradeability_state, TradeabilityState::Tradeable);
    assert!(snapshot.spread_bps.expect("spread") > 10.0);
    assert!(snapshot.tob_depth_usd.expect("depth") > 10_000.0);
}

#[test]
fn thin_brittle_book_is_not_reported_tradeable() {
    let snapshot = snapshot_from_fixture(
        include_str!("../../../tests/fixtures/microstructure/thin_brittle_book.ndjson"),
        1_710_002_014_000,
    );

    assert_eq!(snapshot.tradeability_state, TradeabilityState::Thin);
    assert!(snapshot.tob_depth_usd.expect("depth") < 100.0);
    assert!(snapshot.spread_bps.expect("spread") > 300.0);
}

#[test]
fn sparse_trade_only_window_stays_unknown_when_quote_history_is_insufficient() {
    let snapshot = snapshot_from_fixture(
        include_str!("../../../tests/fixtures/microstructure/sparse_trades.ndjson"),
        1_710_000_002_000,
    );

    assert_eq!(snapshot.tradeability_state, TradeabilityState::Unknown);
}

fn snapshot_from_fixture(raw: &str, now_ms: i64) -> hls_core::market_state::FeatureSnapshot {
    let events = parse_ws_ndjson(raw).expect("fixture parses");
    let mut state = LiveMarketState::new(["@107".to_owned()]);
    for event in events {
        state.apply(event).expect("event applies");
    }

    FeatureEngine::default()
        .snapshots(&state, now_ms)
        .into_iter()
        .find(|snapshot| snapshot.symbol == "@107")
        .expect("snapshot exists")
}
