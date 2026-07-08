use hls_core::{
    confidence::DataConfidenceSnapshot,
    market_state::{
        AdverseSelectionProxy, FeatureSnapshot, LiquidityResilienceState, StalenessState,
        TradeabilityState,
    },
    score::{ScoreBreakdown, ScoreComponent, ScoreComponentKind},
};
use hls_screen::{ScreenEngine, ScreenRequest, ScreenSession};

#[test]
fn evaluator_filters_sorts_and_treats_missing_numeric_values_as_non_matches() {
    let rows = fixture_rows();
    let request = ScreenRequest {
        where_expr: Some("liquidity_score > 60 and spread_bps < 30".to_owned()),
        sort: Some("momentum_score:desc".to_owned()),
        ..ScreenRequest::default()
    };

    let visible = ScreenEngine.apply(&rows, &request).expect("screen applies");

    assert_eq!(symbols(&visible), vec!["AAA/USDC", "CCC/USDC"]);
}

#[test]
fn invalid_expression_does_not_replace_active_session_rows() {
    let rows = fixture_rows();
    let mut session = ScreenSession::default();
    let active = session
        .apply(
            &rows,
            &ScreenRequest {
                where_expr: Some("liquidity_score > 60 and spread_bps < 30".to_owned()),
                sort: Some("liquidity_score:desc".to_owned()),
                ..ScreenRequest::default()
            },
        )
        .expect("valid request applies")
        .to_vec();

    assert_eq!(symbols(&active), vec!["AAA/USDC", "CCC/USDC"]);

    let err = session
        .apply(
            &rows,
            &ScreenRequest {
                where_expr: Some("liquidity_score >".to_owned()),
                ..ScreenRequest::default()
            },
        )
        .expect_err("invalid request rejected");

    assert!(err.to_string().contains("expected value"));
    assert_eq!(symbols(session.active_rows()), vec!["AAA/USDC", "CCC/USDC"]);
}

#[test]
fn type_incompatible_comparisons_are_rejected() {
    let rows = fixture_rows();
    let err = ScreenEngine
        .apply(
            &rows,
            &ScreenRequest {
                where_expr: Some("symbol > 10".to_owned()),
                ..ScreenRequest::default()
            },
        )
        .expect_err("type mismatch rejected");

    assert!(err.to_string().contains("type-incompatible comparison"));
}

#[test]
fn score_fields_filter_sort_and_keep_missing_components_out() {
    let mut rows = fixture_rows();
    rows[0].score_breakdown = Some(ScoreBreakdown::from_components(
        "AAA/USDC",
        80,
        vec![
            ScoreComponent::new("momentum", ScoreComponentKind::Momentum, 40.0),
            ScoreComponent::new("spread_cost", ScoreComponentKind::SpreadCost, -5.0),
        ],
    ));
    rows[1].score_breakdown = Some(ScoreBreakdown::from_components(
        "BBB/USDC",
        100,
        vec![ScoreComponent::new(
            "momentum",
            ScoreComponentKind::Momentum,
            20.0,
        )],
    ));

    let visible = ScreenEngine
        .apply(
            &rows,
            &ScreenRequest {
                where_expr: Some("score_component.momentum >= 20 and score_total > 15".to_owned()),
                sort: Some("score_total:desc".to_owned()),
                ..ScreenRequest::default()
            },
        )
        .expect("score fields apply");

    assert_eq!(symbols(&visible), vec!["AAA/USDC", "BBB/USDC"]);

    let spread_cost_rows = ScreenEngine
        .apply(
            &rows,
            &ScreenRequest {
                where_expr: Some("score_component.spread_cost < 0".to_owned()),
                ..ScreenRequest::default()
            },
        )
        .expect("component filter applies");

    assert_eq!(symbols(&spread_cost_rows), vec!["AAA/USDC"]);
}

fn symbols(rows: &[FeatureSnapshot]) -> Vec<String> {
    rows.iter().map(|row| row.symbol.clone()).collect()
}

fn fixture_rows() -> Vec<FeatureSnapshot> {
    vec![
        row("AAA/USDC", Some(10.0), Some(12.0), 72.0, 83.0, 40.0),
        row("BBB/USDC", Some(50.0), Some(8.0), 88.0, 20.0, 65.0),
        row("CCC/USDC", Some(5.0), Some(-15.0), 65.0, 77.0, 75.0),
        row("MISSING/USDC", None, Some(20.0), 95.0, 90.0, 90.0),
    ]
}

fn row(
    symbol: &str,
    spread_bps: Option<f64>,
    ret_5m: Option<f64>,
    liquidity_score: f64,
    momentum_score: f64,
    mean_reversion_score: f64,
) -> FeatureSnapshot {
    FeatureSnapshot {
        symbol: symbol.to_owned(),
        confidence: DataConfidenceSnapshot::new(symbol),
        price: Some(1.0),
        mid_px: Some(1.0),
        mark_px: Some(1.0),
        day_ntl_vlm: Some(1_000_000.0),
        bid_px: Some(0.99),
        bid_sz: Some(10.0),
        ask_px: Some(1.01),
        ask_sz: Some(10.0),
        spread_bps,
        spread_shock_bps: None,
        spread_recovery_ms: None,
        resilience_state: LiquidityResilienceState::Unknown,
        tradeability_state: TradeabilityState::Unknown,
        adverse_selection_proxy: AdverseSelectionProxy::Unknown,
        signed_notional_flow_30s: None,
        bbo_ofi_proxy_30s: None,
        tob_depth_usd: Some(1_000.0),
        tob_imbalance: Some(0.0),
        ret_1m: Some(0.0),
        ret_5m,
        ret_1h: Some(0.0),
        rv_1m: Some(0.0),
        rv_5m: Some(0.0),
        rv_1h: Some(0.0),
        volume_z_1h: Some(0.0),
        trade_count_z_1h: Some(0.0),
        liquidity_score,
        momentum_score,
        mean_reversion_score,
        score_breakdown: None,
        metadata: None,
        updated_ms_ago: Some(0),
        staleness_state: StalenessState::Fresh,
        incomplete_window_reason: None,
    }
}
