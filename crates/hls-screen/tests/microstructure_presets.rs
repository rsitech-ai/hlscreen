use hls_core::{
    confidence::DataConfidenceSnapshot,
    market_state::{
        AdverseSelectionProxy, FeatureSnapshot, LiquidityResilienceState, StalenessState,
        TradeabilityState,
    },
};
use hls_screen::{ScreenEngine, ScreenRequest, presets::builtin_presets};

#[test]
fn microstructure_presets_filter_resilience_and_tradeability_fields() {
    let presets = builtin_presets();
    let names = presets.iter().map(|preset| preset.name).collect::<Vec<_>>();
    assert!(names.contains(&"liquidity_resilience"));
    assert!(names.contains(&"brittle_tradeability"));
    assert!(names.contains(&"flow_pressure"));

    let rows = rows();
    let engine = ScreenEngine;

    assert_eq!(
        symbols(
            &engine
                .apply(&rows, &ScreenRequest::preset("liquidity_resilience"))
                .expect("liquidity resilience applies")
        ),
        vec!["RESILIENT/USDC"]
    );
    assert_eq!(
        symbols(
            &engine
                .apply(&rows, &ScreenRequest::preset("brittle_tradeability"))
                .expect("brittle tradeability applies")
        ),
        vec!["BRITTLE/USDC"]
    );
    assert_eq!(
        symbols(
            &engine
                .apply(&rows, &ScreenRequest::preset("flow_pressure"))
                .expect("flow pressure applies")
        ),
        vec!["FLOW/USDC"]
    );
}

fn symbols(rows: &[FeatureSnapshot]) -> Vec<String> {
    rows.iter().map(|row| row.symbol.clone()).collect()
}

fn rows() -> Vec<FeatureSnapshot> {
    vec![
        row(
            "RESILIENT/USDC",
            LiquidityResilienceState::Normal,
            TradeabilityState::Tradeable,
            AdverseSelectionProxy::Normal,
            Some(0.0),
            Some(30_000.0),
        ),
        row(
            "BRITTLE/USDC",
            LiquidityResilienceState::Brittle,
            TradeabilityState::Thin,
            AdverseSelectionProxy::Brittle,
            Some(380.0),
            Some(100.0),
        ),
        row(
            "FLOW/USDC",
            LiquidityResilienceState::Normal,
            TradeabilityState::Costly,
            AdverseSelectionProxy::Watch,
            Some(15.0),
            Some(25_000.0),
        )
        .with_flow(Some(12_500.0), Some(8_000.0)),
    ]
}

fn row(
    symbol: &str,
    resilience_state: LiquidityResilienceState,
    tradeability_state: TradeabilityState,
    adverse_selection_proxy: AdverseSelectionProxy,
    spread_shock_bps: Option<f64>,
    tob_depth_usd: Option<f64>,
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
        spread_bps: Some(12.0),
        spread_shock_bps,
        spread_recovery_ms: Some(1_000),
        resilience_state,
        tradeability_state,
        adverse_selection_proxy,
        signed_notional_flow_30s: Some(0.0),
        bbo_ofi_proxy_30s: Some(0.0),
        tob_depth_usd,
        tob_imbalance: Some(0.0),
        ret_1m: Some(0.0),
        ret_5m: Some(0.0),
        ret_1h: Some(0.0),
        rv_1m: Some(0.0),
        rv_5m: Some(0.0),
        rv_1h: Some(0.0),
        volume_z_1h: Some(0.0),
        trade_count_z_1h: Some(0.0),
        liquidity_score: 75.0,
        momentum_score: 50.0,
        mean_reversion_score: 50.0,
        score_breakdown: None,
        metadata: None,
        updated_ms_ago: Some(0),
        staleness_state: StalenessState::Fresh,
        incomplete_window_reason: None,
    }
}

trait WithFlow {
    fn with_flow(
        self,
        signed_notional_flow_30s: Option<f64>,
        bbo_ofi_proxy_30s: Option<f64>,
    ) -> Self;
}

impl WithFlow for FeatureSnapshot {
    fn with_flow(
        mut self,
        signed_notional_flow_30s: Option<f64>,
        bbo_ofi_proxy_30s: Option<f64>,
    ) -> Self {
        self.signed_notional_flow_30s = signed_notional_flow_30s;
        self.bbo_ofi_proxy_30s = bbo_ofi_proxy_30s;
        self
    }
}
