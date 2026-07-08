use hls_core::market_state::LiveMarketState;
use hls_features::engine::FeatureEngine;
use hls_hyperliquid::ws::parser::parse_ws_ndjson;
use hls_tui::app::render_main_table;

#[test]
fn renders_degraded_confidence_in_market_board_and_detail_pane() {
    let events = parse_ws_ndjson(include_str!(
        "../../../tests/fixtures/microstructure/sparse_trades.ndjson"
    ))
    .expect("fixture parses");
    let mut state = LiveMarketState::new(["@107".to_owned()]);
    for event in events {
        state.apply(event).expect("event applies");
    }
    let snapshots = FeatureEngine::default().snapshots(&state, 1_710_000_002_000);

    let table = render_main_table(&snapshots);

    assert!(table.contains("Hyperliquid Spot Microstructure Workstation"));
    assert!(table.contains("conf"));
    assert!(table.contains("0.60"));
    assert!(table.contains("low confidence"));
    assert!(table.contains("Confidence     gap:3 stale:0 sparse:1 reconnect:0 parser_drop:0"));
    assert!(table.contains("missing:return_window"));
}
