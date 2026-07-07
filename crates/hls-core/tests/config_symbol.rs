use std::time::Duration;

use hls_core::{
    config::load_config_str,
    symbol::{MarketSymbol, feed_id_for_spot},
    time::duration_to_millis,
};

#[test]
fn example_config_parses_with_read_only_safety_defaults() {
    let config = load_config_str(include_str!("../../../config/example.toml"))
        .expect("example config parses");

    assert_eq!(config.data_dir.to_string_lossy(), ".hls");
    assert_eq!(config.universe.top_n, 150);
    assert!(config.streams.trades);
    assert!(config.streams.bbo);
    assert!(!config.streams.l2_book);
    assert!(config.safety.read_only);
    assert!(!config.safety.wallet_enabled);
    assert!(!config.safety.trading_enabled);
}

#[test]
fn symbol_mapping_preserves_display_name_and_feed_identifier() {
    let purr = MarketSymbol::new("PURR/USDC", 0, 1, 0, 0, 5, true).expect("valid PURR symbol");
    let hype = MarketSymbol::new("HYPE/USDC", 107, 150, 0, 2, 8, true).expect("valid HYPE symbol");

    assert_eq!(purr.display_name, "PURR/USDC");
    assert_eq!(purr.hl_coin, "PURR/USDC");
    assert_eq!(feed_id_for_spot("PURR/USDC", 0), "PURR/USDC");

    assert_eq!(hype.display_name, "HYPE/USDC");
    assert_eq!(hype.hl_coin, "@107");
    assert_eq!(feed_id_for_spot("HYPE/USDC", 107), "@107");
}

#[test]
fn symbol_mapping_rejects_empty_display_names() {
    let err =
        MarketSymbol::new("", 107, 150, 0, 2, 8, true).expect_err("empty display name is invalid");

    assert!(err.to_string().contains("display name"));
}

#[test]
fn duration_helpers_are_deterministic() {
    assert_eq!(duration_to_millis(Duration::from_secs(10)), 10_000);
    assert_eq!(duration_to_millis(Duration::from_millis(250)), 250);
}
