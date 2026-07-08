use hls_core::market_state::{AdverseSelectionProxy, LiquidityResilienceState, LiveMarketState};
use hls_features::engine::FeatureEngine;
use hls_hyperliquid::ws::parser::parse_ws_ndjson;

#[test]
fn spread_shock_fixture_reports_recovery_and_bbo_proxies() {
    let snapshot = snapshot_from_fixture(
        include_str!("../../../tests/fixtures/microstructure/resilience_shock.ndjson"),
        1_710_001_008_000,
    );

    assert_eq!(snapshot.resilience_state, LiquidityResilienceState::Normal);
    assert!(
        snapshot.spread_shock_bps.expect("shock magnitude") > 85.0,
        "spread shock should be measured versus the local pre-shock baseline"
    );
    assert_eq!(snapshot.spread_recovery_ms, Some(6_000));
    assert!(
        snapshot
            .bbo_ofi_proxy_30s
            .expect("BBO OFI proxy")
            .is_finite()
    );
    assert!(
        snapshot
            .signed_notional_flow_30s
            .expect("signed flow")
            .abs()
            > 500.0
    );
    assert_eq!(
        snapshot.adverse_selection_proxy,
        AdverseSelectionProxy::Normal
    );
}

#[test]
fn brittle_fixture_flags_unrecovered_spread_shock() {
    let snapshot = snapshot_from_fixture(
        include_str!("../../../tests/fixtures/microstructure/thin_brittle_book.ndjson"),
        1_710_002_014_000,
    );

    assert_eq!(snapshot.resilience_state, LiquidityResilienceState::Brittle);
    assert!(
        snapshot.spread_shock_bps.expect("shock magnitude") > 350.0,
        "wide top-of-book spread should exceed shock threshold"
    );
    assert_eq!(snapshot.spread_recovery_ms, None);
    assert_eq!(
        snapshot.adverse_selection_proxy,
        AdverseSelectionProxy::Brittle
    );
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
