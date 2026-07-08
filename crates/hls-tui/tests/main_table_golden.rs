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
         scope: public spot market data only | rows=1 fresh=1 stale=0 incomplete=0\n\
         symbol        price       spread   TOB depth      ret 1m   score   age ms   state\n\
         ------------  ----------  -------  -------------  -------  ------  -------  ----------\n\
         @107             35.2000     57.1            245        -    2.45     6000  fresh\n"
    );
}
