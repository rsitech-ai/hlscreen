use hls_core::{
    confidence::DataConfidenceSnapshot,
    market_state::{
        AdverseSelectionProxy, FeatureSnapshot, LiquidityResilienceState, StalenessState,
        TradeabilityState,
    },
};
use hls_screen::{ScreenEngine, ScreenRequest, presets::builtin_presets};

#[test]
fn built_in_presets_match_expected_rows_and_order() {
    let presets = builtin_presets();
    assert_eq!(
        presets.iter().map(|preset| preset.name).collect::<Vec<_>>(),
        vec![
            "liquid_momentum",
            "volume_anomaly",
            "tight_spread_movers",
            "mean_reversion_watch",
            "thin_books",
            "liquidity_resilience",
            "brittle_tradeability",
            "flow_pressure",
        ]
    );

    let rows = preset_rows();
    let engine = ScreenEngine;

    assert_eq!(
        symbols(
            &engine
                .apply(&rows, &ScreenRequest::preset("liquid_momentum"))
                .expect("liquid momentum applies")
        ),
        vec!["FAST/USDC"]
    );
    assert_eq!(
        symbols(
            &engine
                .apply(&rows, &ScreenRequest::preset("volume_anomaly"))
                .expect("volume anomaly applies")
        ),
        vec!["VOLUME/USDC"]
    );
    assert_eq!(
        symbols(
            &engine
                .apply(&rows, &ScreenRequest::preset("tight_spread_movers"))
                .expect("tight spread movers applies")
        ),
        vec!["MOVER/USDC"]
    );
    assert_eq!(
        symbols(
            &engine
                .apply(&rows, &ScreenRequest::preset("mean_reversion_watch"))
                .expect("mean reversion applies")
        ),
        vec!["REVERT/USDC"]
    );
    assert_eq!(
        symbols(
            &engine
                .apply(&rows, &ScreenRequest::preset("thin_books"))
                .expect("thin books applies")
        ),
        vec!["THIN/USDC"]
    );
}

fn symbols(rows: &[FeatureSnapshot]) -> Vec<String> {
    rows.iter().map(|row| row.symbol.clone()).collect()
}

fn preset_rows() -> Vec<FeatureSnapshot> {
    vec![
        row(
            "FAST/USDC",
            10.0,
            10_000.0,
            0.005,
            3.0,
            1.0,
            82.0,
            90.0,
            40.0,
        ),
        row(
            "VOLUME/USDC",
            40.0,
            20_000.0,
            0.0,
            5.0,
            3.0,
            50.0,
            55.0,
            50.0,
        ),
        row(
            "MOVER/USDC",
            5.0,
            15_000.0,
            -0.02,
            0.0,
            0.0,
            50.0,
            40.0,
            60.0,
        ),
        row(
            "REVERT/USDC",
            50.0,
            12_000.0,
            -0.005,
            0.0,
            0.0,
            72.0,
            35.0,
            88.0,
        ),
        row("THIN/USDC", 80.0, 4_000.0, 0.0, 0.0, 0.0, 45.0, 45.0, 45.0),
    ]
}

#[allow(clippy::too_many_arguments)]
fn row(
    symbol: &str,
    spread_bps: f64,
    tob_depth_usd: f64,
    ret_5m: f64,
    volume_z_1h: f64,
    trade_count_z_1h: f64,
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
        spread_bps: Some(spread_bps),
        spread_shock_bps: None,
        spread_recovery_ms: None,
        resilience_state: LiquidityResilienceState::Unknown,
        tradeability_state: TradeabilityState::Unknown,
        adverse_selection_proxy: AdverseSelectionProxy::Unknown,
        signed_notional_flow_30s: None,
        bbo_ofi_proxy_30s: None,
        tob_depth_usd: Some(tob_depth_usd),
        tob_imbalance: Some(0.0),
        ret_1m: Some(ret_5m),
        ret_5m: Some(ret_5m),
        ret_1h: Some(ret_5m),
        rv_1m: Some(0.0),
        rv_5m: Some(0.0),
        rv_1h: Some(0.0),
        volume_z_1h: Some(volume_z_1h),
        trade_count_z_1h: Some(trade_count_z_1h),
        liquidity_score,
        momentum_score,
        mean_reversion_score,
        updated_ms_ago: Some(0),
        staleness_state: StalenessState::Fresh,
        incomplete_window_reason: None,
    }
}
