use hls_hyperliquid::rest::{parse_spot_meta, parse_spot_meta_and_asset_ctxs, select_universe};

#[test]
fn parses_spot_meta_into_market_symbols() {
    let symbols = parse_spot_meta(include_str!(
        "../../../tests/fixtures/hyperliquid/spot_meta.json"
    ))
    .expect("fixture parses");

    let purr = symbols
        .iter()
        .find(|symbol| symbol.display_name == "PURR/USDC")
        .expect("PURR symbol exists");
    let hype = symbols
        .iter()
        .find(|symbol| symbol.display_name == "HYPE/USDC")
        .expect("HYPE symbol exists");

    assert_eq!(symbols.len(), 3);
    assert_eq!(purr.hl_coin, "PURR/USDC");
    assert_eq!(hype.hl_coin, "@107");
    assert!(hype.is_canonical);
}

#[test]
fn parses_asset_contexts_and_sorts_volume_ranked_universe() {
    let markets = parse_spot_meta_and_asset_ctxs(include_str!(
        "../../../tests/fixtures/hyperliquid/spot_meta_and_asset_ctxs.json"
    ))
    .expect("fixture parses");

    let top_two = select_universe(&markets, 2, &[], &[]).expect("top universe selected");

    assert_eq!(markets.len(), 3);
    assert_eq!(top_two.len(), 2);
    assert_eq!(top_two[0].symbol.display_name, "HYPE/USDC");
    assert_eq!(top_two[0].symbol.hl_coin, "@107");
    assert_eq!(top_two[0].day_ntl_vlm, Some(25_000_000.5));
    assert_eq!(top_two[1].symbol.display_name, "PURR/USDC");
}

#[test]
fn universe_selection_applies_include_and_exclude_by_display_or_feed_id() {
    let markets = parse_spot_meta_and_asset_ctxs(include_str!(
        "../../../tests/fixtures/hyperliquid/spot_meta_and_asset_ctxs.json"
    ))
    .expect("fixture parses");

    let selected = select_universe(
        &markets,
        1,
        &["@107".to_owned(), "TEST/USDC".to_owned()],
        &["HYPE/USDC".to_owned()],
    )
    .expect("selection succeeds");

    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0].symbol.display_name, "TEST/USDC");
}

#[test]
fn spot_context_parser_rejects_non_finite_numeric_strings() {
    let raw = include_str!("../../../tests/fixtures/hyperliquid/spot_meta_and_asset_ctxs.json")
        .replacen("25000000.5", "NaN", 1);

    let error = parse_spot_meta_and_asset_ctxs(&raw)
        .expect_err("non-finite public REST values must not enter ranking state");

    assert!(error.to_string().contains("finite"));
}

#[test]
fn spot_context_parser_rejects_negative_volume_and_prices() {
    let fixture = include_str!("../../../tests/fixtures/hyperliquid/spot_meta_and_asset_ctxs.json");
    for raw in [
        fixture.replacen("25000000.5", "-1", 1),
        fixture.replacen("35.50", "-1", 1),
    ] {
        let error = parse_spot_meta_and_asset_ctxs(&raw)
            .expect_err("negative public REST values must not enter market state");
        assert!(
            error.to_string().contains("positive") || error.to_string().contains("non-negative"),
            "{error}"
        );
    }
}

#[test]
fn spot_context_parser_treats_zero_price_sentinels_as_missing() {
    let raw = include_str!("../../../tests/fixtures/hyperliquid/spot_meta_and_asset_ctxs.json")
        .replacen("\"prevDayPx\": \"34.00\"", "\"prevDayPx\": \"0.0\"", 1);
    let markets = parse_spot_meta_and_asset_ctxs(&raw)
        .expect("zero price sentinel should not invalidate the complete universe");
    let hype = markets
        .iter()
        .find(|market| market.symbol.hl_coin == "@107")
        .expect("HYPE market");

    assert_eq!(hype.prev_day_px, None);
}
