use std::collections::HashMap;

use hls_core::market_state::{
    AllMidsEvent, AssetContextEvent, CandleEvent, LiveMarketState, MarketEvent, TopOfBookEvent,
    TradeEvent, TradeSide,
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
fn trade_history_is_hour_bounded_and_late_events_do_not_regress_live_price() {
    let mut state = LiveMarketState::new(["@107".to_owned()]);
    let base = 1_710_000_000_000_i64;

    state
        .apply(MarketEvent::Trade(trade(base, 35.0, 1)))
        .expect("first trade applies");
    state
        .apply(MarketEvent::Trade(trade(base + 3_600_001, 36.0, 2)))
        .expect("new hour trade applies");
    state
        .apply(MarketEvent::Trade(trade(base, 10.0, 3)))
        .expect("late stale trade is safely ignored");

    let symbol = state.symbol_state("@107").expect("symbol state exists");
    assert_eq!(symbol.trades.len(), 1);
    assert_eq!(symbol.trades[0].price, 36.0);
    assert_eq!(symbol.last_trade_price, Some(36.0));
    assert_eq!(symbol.last_trade_ts_ms, Some(base + 3_600_001));
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
fn out_of_order_quote_and_context_events_do_not_regress_current_market_state() {
    let mut state = LiveMarketState::new(["@107".to_owned()]);
    state
        .apply(MarketEvent::TopOfBook(bbo(2_000, 35.0, 35.2)))
        .expect("latest bbo applies");
    state
        .apply(MarketEvent::TopOfBook(bbo(1_000, 10.0, 10.2)))
        .expect("late bbo is retained without becoming current");

    state
        .apply(MarketEvent::AssetContext(AssetContextEvent {
            recv_ts_ns: 3_000_000_000,
            hl_coin: "@107".to_owned(),
            day_ntl_vlm: Some(1_000_000.0),
            prev_day_px: Some(34.0),
            mark_px: Some(36.0),
            mid_px: Some(36.1),
            circulating_supply: None,
        }))
        .expect("latest context applies");
    state
        .apply(MarketEvent::AssetContext(AssetContextEvent {
            recv_ts_ns: 2_000_000_000,
            hl_coin: "@107".to_owned(),
            day_ntl_vlm: Some(1.0),
            prev_day_px: Some(9.0),
            mark_px: Some(10.0),
            mid_px: Some(10.1),
            circulating_supply: None,
        }))
        .expect("late context is ignored");

    state
        .apply(MarketEvent::AllMids(AllMidsEvent {
            recv_ts_ns: 5_000_000_000,
            mids_by_hl_coin: HashMap::from([("@107".to_owned(), 37.0)]),
        }))
        .expect("latest all-mids applies");
    state
        .apply(MarketEvent::AllMids(AllMidsEvent {
            recv_ts_ns: 4_000_000_000,
            mids_by_hl_coin: HashMap::from([("@107".to_owned(), 11.0)]),
        }))
        .expect("late all-mids is ignored");

    let symbol = state.symbol_state("@107").expect("symbol state exists");
    assert_eq!(symbol.bid_px, Some(35.0));
    assert_eq!(symbol.ask_px, Some(35.2));
    assert_eq!(symbol.mark_px, Some(36.0));
    assert_eq!(symbol.day_ntl_vlm, Some(1_000_000.0));
    assert_eq!(symbol.mid_px, Some(37.0));
    assert_eq!(symbol.last_update_ms, Some(5_000));
    assert_eq!(
        symbol
            .bbo_events
            .iter()
            .map(|event| event.exchange_ts_ms)
            .collect::<Vec<_>>(),
        vec![1_000, 2_000]
    );
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
        provenance: Default::default(),
        completion: Default::default(),
    }
}

fn trade(exchange_ts_ms: i64, price: f64, tid: u64) -> TradeEvent {
    TradeEvent {
        recv_ts_ns: exchange_ts_ms as u64 * 1_000_000,
        exchange_ts_ms,
        hl_coin: "@107".to_owned(),
        side: TradeSide::Buy,
        price,
        size: 1.0,
        notional: price,
        hash: format!("0x{tid:x}"),
        tid,
        unique_trade_id: format!("@107:{exchange_ts_ms}:{tid}"),
    }
}

fn bbo(exchange_ts_ms: i64, bid: f64, ask: f64) -> TopOfBookEvent {
    TopOfBookEvent {
        recv_ts_ns: exchange_ts_ms as u64 * 1_000_000,
        exchange_ts_ms,
        hl_coin: "@107".to_owned(),
        bid_price: Some(bid),
        bid_size: Some(2.0),
        bid_order_count: Some(1),
        ask_price: Some(ask),
        ask_size: Some(3.0),
        ask_order_count: Some(1),
    }
}
