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
    assert!(table.contains("SESSION"));
    assert!(table.contains("LATENCY"));
    assert!(table.contains("QUALITY"));
    assert!(table.contains("CONFIDENCE"));
    assert!(table.contains("RESILIENCE"));
    assert!(table.contains("high 1 | medium 0 | low 0 | untrusted 0"));
    assert!(table.contains("spread med 57.1 bps"));
    assert!(table.contains("depth top $245"));
    assert!(table.contains("#  SYMBOL"));
    assert!(table.contains("CONF"));
    assert!(table.contains("TRAD"));
    assert!(table.contains("RESIL"));
    assert!(table.contains("H100"));
    assert!(table.contains("OBSERVATION"));
    assert!(table.contains("@107"));
    assert!(table.contains("● fresh"));
    assert!(table.contains("thin book"));
    assert!(table.contains("wide spread"));
    assert!(table.contains("SELECTED SYMBOL"));
    assert!(table.contains("bid 34.9000 x 3.0000"));
    assert!(table.contains("ask 35.1000 x 4.0000"));
    assert!(table.contains("confidence | high 100 | reasons none"));
    assert!(table.contains("No wallet"));
    assert!(table.contains("Scores are screen heuristics, not orders or advice."));
}

#[test]
fn renders_resilience_and_tradeability_in_market_board_and_detail_pane() {
    let events = parse_ws_ndjson(include_str!(
        "../../../tests/fixtures/microstructure/resilience_shock.ndjson"
    ))
    .expect("fixture parses");
    let mut state = LiveMarketState::new(["@107".to_owned()]);
    for event in events {
        state.apply(event).expect("event applies");
    }
    let snapshots = FeatureEngine::default().snapshots(&state, 1_710_001_008_000);

    let table = render_main_table(&snapshots);

    assert!(table.contains("tradeable 1 | costly 0 | thin 0"));
    assert!(table.contains("TRADE"));
    assert!(table.contains("NORMAL"));
    assert!(table.contains("90.0 bps"));
    assert!(table.contains("flow proxy | signed notional 30s +$602"));
    assert!(table.contains("BBO OFI 30s -$515"));
    assert!(table.contains("top-of-book proxy only"));
    assert!(table.contains("resilience | state normal"));
    assert!(table.contains("tradeability tradeable"));
}
