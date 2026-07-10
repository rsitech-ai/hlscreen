use hls_core::{
    market_state::{LiveMarketState, MarketEvent, TopOfBookEvent, TradeEvent, TradeSide},
    metrics::MetricSupport,
};
use hls_features::engine::FeatureEngine;

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-9,
        "actual={actual} expected={expected}"
    );
}

#[test]
fn public_trade_metric_formulas_are_exposed_as_research_proxies() {
    let now_ms = 1_000_000;
    let mut state = LiveMarketState::new(["@107".to_owned()]);
    for event in [
        trade(now_ms - 50_000, 100.0, 1),
        trade(now_ms - 40_000, 101.0, 2),
        trade(now_ms - 30_000, 99.0, 3),
        trade(now_ms - 20_000, 100.5, 4),
        trade(now_ms - 10_000, 100.0, 5),
        bbo(now_ms - 3_000, 99.9, 10.0, 100.1, 12.0),
        bbo(now_ms - 2_000, 100.0, 11.0, 100.2, 9.0),
    ] {
        state.apply(event).expect("event applies");
    }

    let snapshot = FeatureEngine::default()
        .snapshots(&state, now_ms)
        .into_iter()
        .find(|snapshot| snapshot.symbol == "@107")
        .expect("snapshot exists");

    let amihud = metric(&snapshot, "amihud_1m");
    assert_eq!(amihud.support, MetricSupport::Proxy);
    assert!(
        amihud
            .reason
            .as_deref()
            .unwrap_or_default()
            .contains("proxy")
    );
    assert_close(
        amihud.value.expect("amihud value"),
        0.0 / (100.0 + 101.0 + 99.0 + 100.5 + 100.0),
    );

    let roll = metric(&snapshot, "roll_effective_spread");
    assert_eq!(roll.support, MetricSupport::Proxy);
    assert!(roll.reason.as_deref().unwrap_or_default().contains("proxy"));
    assert_close(roll.value.expect("roll value"), 2.7688746209726918);

    let bipower = metric(&snapshot, "bipower_variation_5m");
    assert_eq!(bipower.support, MetricSupport::Proxy);
    assert!(
        bipower
            .reason
            .as_deref()
            .unwrap_or_default()
            .contains("proxy")
    );
    assert!(bipower.value.expect("bipower value") > 0.0);

    let ofi = metric(&snapshot, "bbo_ofi_proxy_30s");
    assert_eq!(ofi.support, MetricSupport::Proxy);
    assert!(
        ofi.reason
            .as_deref()
            .unwrap_or_default()
            .contains("top-of-book")
    );
    assert_close(ofi.value.expect("ofi value"), 2_301.2);

    let toxicity = metric(&snapshot, "adverse_selection_toxicity_proxy");
    assert_eq!(toxicity.support, MetricSupport::Proxy);
    assert!(toxicity.value.expect("toxicity proxy value") >= 0.0);
}

#[test]
fn signed_flow_toxicity_proxy_uses_public_trade_imbalance() {
    let now_ms = 1_000_000;
    let mut state = LiveMarketState::new(["@107".to_owned()]);
    for event in [
        trade_with_side(now_ms - 20_000, 100.0, 1, TradeSide::Buy),
        trade_with_side(now_ms - 10_000, 100.0, 2, TradeSide::Buy),
        trade_with_side(now_ms - 5_000, 100.0, 3, TradeSide::Sell),
    ] {
        state.apply(event).expect("event applies");
    }

    let snapshot = FeatureEngine::default()
        .snapshots(&state, now_ms)
        .into_iter()
        .find(|snapshot| snapshot.symbol == "@107")
        .expect("snapshot exists");

    let toxicity = metric(&snapshot, "signed_flow_toxicity_proxy_30s");
    assert_eq!(toxicity.support, MetricSupport::Proxy);
    assert_close(toxicity.value.expect("toxicity proxy value"), 1.0 / 3.0);
    assert!(
        toxicity
            .reason
            .as_deref()
            .unwrap_or_default()
            .contains("public trade")
    );
}

