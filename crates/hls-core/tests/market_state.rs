use std::collections::HashMap;

use hls_core::market_state::{
    AllMidsEvent, AssetContextEvent, CandleEvent, LiveMarketState, MarketEvent, TradeEvent,
    TradeSide,
};

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
    assert_eq!(symbol.duplicate_trade_count, 1);
    assert_eq!(symbol.last_trade_price, Some(35.0));
}

#[test]
fn market_events_can_be_stamped_with_live_receive_time() {
    let event = MarketEvent::AllMids(AllMidsEvent {
        recv_ts_ns: 0,
        mids_by_hl_coin: HashMap::from([("@107".to_owned(), 35.0)]),
    })
    .with_recv_ts_ns(1_710_000_000_123_000_000);

    assert_eq!(event.recv_ts_ns(), 1_710_000_000_123_000_000);

    let mut state = LiveMarketState::new(["@107".to_owned()]);
    state.apply(event).expect("all mids applies");
    let symbol = state.symbol_state("@107").expect("symbol state exists");
    assert_eq!(symbol.mid_px, Some(35.0));
    assert_eq!(symbol.last_update_ms, Some(1_710_000_000_123));
}

#[test]
fn asset_context_updates_refresh_live_staleness_when_receive_time_is_present() {
    let event = MarketEvent::AssetContext(AssetContextEvent {
        recv_ts_ns: 1_710_000_000_456_000_000,
        hl_coin: "@107".to_owned(),
        day_ntl_vlm: Some(1_000_000.0),
        prev_day_px: Some(34.0),
        mark_px: Some(35.0),
        mid_px: Some(35.1),
        circulating_supply: None,
    });

    let mut state = LiveMarketState::new(["@107".to_owned()]);
    state.apply(event).expect("asset context applies");
    let symbol = state.symbol_state("@107").expect("symbol state exists");
    assert_eq!(symbol.mark_px, Some(35.0));
    assert_eq!(symbol.last_update_ms, Some(1_710_000_000_456));
}

#[test]
fn candle_updates_replace_current_interval_and_bound_history() {
    let mut state = LiveMarketState::new(["@107".to_owned()]);
    state
        .apply(MarketEvent::Candle(candle(1_710_000_000_000, 35.0)))
        .expect("first candle applies");
    state
        .apply(MarketEvent::Candle(candle(1_710_000_000_000, 36.0)))
        .expect("current candle update applies");

    let symbol = state.symbol_state("@107").expect("symbol state exists");
    assert_eq!(symbol.candles.len(), 1);
    assert_eq!(symbol.candles[0].close, 36.0);

    for index in 1..600 {
        state
            .apply(MarketEvent::Candle(candle(
                1_710_000_000_000 + i64::from(index) * 60_000,
                40.0 + f64::from(index),
            )))
            .expect("historical candle applies");
    }

    let symbol = state.symbol_state("@107").expect("symbol state exists");
    assert_eq!(symbol.candles.len(), 512);
    assert!(symbol.candles.first().expect("first candle").open_ts_ms > 1_710_000_000_000);
    assert_eq!(
        symbol.candles.last().expect("last candle").open_ts_ms,
        1_710_000_000_000 + 599 * 60_000
    );
}

fn candle(open_ts_ms: i64, close: f64) -> CandleEvent {
    CandleEvent {
        recv_ts_ns: open_ts_ms as u64 * 1_000_000,
        open_ts_ms,
        close_ts_ms: open_ts_ms + 59_999,
        hl_coin: "@107".to_owned(),
        interval: "1m".to_owned(),
        open: 35.0,
        high: close.max(35.0),
        low: close.min(35.0),
        close,
        volume_base: 1200.0,
        trade_count: 42,
    }
}
