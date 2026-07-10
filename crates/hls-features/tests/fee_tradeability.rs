use hls_core::{
    fees::FeeProfile,
    market_state::{LiveMarketState, TradeabilityState},
};
use hls_features::engine::FeatureEngine;
use hls_hyperliquid::ws::parser::parse_ws_ndjson;

#[test]
fn feature_engine_leaves_fee_aware_tradeability_absent_without_profile() {
    let snapshot = snapshot_with_engine(FeatureEngine::default());

    assert!(snapshot.fee_aware_tradeability.is_none());
    assert_eq!(snapshot.tradeability_state, TradeabilityState::Tradeable);
}

#[test]
fn low_fee_profile_preserves_tradeable_state_with_explicit_cost() {
    let profile = FeeProfile::new_hundredths_bps("manual-low-fee", 0, 10, 25, 2_000, 5_000)
        .expect("valid low fee profile");

    let snapshot = snapshot_with_engine(FeatureEngine::default().with_fee_profile(profile));
    let fee = snapshot
        .fee_aware_tradeability
        .expect("fee-aware evidence exists");

    assert_eq!(snapshot.tradeability_state, TradeabilityState::Tradeable);
    assert_eq!(fee.profile_name, "manual-low-fee");
    assert_eq!(fee.state, TradeabilityState::Tradeable);
    assert!(fee.expected_round_trip_cost_bps < 20.0);
    assert_eq!(fee.reason, "within_tradeable_fee_threshold");
}

#[test]
fn high_taker_fee_profile_marks_otherwise_tradeable_row_costly() {
    let profile = FeeProfile::new_hundredths_bps("manual-high-fee", 0, 2_500, 100, 2_000, 5_000)
        .expect("valid high fee profile");

    let snapshot = snapshot_with_engine(FeatureEngine::default().with_fee_profile(profile));
    let fee = snapshot
        .fee_aware_tradeability
        .expect("fee-aware evidence exists");

    assert_eq!(snapshot.tradeability_state, TradeabilityState::Tradeable);
    assert_eq!(fee.state, TradeabilityState::Costly);
    assert!(fee.expected_round_trip_cost_bps > 50.0);
    assert_eq!(fee.reason, "fee_cost_exceeds_tradeable_threshold");
}

#[test]
fn blended_fee_profile_uses_maker_taker_fill_mix() {
    let all_taker = FeeProfile::new_hundredths_bps("manual-all-taker", 0, 2_000, 0, 6_000, 8_000)
        .expect("valid all-taker profile");
    let blended = FeeProfile::new_hundredths_bps("manual-blended", 0, 2_000, 0, 6_000, 8_000)
        .expect("valid blended profile")
        .with_taker_fill_ratio_hundredths(2_500)
        .expect("valid fill mix");

    let all_taker_fee = snapshot_with_engine(FeatureEngine::default().with_fee_profile(all_taker))
        .fee_aware_tradeability
        .expect("all-taker fee evidence exists");
    let blended_fee = snapshot_with_engine(FeatureEngine::default().with_fee_profile(blended))
        .fee_aware_tradeability
        .expect("blended fee evidence exists");

    assert!(blended_fee.expected_round_trip_cost_bps < all_taker_fee.expected_round_trip_cost_bps);
    assert_eq!(blended_fee.maker_fee_bps, 0.0);
    assert_eq!(blended_fee.taker_fee_bps, 20.0);
    assert_eq!(blended_fee.taker_fill_ratio, 0.25);
    assert_eq!(blended_fee.reason, "within_tradeable_fee_threshold");
}

fn snapshot_with_engine(engine: FeatureEngine) -> hls_core::market_state::FeatureSnapshot {
    let events = parse_ws_ndjson(include_str!(
        "../../../tests/fixtures/microstructure/resilience_shock.ndjson"
    ))
    .expect("fixture parses");
    let mut state = LiveMarketState::new(["@107".to_owned()]);
    for event in events {
        state.apply(event).expect("event applies");
    }

    engine
        .snapshots(&state, 1_710_001_008_000)
        .into_iter()
        .find(|snapshot| snapshot.symbol == "@107")
        .expect("snapshot exists")
}
