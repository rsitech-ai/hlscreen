use hls_core::market_state::LiveMarketState;
use hls_features::engine::FeatureEngine;
use hls_hyperliquid::ws::parser::parse_ws_ndjson;
use hls_tui::app::render_main_table;

#[test]
fn renders_read_only_main_table_for_fixture_snapshot() {
    let events = parse_ws_ndjson(include_str!(
        "../../../tests/fixtures/hyperliquid/ws_mock_live.ndjson"
    ))
    .expect("fixture parses");
    let mut state = LiveMarketState::new(["@107".to_owned()]);
    for event in events {
        state.apply(event).expect("event applies");
    }
    let snapshots = FeatureEngine::default().snapshots(&state, 1_710_000_066_000);

    let table = render_main_table(&snapshots);

    assert!(table.contains("Hyperliquid Microstructure Workstation"));
    assert!(table.contains("PUBLIC WS/REST"));
    assert!(table.contains("QUALITY"));
    assert!(table.contains("median spread 57.1 bps"));
    assert!(table.contains("top depth $245"));
    assert!(table.contains("#   SYMBOL"));
    assert!(table.contains("@107"));
    assert!(table.contains("● FRESH"));
    assert!(table.contains("No wallet"));
    assert!(table.contains("Scores are screen heuristics, not orders or advice."));
}
