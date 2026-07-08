use hls_core::market_state::LiveMarketState;
use hls_features::engine::FeatureEngine;
use hls_hyperliquid::ws::parser::parse_ws_ndjson;
use hls_tui::detail::render_why_ranked_pane;

#[test]
fn renders_why_ranked_pane_with_components_and_caveats() {
    let events = parse_ws_ndjson(include_str!(
        "../../../tests/fixtures/microstructure/resilience_shock.ndjson"
    ))
    .expect("fixture parses");
    let mut state = LiveMarketState::new(["@107".to_owned()]);
    for event in events {
        state.apply(event).expect("event applies");
    }
    let snapshots = FeatureEngine::default().snapshots(&state, 1_710_001_008_000);
    let snapshot = snapshots
        .iter()
        .find(|snapshot| snapshot.symbol == "@107")
        .expect("snapshot exists");

    let pane = render_why_ranked_pane(snapshot);

    assert!(pane.contains("WHY RANKED"));
    assert!(pane.contains("@107 score explanation"));
    assert!(pane.contains("adjusted"));
    assert!(pane.contains("confidence 100"));
    assert!(pane.contains("COMPONENTS"));
    assert!(pane.contains("liquidity_resilience"));
    assert!(pane.contains("spread_cost"));
    assert!(pane.contains("signed_flow"));
    assert!(pane.contains("negative"));
    assert!(pane.contains("unavailable evidence"));
    assert!(pane.contains("BBO/top-of-book proxy only"));
    assert!(pane.contains("screen heuristic, not advice"));
}
