use hls_core::{
    confidence::DataConfidenceSnapshot,
    market_state::{
        AdverseSelectionProxy, FeatureSnapshot, LiquidityResilienceState, StalenessState,
        TradeabilityState,
    },
    metadata::{MetadataEnrichment, MetadataEnrichmentInput},
};
use hls_screen::{ScreenEngine, ScreenRequest};

#[test]
fn metadata_presets_filter_and_sort_cohort_tags() {
    let rows = vec![
        row(
            "@107",
            Some(metadata(
                "@107",
                "HYPE/USDC",
                107,
                Some(1_709_400_000_000),
                Some(1_250_000.0),
                Some(1_000_000_000.0),
                Some(100_000_000.0),
            )),
        ),
        row(
            "@404",
            Some(metadata(
                "@404",
                "PARTIAL/USDC",
                404,
                None,
                None,
                None,
                None,
            )),
        ),
        row("@999", None),
    ];
    let engine = ScreenEngine;

    assert_eq!(
        symbols(
            &engine
                .apply(&rows, &ScreenRequest::preset("new_listings"))
                .expect("new listing preset")
        ),
        vec!["@107"]
    );
    assert_eq!(
        symbols(
            &engine
                .apply(&rows, &ScreenRequest::preset("fresh_liquidity"))
                .expect("fresh liquidity preset")
        ),
        vec!["@107"]
    );
    assert_eq!(
        symbols(
            &engine
                .apply(&rows, &ScreenRequest::preset("metadata_unknown"))
                .expect("unknown metadata preset")
        ),
        vec!["@404", "@999"]
    );

    let custom = engine
        .apply(
            &rows,
            &ScreenRequest {
                where_expr: Some(
                    "cohort_tag == \"unknown_metadata\" or seeded_usdc > 1000000".to_owned(),
                ),
                sort: Some("symbol:asc".to_owned()),
                ..ScreenRequest::default()
            },
        )
        .expect("custom metadata rule");
    assert_eq!(symbols(&custom), vec!["@107", "@404", "@999"]);
}

fn symbols(rows: &[FeatureSnapshot]) -> Vec<String> {
    rows.iter().map(|row| row.symbol.clone()).collect()
}

fn metadata(
    symbol: &str,
    display_name: &str,
    spot_index: u32,
    deploy_time_ms: Option<i64>,
    seeded_usdc: Option<f64>,
    max_supply: Option<f64>,
    circulating_supply: Option<f64>,
) -> MetadataEnrichment {
    MetadataEnrichment::from_public_input(MetadataEnrichmentInput {
        symbol: symbol.to_owned(),
        display_name: display_name.to_owned(),
        feed_identifier: symbol.to_owned(),
        spot_index,
        base_token_index: spot_index,
        quote_token_index: 0,
        metadata_source: "spotMetaAndAssetCtxs+tokenDetails".to_owned(),
        metadata_fetched_at_ms: 1_710_000_100_000,
        deploy_time_ms,
        deployer: deploy_time_ms
            .is_some()
            .then(|| "0x1234567890abcdef1234567890abcdef12345678".to_owned()),
        seeded_usdc,
        max_supply,
        circulating_supply,
        now_ms: 1_710_000_100_000,
    })
}

fn row(symbol: &str, metadata: Option<MetadataEnrichment>) -> FeatureSnapshot {
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
        metadata,
        updated_ms_ago: Some(0),
        staleness_state: StalenessState::Fresh,
        incomplete_window_reason: None,
    }
}
