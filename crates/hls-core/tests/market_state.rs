use hls_core::market_state::{LiveMarketState, MarketEvent, TradeEvent, TradeSide};

#[test]
fn duplicate_trade_ids_are_idempotent_across_replay_or_reconnect() {
    let mut state = LiveMarketState::new(["@107".to_owned()]);
    let trade = TradeEvent {
        recv_ts_ns: 1,
        exchange_ts_ms: 1_710_000_000_000,
        hl_coin: "@107".to_owned(),
        side: TradeSide::Buy,
        price: 35.0,
        size: 2.0,
        notional: 70.0,
        hash: "0xabc".to_owned(),
        tid: 11,
        unique_trade_id: "@107:1710000000000:11".to_owned(),
    };

    state
        .apply(MarketEvent::Trade(trade.clone()))
        .expect("first trade applies");
    state
        .apply(MarketEvent::Trade(trade))
        .expect("duplicate trade is ignored without error");

    let symbol = state.symbol_state("@107").expect("symbol state exists");
    assert_eq!(symbol.trades.len(), 1);
    assert_eq!(symbol.last_trade_price, Some(35.0));
}
