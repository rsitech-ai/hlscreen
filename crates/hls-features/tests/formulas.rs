use hls_core::market_state::{FeatureSnapshot, LiveMarketState, StalenessState};
use hls_features::{
    engine::FeatureEngine,
    formulas::{
        bounded_score, percent_return, realized_volatility, spread_bps, tob_depth_usd,
        tob_imbalance, z_score,
    },
};
use hls_hyperliquid::ws::parser::parse_ws_ndjson;

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-6,
        "actual={actual} expected={expected}"
    );
}

fn assert_option_close(actual: Option<f64>, expected: f64) {
    assert_close(actual.expect("value exists"), expected);
}

#[test]
fn computes_top_of_book_and_return_formulas() {
    assert_close(
        spread_bps(34.90, 35.10).expect("spread"),
        57.142857142857146,
    );
    assert_close(tob_depth_usd(34.90, 3.0, 35.10, 4.0), 245.10);
    assert_close(
        tob_imbalance(34.90, 3.0, 35.10, 4.0).expect("imbalance"),
        -0.14565483476132195,
    );
    assert_close(
        percent_return(35.0, 35.2).expect("return"),
        0.005714285714285714,
    );
}

#[test]
fn computes_anomaly_and_bounded_score_helpers() {
    assert_close(z_score(120.0, 100.0, 10.0).expect("z"), 2.0);
    assert!(z_score(120.0, 100.0, 0.0).is_none());
    assert_close(
        realized_volatility(&[0.01, -0.02, 0.03]).expect("rv"),
        0.020548046676563253,
    );
    assert_close(bounded_score(125.0), 100.0);
    assert_close(bounded_score(-1.0), 0.0);
}

#[test]
fn fixture_events_produce_feature_snapshot_with_freshness_state() {
    let events = parse_ws_ndjson(include_str!(
        "../../../tests/fixtures/hyperliquid/ws_mock_live.ndjson"
    ))
    .expect("fixture parses");
    let mut state = LiveMarketState::new(["@107".to_owned()]);
    for event in events {
        state.apply(event).expect("event applies");
    }

    let engine = FeatureEngine::default();
    let snapshots = engine.snapshots(&state, 1_710_000_066_000);
    let snapshot = snapshots
        .iter()
        .find(|snapshot| snapshot.symbol == "@107")
        .expect("HYPE snapshot exists");

    let expected_shape = FeatureSnapshot {
        symbol: "@107".to_owned(),
        price: Some(35.20),
        mid_px: Some(35.00),
        mark_px: Some(35.50),
        day_ntl_vlm: Some(25_000_000.5),
        bid_px: Some(34.90),
        bid_sz: Some(3.0),
        ask_px: Some(35.10),
        ask_sz: Some(4.0),
        spread_bps: None,
        tob_depth_usd: None,
        tob_imbalance: None,
        ret_1m: None,
        ret_5m: None,
        ret_1h: None,
        rv_1m: Some(0.0),
        rv_5m: Some(0.0),
        rv_1h: Some(0.0),
        volume_z_1h: Some(0.0),
        trade_count_z_1h: Some(0.0),
        liquidity_score: 0.0,
        momentum_score: 0.0,
        mean_reversion_score: 0.0,
        updated_ms_ago: Some(6_000),
        staleness_state: StalenessState::Fresh,
        incomplete_window_reason: None,
    };
    assert_eq!(snapshot.symbol, expected_shape.symbol);
    assert_eq!(snapshot.price, expected_shape.price);
    assert_eq!(snapshot.mid_px, expected_shape.mid_px);
    assert_eq!(snapshot.mark_px, expected_shape.mark_px);
    assert_eq!(snapshot.day_ntl_vlm, expected_shape.day_ntl_vlm);
    assert_eq!(snapshot.bid_px, expected_shape.bid_px);
    assert_eq!(snapshot.bid_sz, expected_shape.bid_sz);
    assert_eq!(snapshot.ask_px, expected_shape.ask_px);
    assert_eq!(snapshot.ask_sz, expected_shape.ask_sz);
    assert_eq!(snapshot.rv_1m, expected_shape.rv_1m);
    assert_eq!(snapshot.rv_5m, expected_shape.rv_5m);
    assert_eq!(snapshot.rv_1h, expected_shape.rv_1h);
    assert_eq!(snapshot.volume_z_1h, expected_shape.volume_z_1h);
    assert_eq!(snapshot.trade_count_z_1h, expected_shape.trade_count_z_1h);
    assert_eq!(snapshot.updated_ms_ago, expected_shape.updated_ms_ago);
    assert_eq!(snapshot.staleness_state, expected_shape.staleness_state);
    assert_eq!(
        snapshot.incomplete_window_reason,
        expected_shape.incomplete_window_reason
    );
    assert_option_close(snapshot.spread_bps, 57.142857142857146);
    assert_option_close(snapshot.tob_depth_usd, 245.10);
    assert_option_close(snapshot.tob_imbalance, -0.14565483476132195);
    assert_option_close(snapshot.ret_1m, 0.005714285714285714);
    assert_option_close(snapshot.ret_5m, 0.005714285714285714);
    assert_option_close(snapshot.ret_1h, 0.005714285714285714);
    assert_close(snapshot.liquidity_score, 2.451);
    assert_close(snapshot.momentum_score, 50.57142857142857);
    assert_close(snapshot.mean_reversion_score, 49.42857142857143);
}
