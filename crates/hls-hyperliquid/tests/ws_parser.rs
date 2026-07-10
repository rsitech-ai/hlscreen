use hls_core::market_state::{MarketEvent, TradeSide};
use hls_hyperliquid::ws::{
    parser::{parse_ws_message, parse_ws_ndjson},
    subscriptions::{StreamKind, SubscriptionPlan, ping_message, subscribe_message},
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
fn parses_spot_asset_context_runtime_channel_alias() {
    let events = parse_ws_message(
        r#"{"channel":"activeSpotAssetCtx","data":{"coin":"@107","ctx":{"dayNtlVlm":"25000000.5","prevDayPx":"34.5","markPx":"35.5","midPx":"35.45","circulatingSupply":"12345.0"}}}"#,
    )
    .expect("spot asset context alias parses");

    match &events[0] {
        MarketEvent::AssetContext(ctx) => {
            assert_eq!(ctx.hl_coin, "@107");
            assert_eq!(ctx.day_ntl_vlm, Some(25_000_000.5));
            assert_eq!(ctx.mark_px, Some(35.5));
        }
        other => panic!("expected asset context, got {other:?}"),
    }
}

#[test]
fn parses_single_candle_runtime_payload() {
    let events = parse_ws_message(
        r#"{"channel":"candle","data":{"t":1710000000000,"T":1710000059999,"s":"@107","i":"1m","o":"35.0","c":"35.2","h":"35.3","l":"34.9","v":"120.0","n":"42"}}"#,
    )
    .expect("single candle parses");

    match &events[0] {
        MarketEvent::Candle(candle) => {
            assert_eq!(candle.hl_coin, "@107");
            assert_eq!(candle.close, 35.2);
            assert_eq!(candle.trade_count, 42);
        }
        other => panic!("expected candle, got {other:?}"),
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
fn parse_ws_message_ignores_control_channels() {
    assert!(
        parse_ws_message(
            r#"{"channel":"subscriptionResponse","data":{"method":"subscribe","subscription":{"type":"trades","coin":"@107"}}}"#
        )
        .expect("subscription response parses")
        .is_empty()
    );
    assert!(
        parse_ws_message(r#"{"channel":"pong"}"#)
            .expect("pong parses")
            .is_empty()
    );
}

#[test]
fn parse_ws_message_rejects_non_finite_market_numbers() {
    for raw in [
        r#"{"channel":"trades","data":[{"coin":"@107","side":"B","px":"NaN","sz":"2","time":1710000000000,"hash":"0xabc","tid":11}]}"#,
        r#"{"channel":"activeAssetCtx","data":{"coin":"@107","ctx":{"dayNtlVlm":"Infinity","prevDayPx":"34.5","markPx":"35.5","midPx":"35.45"}}}"#,
        r#"{"channel":"candle","data":{"t":1710000000000,"T":1710000059999,"s":"@107","i":"1m","o":"35.0","c":"35.2","h":"35.3","l":"34.9","v":"NaN","n":"42"}}"#,
    ] {
        let error = parse_ws_message(raw).expect_err("non-finite market data must be rejected");
        assert!(error.to_string().contains("finite"), "{error}");
    }
}

#[test]
fn parse_ws_message_rejects_semantically_invalid_market_numbers() {
    for raw in [
        r#"{"channel":"trades","data":[{"coin":"@107","side":"B","px":"35","sz":"2","time":-1,"hash":"0xabc","tid":11}]}"#,
        r#"{"channel":"bbo","data":{"coin":"@107","time":-1,"bbo":[{"px":"34.9","sz":"2","n":1},{"px":"35.1","sz":"3","n":1}]}}"#,
        r#"{"channel":"activeAssetCtx","data":{"coin":"@107","ctx":{"dayNtlVlm":"-1","prevDayPx":"34.5","markPx":"35.5","midPx":"35.45"}}}"#,
        r#"{"channel":"activeAssetCtx","data":{"coin":"@107","ctx":{"dayNtlVlm":"1","prevDayPx":"-34.5","markPx":"35.5","midPx":"35.45"}}}"#,
        r#"{"channel":"candle","data":{"t":1710000000000,"T":1710000059999,"s":"@107","i":"1m","o":"35.0","c":"35.2","h":"35.3","l":"34.9","v":"-1","n":"42"}}"#,
    ] {
        let error = parse_ws_message(raw).expect_err("invalid market values must be rejected");
        assert!(
            error.to_string().contains("positive") || error.to_string().contains("non-negative"),
            "{error}"
        );
    }
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

    let cap_bypass = SubscriptionPlan::new(vec!["@107".to_owned()]).with_max_subscriptions(1_001);
    let err = cap_bypass
        .validate()
        .expect_err("caller cannot raise the official connection limit");
    assert!(err.to_string().contains("official"));
}

#[test]
fn subscription_messages_match_official_public_shape() {
    assert_eq!(
        subscribe_message("@107", StreamKind::Trades).expect("subscribe serializes"),
        r#"{"method":"subscribe","subscription":{"coin":"@107","type":"trades"}}"#
    );
    assert_eq!(
        subscribe_message("@107", StreamKind::Bbo).expect("subscribe serializes"),
        r#"{"method":"subscribe","subscription":{"coin":"@107","type":"bbo"}}"#
    );
    assert_eq!(
        subscribe_message("@107", StreamKind::ActiveAssetCtx).expect("subscribe serializes"),
        r#"{"method":"subscribe","subscription":{"coin":"@107","type":"activeAssetCtx"}}"#
    );
    assert_eq!(
        subscribe_message("@107", StreamKind::Candle1m).expect("subscribe serializes"),
        r#"{"method":"subscribe","subscription":{"coin":"@107","interval":"1m","type":"candle"}}"#
    );
    assert_eq!(ping_message(), r#"{"method":"ping"}"#);
}

#[test]
fn default_subscription_budget_covers_default_top_universe_with_headroom() {
    let symbols = (0..150).map(|index| format!("@{index}")).collect();
    let plan = SubscriptionPlan::new(symbols);

    assert_eq!(plan.subscription_count(), 600);
    assert!(plan.validate().is_ok());

    let too_many_symbols = (0..246).map(|index| format!("@{index}")).collect();
    let too_many = SubscriptionPlan::new(too_many_symbols);

    assert_eq!(too_many.subscription_count(), 984);
    assert!(too_many.validate().is_err());
}

#[test]
fn global_all_mids_is_counted_once_for_large_universes() {
    let symbols = (0..700).map(|index| format!("@{index}")).collect();
    let plan = SubscriptionPlan::new(symbols)
        .with_streams([StreamKind::AllMids, StreamKind::ActiveAssetCtx]);

    assert_eq!(plan.subscription_count(), 701);
    assert_eq!(plan.per_symbol_stream_count(), 1);
    assert_eq!(plan.global_stream_count(), 1);
    let messages = plan
        .subscribe_messages()
        .expect("large plan stays in budget");
    assert_eq!(messages.len(), 701);
    assert_eq!(
        messages.first().map(String::as_str),
        Some(r#"{"method":"subscribe","subscription":{"type":"allMids"}}"#)
    );
}