#[test]
fn research_metrics_expose_unavailable_states_instead_of_fake_values() {
    let now_ms = 1_000_000;
    let mut state = LiveMarketState::new(["@107".to_owned()]);
    for event in [
        trade(now_ms - 50_000, 100.0, 1),
        trade(now_ms - 40_000, 101.0, 2),
        trade(now_ms - 30_000, 102.0, 3),
        trade(now_ms - 20_000, 103.0, 4),
    ] {
        state.apply(event).expect("event applies");
    }

    let snapshot = FeatureEngine::default()
        .snapshots(&state, now_ms)
        .into_iter()
        .find(|snapshot| snapshot.symbol == "@107")
        .expect("snapshot exists");

    let roll = metric(&snapshot, "roll_effective_spread");
    assert_eq!(roll.support, MetricSupport::Unavailable);
    assert!(roll.value.is_none());
    assert!(
        roll.reason
            .as_deref()
            .unwrap_or_default()
            .contains("non-negative")
    );

    let ofi = metric(&snapshot, "bbo_ofi_proxy_30s");
    assert_eq!(ofi.support, MetricSupport::Unavailable);
    assert!(ofi.value.is_none());
}

#[test]
fn signed_flow_toxicity_proxy_is_unavailable_for_sparse_public_trades() {
    let now_ms = 1_000_000;
    let mut state = LiveMarketState::new(["@107".to_owned()]);
    state
        .apply(trade_with_side(now_ms - 10_000, 100.0, 1, TradeSide::Buy))
        .expect("event applies");

    let snapshot = FeatureEngine::default()
        .snapshots(&state, now_ms)
        .into_iter()
        .find(|snapshot| snapshot.symbol == "@107")
        .expect("snapshot exists");

    let toxicity = metric(&snapshot, "signed_flow_toxicity_proxy_30s");
    assert_eq!(toxicity.support, MetricSupport::Unavailable);
    assert!(toxicity.value.is_none());
    assert!(
        toxicity
            .reason
            .as_deref()
            .unwrap_or_default()
            .contains("30s public trade")
    );
}

fn metric<'a>(
    snapshot: &'a hls_core::market_state::FeatureSnapshot,
    name: &str,
) -> &'a hls_core::metrics::MicrostructureMetricSnapshot {
    snapshot
        .microstructure_metrics
        .iter()
        .find(|metric| metric.name == name)
        .unwrap_or_else(|| panic!("metric {name} exists"))
}

fn trade(exchange_ts_ms: i64, price: f64, tid: u64) -> MarketEvent {
    let side = if tid % 2 == 0 {
        TradeSide::Buy
    } else {
        TradeSide::Sell
    };
    trade_with_side(exchange_ts_ms, price, tid, side)
}

fn trade_with_side(exchange_ts_ms: i64, price: f64, tid: u64, side: TradeSide) -> MarketEvent {
    MarketEvent::Trade(TradeEvent {
        recv_ts_ns: exchange_ts_ms as u64 * 1_000_000,
        exchange_ts_ms,
        hl_coin: "@107".to_owned(),
        side,
        price,
        size: 1.0,
        notional: price,
        hash: format!("0x{tid:x}"),
        tid,
        unique_trade_id: format!("@107:{exchange_ts_ms}:{tid}"),
    })
}

fn bbo(
    exchange_ts_ms: i64,
    bid_price: f64,
    bid_size: f64,
    ask_price: f64,
    ask_size: f64,
) -> MarketEvent {
    MarketEvent::TopOfBook(TopOfBookEvent {
        recv_ts_ns: exchange_ts_ms as u64 * 1_000_000,
        exchange_ts_ms,
        hl_coin: "@107".to_owned(),
        bid_price: Some(bid_price),
        bid_size: Some(bid_size),
        bid_order_count: Some(1),
        ask_price: Some(ask_price),
        ask_size: Some(ask_size),
        ask_order_count: Some(1),
    })
}
