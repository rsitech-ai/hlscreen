use hls_core::{
    confidence::DataConfidenceSnapshot,
    health::HealthInputs,
    market_state::{
        AdverseSelectionProxy, FeatureSnapshot, LiquidityResilienceState, StalenessState,
        TradeabilityState,
    },
};
use hls_server::{
    ApiState, SharedApiState, handle_get, serve_shared_until_shutdown, serve_until_shutdown,
};
use serde_json::Value;
use tokio::{net::TcpListener, sync::oneshot};

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

#[test]
fn encoded_symbol_paths_round_trip_and_malformed_queries_return_400() {
    let mut rows = rows();
    rows.push(row("HYPÉ/USDC", 3.0));
    let state = ApiState::new(HealthInputs::healthy_fixture().snapshot(), rows);

    let response = handle_get("/symbol/HYP%C3%89%2FUSDC", "", &state).expect("symbol response");
    assert_eq!(response.status_code, 200);
    assert!(response.body.contains("HYPÉ/USDC"));

    let response = handle_get("/screen", "where=%ZZ", &state).expect("bad query response");
    assert_eq!(response.status_code, 400);
    assert!(response.body.contains("invalid percent escape"));

    let response = handle_get("/symbol/%FF", "", &state).expect("bad path response");
    assert_eq!(response.status_code, 400);
    assert!(response.body.contains("invalid UTF-8"));
}

#[tokio::test]
async fn long_running_server_serves_read_only_http_until_shutdown() {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind loopback listener");
    let address = listener.local_addr().expect("listener address");
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let server = tokio::spawn(serve_until_shutdown(
        listener,
        ApiState::new(HealthInputs::healthy_fixture().snapshot(), rows()),
        async move {
            let _ = shutdown_rx.await;
        },
    ));
    let client = reqwest::Client::new();

    let health: Value = client
        .get(format!("http://{address}/health"))
        .send()
        .await
        .expect("health request")
        .error_for_status()
        .expect("health status")
        .json()
        .await
        .expect("health json");
    assert_eq!(health["status"], "healthy");
    assert_eq!(health["read_only"], true);

    let screen: Value = client
        .get(format!(
            "http://{address}/screen?where=symbol%20%3D%3D%20%22AAA%2FUSDC%22&limit=1"
        ))
        .send()
        .await
        .expect("screen request")
        .error_for_status()
        .expect("screen status")
        .json()
        .await
        .expect("screen json");
    assert_eq!(screen["rows"][0]["symbol"], "AAA/USDC");

    let unsafe_route = client
        .get(format!("http://{address}/orders"))
        .send()
        .await
        .expect("orders request");
    assert_eq!(unsafe_route.status(), reqwest::StatusCode::NOT_FOUND);
    let unsafe_body = unsafe_route.text().await.expect("orders body");
    assert!(!unsafe_body.contains("wallet"));
    assert!(!unsafe_body.contains("order"));

    shutdown_tx.send(()).expect("send shutdown");
    server
        .await
        .expect("server task")
        .expect("server exits cleanly");
}

#[tokio::test]
async fn running_server_serves_replaced_market_state_without_restart() {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind loopback listener");
    let address = listener.local_addr().expect("listener address");
    let shared = SharedApiState::new(ApiState::new(
        HealthInputs::healthy_fixture().snapshot(),
        vec![row("AAA/USDC", 2.0)],
    ));
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let server = tokio::spawn(serve_shared_until_shutdown(
        listener,
        shared.clone(),
        async move {
            let _ = shutdown_rx.await;
        },
    ));
    let client = reqwest::Client::new();

    let first: Value = client
        .get(format!("http://{address}/screen?limit=1"))
        .send()
        .await
        .expect("first screen request")
        .error_for_status()
        .expect("first screen status")
        .json()
        .await
        .expect("first screen json");
    assert_eq!(first["rows"][0]["symbol"], "AAA/USDC");

    shared
        .replace(ApiState::new(
            HealthInputs::writer_lag_fixture().snapshot(),
            vec![row("BBB/USDC", 1.0)],
        ))
        .expect("replace API state");

    let second: Value = client
        .get(format!("http://{address}/screen?limit=1"))
        .send()
        .await
        .expect("second screen request")
        .error_for_status()
        .expect("second screen status")
        .json()
        .await
        .expect("second screen json");
    assert_eq!(second["rows"][0]["symbol"], "BBB/USDC");

    let health: Value = client
        .get(format!("http://{address}/health"))
        .send()
        .await
        .expect("health request")
        .error_for_status()
        .expect("health status")
        .json()
        .await
        .expect("health json");
    assert_eq!(health["status"], "degraded");
    assert_eq!(health["writer_backlog"], 250);

    shutdown_tx.send(()).expect("send shutdown");
    server
        .await
        .expect("server task")
        .expect("server exits cleanly");
}

fn rows() -> Vec<FeatureSnapshot> {
    vec![row("AAA/USDC", 2.0), row("BBB/USDC", 1.0)]
}

fn row(symbol: &str, price: f64) -> FeatureSnapshot {
    FeatureSnapshot {
        symbol: symbol.to_owned(),
        confidence: DataConfidenceSnapshot::new(symbol),
        price: Some(price),
        mid_px: Some(price),
        mark_px: Some(price),
        day_ntl_vlm: Some(1_000_000.0),
        bid_px: Some(price - 0.01),
        bid_sz: Some(10.0),
        ask_px: Some(price + 0.01),
        ask_sz: Some(10.0),
        spread_bps: Some(10.0),
        spread_shock_bps: None,
        spread_recovery_ms: None,
        resilience_state: LiquidityResilienceState::Unknown,
        tradeability_state: TradeabilityState::Unknown,
        fee_aware_tradeability: None,
        adverse_selection_proxy: AdverseSelectionProxy::Unknown,
        signed_notional_flow_30s: None,
        bbo_ofi_proxy_30s: None,
        microstructure_metrics: Vec::new(),
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
        score_breakdown: None,
        metadata: None,
        updated_ms_ago: Some(0),
        staleness_state: StalenessState::Fresh,
        incomplete_window_reason: None,
    }
}
