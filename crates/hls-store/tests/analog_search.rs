use hls_core::{
    confidence::DataConfidenceSnapshot,
    market_state::{
        AdverseSelectionProxy, FeatureSnapshot, LiquidityResilienceState, StalenessState,
        TradeabilityState,
    },
};
use hls_store::analog::{
    AnalogCandidate, AnalogIndex, AnalogSearchOptions, search_analogs, search_analogs_in_index,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct AnalogFixture {
    target: FixtureSnapshot,
    candidates: Vec<FixtureSnapshot>,
}

#[derive(Clone, Debug, Deserialize)]
struct FixtureSnapshot {
    symbol: String,
    snapshot_ts_ms: i64,
    spread_bps: Option<f64>,
    tob_imbalance: Option<f64>,
    signed_notional_flow_30s: Option<f64>,
    bbo_ofi_proxy_30s: Option<f64>,
    rv_5m: Option<f64>,
    liquidity_score: f64,
    momentum_score: f64,
}

#[test]
fn analog_search_returns_nearest_matches_with_driver_explanations() {
    let fixture: AnalogFixture = serde_json::from_str(include_str!(
        "../../../tests/fixtures/microstructure/analog_search_snapshots.json"
    ))
    .expect("analog fixture parses");

    let target = snapshot_from_fixture(&fixture.target);
    let candidates: Vec<AnalogCandidate> = fixture
        .candidates
        .iter()
        .map(|candidate| AnalogCandidate {
            symbol: candidate.symbol.clone(),
            snapshot_ts_ms: candidate.snapshot_ts_ms,
            snapshot: snapshot_from_fixture(candidate),
        })
        .collect();

    let report = search_analogs(
        Some("fixture"),
        &target,
        fixture.target.snapshot_ts_ms,
        &candidates,
        AnalogSearchOptions {
            limit: 2,
            min_candidates: 1,
        },
    );

    assert_eq!(report.run_id.as_deref(), Some("fixture"));
    assert_eq!(report.target_symbol, "HYPE/USDC");
    assert_eq!(report.matches.len(), 2);
    assert!(report.insufficient_evidence.is_none());
    assert_eq!(report.matches[0].symbol, "PURR/USDC");
    assert_eq!(report.matches[1].symbol, "FAR/USDC");
    assert!(report.matches[0].distance < report.matches[1].distance);
    assert!(!report.matches[0].drivers.is_empty());
    assert!(
        report.matches[0]
            .drivers
            .iter()
            .any(|driver| driver.field == "spread_bps" || driver.field == "tob_imbalance")
    );
}

#[test]
fn analog_search_reports_insufficient_evidence_without_fabricating_matches() {
    let target = FeatureSnapshot {
        symbol: "HYPE/USDC".to_owned(),
        spread_bps: Some(12.0),
        tob_imbalance: None,
        signed_notional_flow_30s: None,
        bbo_ofi_proxy_30s: None,
        rv_5m: None,
        liquidity_score: 0.0,
        momentum_score: 0.0,
        ..empty_snapshot("HYPE/USDC")
    };
    let candidate = AnalogCandidate {
        symbol: "PURR/USDC".to_owned(),
        snapshot_ts_ms: 1710000001000,
        snapshot: FeatureSnapshot {
            symbol: "PURR/USDC".to_owned(),
            spread_bps: Some(13.0),
            ..empty_snapshot("PURR/USDC")
        },
    };

    let report = search_analogs(
        Some("sparse"),
        &target,
        1710000003000,
        &[candidate],
        AnalogSearchOptions {
            limit: 5,
            min_candidates: 1,
        },
    );

    assert!(report.matches.is_empty());
    assert!(
        report
            .insufficient_evidence
            .as_deref()
            .expect("insufficient evidence reason")
            .contains("comparable")
    );
}

#[test]
fn analog_index_round_trips_replay_candidates() {
    let fixture: AnalogFixture = serde_json::from_str(include_str!(
        "../../../tests/fixtures/microstructure/analog_search_snapshots.json"
    ))
    .expect("analog fixture parses");
    let target = snapshot_from_fixture(&fixture.target);
    let candidates: Vec<AnalogCandidate> = fixture
        .candidates
        .iter()
        .map(|candidate| AnalogCandidate {
            symbol: candidate.symbol.clone(),
            snapshot_ts_ms: candidate.snapshot_ts_ms,
            snapshot: snapshot_from_fixture(candidate),
        })
        .collect();
    let index = AnalogIndex::new(
        "fixture-run",
        fixture.target.symbol.clone(),
        fixture.target.snapshot_ts_ms,
        target,
        candidates,
    );
    let temp = tempfile::tempdir().expect("tempdir");
    let path = temp.path().join("analog-index.json");

    index.write_json(&path).expect("write index");
    let loaded = AnalogIndex::read_json(&path).expect("read index");
    let report = search_analogs_in_index(
        &loaded,
        AnalogSearchOptions {
            limit: 2,
            min_candidates: 1,
        },
    )
    .expect("search index");

    assert_eq!(loaded.schema_version, 1);
    assert_eq!(loaded.source_run_id, "fixture-run");
    assert_eq!(report.run_id.as_deref(), Some("fixture-run"));
    assert_eq!(report.target_symbol, "HYPE/USDC");
    assert_eq!(report.matches.len(), 2);
    assert_eq!(report.matches[0].symbol, "PURR/USDC");
    assert!(report.insufficient_evidence.is_none());
}

fn snapshot_from_fixture(fixture: &FixtureSnapshot) -> FeatureSnapshot {
    FeatureSnapshot {
        symbol: fixture.symbol.clone(),
        spread_bps: fixture.spread_bps,
        tob_imbalance: fixture.tob_imbalance,
        signed_notional_flow_30s: fixture.signed_notional_flow_30s,
        bbo_ofi_proxy_30s: fixture.bbo_ofi_proxy_30s,
        rv_5m: fixture.rv_5m,
        liquidity_score: fixture.liquidity_score,
        momentum_score: fixture.momentum_score,
        ..empty_snapshot(&fixture.symbol)
    }
}

fn empty_snapshot(symbol: &str) -> FeatureSnapshot {
    FeatureSnapshot {
        symbol: symbol.to_owned(),
        confidence: DataConfidenceSnapshot::new(symbol),
        price: None,
        mid_px: None,
        mark_px: None,
        day_ntl_vlm: None,
        bid_px: None,
        bid_sz: None,
        ask_px: None,
        ask_sz: None,
        spread_bps: None,
        spread_shock_bps: None,
        spread_recovery_ms: None,
        resilience_state: LiquidityResilienceState::Unknown,
        tradeability_state: TradeabilityState::Unknown,
        fee_aware_tradeability: None,
        adverse_selection_proxy: AdverseSelectionProxy::Unknown,
        signed_notional_flow_30s: None,
        bbo_ofi_proxy_30s: None,
        microstructure_metrics: Vec::new(),
        tob_depth_usd: None,
        tob_imbalance: None,
        ret_1m: None,
        ret_5m: None,
        ret_1h: None,
        rv_1m: None,
        rv_5m: None,
        rv_1h: None,
        volume_z_1h: None,
        trade_count_z_1h: None,
        liquidity_score: 0.0,
        momentum_score: 0.0,
        mean_reversion_score: 0.0,
        score_breakdown: None,
        metadata: None,
        updated_ms_ago: None,
        staleness_state: StalenessState::Incomplete,
        incomplete_window_reason: None,
    }
}
