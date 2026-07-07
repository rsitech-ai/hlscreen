use hls_core::{
    health::HealthInputs,
    market_state::{FeatureSnapshot, StalenessState},
};
use hls_server::{ApiState, handle_get};

#[test]
fn health_endpoint_returns_read_only_status_without_action_surfaces() {
    let response = handle_get(
        "/health",
        "",
        &ApiState::new(HealthInputs::healthy_fixture().snapshot(), rows()),
    )
    .expect("health response");

    assert_eq!(response.status_code, 200);
    assert!(response.body.contains(r#""read_only":true"#));
    assert!(response.body.contains(r#""status":"healthy"#));
    assert!(!response.body.contains("wallet"));
    assert!(!response.body.contains("order"));
}

#[test]
fn screen_endpoint_applies_filter_and_rejects_invalid_rules() {
    let state = ApiState::new(HealthInputs::healthy_fixture().snapshot(), rows());
    let response = handle_get(
        "/screen",
        "where=symbol%20%3D%3D%20%22AAA%2FUSDC%22&sort=price%3Adesc",
        &state,
    )
    .expect("screen response");
    assert_eq!(response.status_code, 200);
    assert!(response.body.contains("AAA/USDC"));
    assert!(!response.body.contains("BBB/USDC"));

    let response =
        handle_get("/screen", "where=symbol%20%3E%2010", &state).expect("validation response");
    assert_eq!(response.status_code, 400);
    assert!(response.body.contains("type-incompatible comparison"));
}

#[test]
fn unsafe_or_unknown_routes_are_not_exposed() {
    let response = handle_get(
        "/orders",
        "",
        &ApiState::new(HealthInputs::healthy_fixture().snapshot(), rows()),
    )
    .expect("not found response");

    assert_eq!(response.status_code, 404);
    assert!(!response.body.contains("private"));
}

fn rows() -> Vec<FeatureSnapshot> {
    vec![row("AAA/USDC", 2.0), row("BBB/USDC", 1.0)]
}

fn row(symbol: &str, price: f64) -> FeatureSnapshot {
    FeatureSnapshot {
        symbol: symbol.to_owned(),
        price: Some(price),
        mid_px: Some(price),
        mark_px: Some(price),
        day_ntl_vlm: Some(1_000_000.0),
        bid_px: Some(price - 0.01),
        bid_sz: Some(10.0),
        ask_px: Some(price + 0.01),
        ask_sz: Some(10.0),
        spread_bps: Some(10.0),
        tob_depth_usd: Some(1_000.0),
        tob_imbalance: Some(0.0),
        ret_1m: Some(0.0),
        ret_5m: Some(0.0),
        ret_1h: Some(0.0),
        rv_1m: Some(0.0),
        rv_5m: Some(0.0),
        rv_1h: Some(0.0),
        volume_z_1h: Some(0.0),
        trade_count_z_1h: Some(0.0),
        liquidity_score: 50.0,
        momentum_score: 50.0,
        mean_reversion_score: 50.0,
        updated_ms_ago: Some(0),
        staleness_state: StalenessState::Fresh,
        incomplete_window_reason: None,
    }
}
