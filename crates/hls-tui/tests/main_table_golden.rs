use hls_core::market_state::LiveMarketState;
use hls_features::engine::FeatureEngine;
use hls_hyperliquid::{rest::parse_metadata_enrichment_bundle, ws::parser::parse_ws_ndjson};
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

    assert!(table.contains("┌ Hyperliquid Spot Microstructure Workstation"));
    assert!(table.contains("REC ready"));
    assert!(table.contains("LIVE ●"));
    assert!(table.contains("p95 local"));
    assert!(table.contains("filter: READ-ONLY Hyperliquid spot live screen"));
    assert!(table.contains("mode: top-1 by screen rank"));
    assert!(table.contains("│ symbol"));
    assert!(table.contains("sprbp"));
    assert!(table.contains("flow30"));
    assert!(table.contains("amihud"));
    assert!(table.contains("why now"));
    assert!(table.contains("@107"));
    assert!(table.contains("35.2000"));
    assert!(table.contains("57.1"));
    assert!(table.contains("-0.15"));
    assert!(table.contains("thin + wide"));
    assert!(table.contains("Selected: @107"));
    assert!(table.contains("Bid/Ask        34.9000 / 35.1000"));
    assert!(table.contains("Micro-BBO      35.0000"));
    assert!(table.contains("Mark-Mid basis +142.9 bps"));
    assert!(table.contains("Top book       $105 / $140"));
    assert!(table.contains("OFI 30s"));
    assert!(table.contains("Spread recovery"));
    assert!(table.contains("Signed flow    5s:-  30s:"));
    assert!(table.contains("RV 1m/5m/1h   0.00/0.00/0.00"));
    assert!(table.contains("Confidence     gap:0 stale:0 sparse:0 reconnect:0 parser_drop:0"));
    assert!(table.contains("Why ranked"));
    assert!(table.contains("No wallet"));
    assert!(table.contains("Scores are screen heuristics, not orders or advice."));
}

#[test]
fn renders_compact_row_for_each_visible_pair_and_selects_first() {
    let events = parse_ws_ndjson(include_str!(
        "../../../tests/fixtures/hyperliquid/ws_mock_live.ndjson"
    ))
    .expect("fixture parses");
    let mut state = LiveMarketState::new(["@107".to_owned()]);
    for event in events {
        state.apply(event).expect("event applies");
    }
    let mut snapshots = FeatureEngine::default().snapshots(&state, 1_710_000_066_000);
    let mut second = snapshots[0].clone();
    second.symbol = "PURR/USDC".to_owned();
    second.price = Some(0.4200);
    second.day_ntl_vlm = Some(987_654.0);
    second.bid_px = Some(0.4000);
    second.bid_sz = Some(1_200.0);
    second.ask_px = Some(0.4200);
    second.ask_sz = Some(1_100.0);
    second.mid_px = Some(0.4100);
    second.mark_px = Some(0.4150);
    second.spread_bps = Some(487.8);
    second.tob_depth_usd = Some(966.0);
    second.tob_imbalance = Some(0.04);
    second.ret_1m = Some(0.0123);
    second.ret_5m = Some(-0.0042);
    second.ret_1h = Some(0.0840);
    second.rv_1m = Some(0.0090);
    second.rv_5m = Some(0.0210);
    second.rv_1h = Some(0.0440);
    second.volume_z_1h = Some(2.4);
    second.trade_count_z_1h = Some(-0.8);
    second.liquidity_score = 9.7;
    second.momentum_score = 49.6;
    second.mean_reversion_score = 50.4;
    second.score_breakdown = None;
    second.updated_ms_ago = Some(250);
    snapshots.push(second);

    let table = render_main_table(&snapshots);

    assert!(table.contains("│ @107"));
    assert!(table.contains("│ PURR/USDC"));
    assert!(table.contains("0.4200"));
    assert!(table.contains("487.8"));
    assert!(table.contains("+0.04"));
    assert!(table.contains("-$35"));
    assert!(table.contains("Selected: @107"));
    assert!(!table.contains("PAIR DETAIL CARDS"));
    assert!(!table.contains("02 PURR/USDC | px"));
    assert!(table.contains("metadata | tags unknown_metadata"));
}

#[test]
fn missing_quote_depth_marks_quality_partial() {
    let events = parse_ws_ndjson(include_str!(
        "../../../tests/fixtures/hyperliquid/ws_mock_live.ndjson"
    ))
    .expect("fixture parses");
    let mut state = LiveMarketState::new(["@107".to_owned()]);
    for event in events {
        state.apply(event).expect("event applies");
    }
    let mut snapshots = FeatureEngine::default().snapshots(&state, 1_710_000_066_000);
    for snapshot in &mut snapshots {
        snapshot.bid_px = None;
        snapshot.bid_sz = None;
        snapshot.ask_px = None;
        snapshot.ask_sz = None;
        snapshot.spread_bps = None;
        snapshot.tob_depth_usd = None;
        snapshot.tob_imbalance = None;
        snapshot.liquidity_score = 0.0;
    }

    let table = render_main_table(&snapshots);

    assert!(table.contains("│ @107"));
    assert!(table.contains("unknown"));
    assert!(table.contains("Bid/Ask        - / -"));
    assert!(table.contains("Top book       - / -"));
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

    assert!(table.contains("12.0"));
    assert!(table.contains("+$602"));
    assert!(table.contains("OFI 30s        -$515"));
    assert!(table.contains("Spread recovery 6.0s"));
    assert!(table.contains("Why ranked"));
    assert!(table.contains("+signed_flow"));
    assert!(table.contains("tradeability tradeable"));
}

#[test]
fn renders_metadata_tags_in_market_board_and_detail_pane() {
    let events = parse_ws_ndjson(include_str!(
        "../../../tests/fixtures/hyperliquid/ws_mock_live.ndjson"
    ))
    .expect("fixture parses");
    let metadata = parse_metadata_enrichment_bundle(include_str!(
        "../../../tests/fixtures/microstructure/metadata_enrichment.json"
    ))
    .expect("metadata fixture parses");
    let mut state = LiveMarketState::new(["@107".to_owned()]);
    for event in events {
        state.apply(event).expect("event applies");
    }
    let mut snapshots = FeatureEngine::default().snapshots(&state, 1_710_000_066_000);
    for snapshot in &mut snapshots {
        snapshot.metadata = metadata
            .iter()
            .find(|metadata| metadata.feed_identifier == snapshot.symbol)
            .cloned();
    }

    let table = render_main_table(&snapshots);

    assert!(table.contains("metadata | tags fresh_liquidity,low_float,new_listing"));
    assert!(table.contains("listing age 6.3d"));
    assert!(table.contains("seeded $1.2M"));
    assert!(table.contains("source spotMetaAndAssetCtxs+tokenDetails"));
    assert!(table.contains("new_listing"));
    assert!(table.contains("fresh_liquidity"));
}
