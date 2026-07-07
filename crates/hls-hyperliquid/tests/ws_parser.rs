use hls_core::market_state::{MarketEvent, TradeSide};
use hls_hyperliquid::ws::{
    parser::{parse_ws_message, parse_ws_ndjson},
    subscriptions::{StreamKind, SubscriptionPlan},
};

#[test]
fn parses_public_market_data_channels_from_fixture() {
    let events = parse_ws_ndjson(include_str!(
        "../../../tests/fixtures/hyperliquid/ws_mock_live.ndjson"
    ))
    .expect("fixture parses");

    assert_eq!(events.len(), 6);

    match &events[0] {
        MarketEvent::Trade(trade) => {
            assert_eq!(trade.hl_coin, "@107");
            assert_eq!(trade.side, TradeSide::Buy);
            assert_eq!(trade.price, 35.00);
            assert_eq!(trade.size, 2.0);
            assert_eq!(trade.exchange_ts_ms, 1_710_000_000_000);
            assert_eq!(trade.unique_trade_id, "@107:1710000000000:11");
        }
        other => panic!("expected trade event, got {other:?}"),
    }

    match &events[1] {
        MarketEvent::TopOfBook(book) => {
            assert_eq!(book.hl_coin, "@107");
            assert_eq!(book.bid_price, Some(34.90));
            assert_eq!(book.ask_price, Some(35.10));
            assert_eq!(book.bid_order_count, Some(2));
            assert_eq!(book.ask_order_count, Some(3));
        }
        other => panic!("expected top-of-book event, got {other:?}"),
    }

    match &events[2] {
        MarketEvent::AllMids(mids) => {
            assert_eq!(mids.mids_by_hl_coin.get("@107"), Some(&35.00));
            assert_eq!(mids.mids_by_hl_coin.get("PURR/USDC"), Some(&0.1248));
        }
        other => panic!("expected all-mids event, got {other:?}"),
    }

    match &events[3] {
        MarketEvent::AssetContext(ctx) => {
            assert_eq!(ctx.hl_coin, "@107");
            assert_eq!(ctx.day_ntl_vlm, Some(25_000_000.5));
            assert_eq!(ctx.mark_px, Some(35.50));
            assert_eq!(ctx.mid_px, Some(35.45));
        }
        other => panic!("expected asset-context event, got {other:?}"),
    }

    match &events[4] {
        MarketEvent::Candle(candle) => {
            assert_eq!(candle.hl_coin, "@107");
            assert_eq!(candle.interval, "1m");
            assert_eq!(candle.open_ts_ms, 1_710_000_000_000);
            assert_eq!(candle.close, 35.00);
            assert_eq!(candle.trade_count, 42);
        }
        other => panic!("expected candle event, got {other:?}"),
    }
}

#[test]
fn parse_ws_message_rejects_private_user_channels() {
    let err = parse_ws_message(r#"{"channel":"userFills","data":{"user":"0xabc","fills":[]}}"#)
        .expect_err("private user stream is out of scope");

    assert!(
        err.to_string()
            .contains("unsupported private or trading channel")
    );
}

#[test]
fn subscription_plan_counts_public_channels_and_preserves_headroom() {
    let plan = SubscriptionPlan::new(vec!["@107".to_owned(), "PURR/USDC".to_owned()])
        .with_streams([
            StreamKind::Trades,
            StreamKind::Bbo,
            StreamKind::ActiveAssetCtx,
        ])
        .with_max_subscriptions(10);

    assert_eq!(plan.subscription_count(), 6);
    assert!(plan.validate().is_ok());

    let unsafe_plan = plan.with_max_subscriptions(5);
    let err = unsafe_plan
        .validate()
        .expect_err("budget violation is rejected");
    assert!(err.to_string().contains("subscription budget"));
}
