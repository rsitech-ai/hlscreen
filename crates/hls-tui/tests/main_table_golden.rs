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

    assert_eq!(
        table,
        "READ-ONLY Hyperliquid spot live screen\n\
         symbol        price      spread_bps  tob_depth_usd  ret_1m    liq_score  updated_ms  state\n\
         @107          35.2000    57.14       245.10         -         2.45       6000        fresh\n"
    );
}
