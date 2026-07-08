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

    assert!(table.contains("Hyperliquid Microstructure Workstation"));
    assert!(table.contains("PUBLIC WS/REST"));
    assert!(table.contains("SESSION"));
    assert!(table.contains("LATENCY"));
    assert!(table.contains("QUALITY"));
    assert!(table.contains("CONFIDENCE"));
    assert!(table.contains("RESILIENCE"));
    assert!(table.contains("METADATA"));
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
    assert!(table.contains("PAIR DETAIL CARDS"));
    assert!(table.contains("bid 34.9000 x 3.0000"));
    assert!(table.contains("ask 35.1000 x 4.0000"));
    assert!(table.contains("24h notional $25.0M"));
    assert!(table.contains("ret 1m - / 5m +0.57% / 1h +0.57%"));
    assert!(table.contains("rv 1m 0.00% / 5m 0.00% / 1h 0.00%"));
    assert!(table.contains("activity | volume z +0.0 | trades z +0.0"));
    assert!(table.contains("liq/mom/mr 2.5/50.6/49.4"));
    assert!(table.contains("confidence | high 100 | reasons none"));
    assert!(table.contains("why ranked | score"));
    assert!(table.contains("components 5"));
    assert!(table.contains("No wallet"));
    assert!(table.contains("Scores are screen heuristics, not orders or advice."));
}

#[test]
fn renders_pair_detail_card_for_each_visible_pair() {
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

    assert!(table.contains("01 @107 | px 35.2000 | 24h notional $25.0M"));
    assert!(table.contains("02 PURR/USDC | px 0.4200 | 24h notional $987.7K"));
    assert!(table.contains("bid 0.4000 x 1200.0000"));
    assert!(table.contains("ask 0.4200 x 1100.0000"));
    assert!(table.contains("mid 0.4100 | mark 0.4150"));
    assert!(table.contains("ret 1m +1.23% / 5m -0.42% / 1h +8.40%"));
    assert!(table.contains("rv 1m 0.90% / 5m 2.10% / 1h 4.40%"));
    assert!(table.contains("activity | volume z +2.4 | trades z -0.8 | liq/mom/mr 9.7/49.6/50.4",));
    assert!(table.contains("quality | ● fresh age 250ms"));
    assert!(table.contains("metadata | tags unknown_metadata"));
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
    assert!(table.contains("flow | signed notional 30s +$602"));
    assert!(table.contains("BBO OFI 30s -$515"));
    assert!(table.contains("top-of-book proxy only"));
    assert!(table.contains("resilience | state normal"));
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

    assert!(table.contains("META"));
    assert!(table.contains("complete 1 | partial 0 | missing 0 | new 1 | fresh liquidity 1"));
    assert!(table.contains("NEW+SEED"));
    assert!(table.contains("metadata | tags fresh_liquidity,low_float,new_listing"));
    assert!(table.contains("listing age 6.3d"));
    assert!(table.contains("seeded $1.2M"));
    assert!(table.contains("source spotMetaAndAssetCtxs+tokenDetails"));
    assert!(table.contains("new listing"));
    assert!(table.contains("fresh liquidity"));
}
