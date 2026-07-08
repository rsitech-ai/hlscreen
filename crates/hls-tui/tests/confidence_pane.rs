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

    assert!(table.contains("CONFIDENCE"));
    assert!(table.contains("high 0 | medium 0 | low 1 | untrusted 0"));
    assert!(table.contains("L060"));
    assert!(table.contains("low confidence"));
    assert!(table.contains("confidence | low 60"));
    assert!(table.contains("reasons sparse_trades,incomplete_window"));
    assert!(
        table.contains("incomplete windows returns,realized_volatility,microstructure_windows")
    );
}
