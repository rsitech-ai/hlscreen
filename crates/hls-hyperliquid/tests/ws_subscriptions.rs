use hls_hyperliquid::ws::subscriptions::{
    StreamKind, SubscriptionPlan, subscribe_message, unsubscribe_message,
};

#[test]
fn tiered_plan_keeps_every_candle_and_stays_below_budget() {
    let symbols = (0..309)
        .map(|index| format!("@{index}"))
        .collect::<Vec<_>>();

    let plan = SubscriptionPlan::tiered(symbols, 309, 100, Some("@0".to_owned()));

    assert_eq!(plan.subscription_count(), 720);
    assert_eq!(plan.stream_count(StreamKind::Candle1m), 309);
    assert_eq!(plan.stream_count(StreamKind::Trades), 309);
    assert_eq!(plan.stream_count(StreamKind::Bbo), 100);
    assert_eq!(plan.stream_count(StreamKind::AllMids), 1);
    assert_eq!(plan.stream_count(StreamKind::L2Book), 1);
    plan.validate().expect("tiered plan stays within budget");
}

#[test]
fn tiered_plan_deduplicates_symbols_before_budgeting() {
    let plan = SubscriptionPlan::tiered(
        vec!["@0".to_owned(), "@0".to_owned(), "@1".to_owned()],
        usize::MAX,
        usize::MAX,
        Some("@0".to_owned()),
    );

    assert_eq!(plan.symbols(), &["@0".to_owned(), "@1".to_owned()]);
    assert_eq!(plan.stream_count(StreamKind::Candle1m), 2);
    assert_eq!(plan.subscription_count(), 8);
    plan.validate().expect("deduplicated plan is valid");
}

#[test]
fn mandatory_candles_fail_closed_when_the_budget_cannot_fit() {
    let symbols = (0..980)
        .map(|index| format!("@{index}"))
        .collect::<Vec<_>>();
    let plan = SubscriptionPlan::tiered(symbols, 0, 0, None);

    assert_eq!(plan.stream_count(StreamKind::Candle1m), 980);
    let error = plan
        .validate()
        .expect_err("global mids plus mandatory candles exceed the 980 ceiling");
    assert!(error.to_string().contains("subscription budget exceeded"));
}

#[test]
fn configured_ceiling_cannot_exceed_the_official_limit() {
    let plan =
        SubscriptionPlan::tiered(vec!["@0".to_owned()], 1, 1, None).with_max_subscriptions(1_001);

    let error = plan
        .validate()
        .expect_err("official ceiling is enforced locally");
    assert!(error.to_string().contains("official limit of 1000"));
}

#[test]
fn unsubscribe_payload_matches_the_original_subscription() {
    let subscribe =
        subscribe_message("@107", StreamKind::L2Book).expect("L2 subscribe payload serializes");
    let unsubscribe =
        unsubscribe_message("@107", StreamKind::L2Book).expect("L2 unsubscribe payload serializes");

    assert_eq!(
        subscribe,
        r#"{"method":"subscribe","subscription":{"coin":"@107","type":"l2Book"}}"#
    );
    assert_eq!(
        unsubscribe,
        r#"{"method":"unsubscribe","subscription":{"coin":"@107","type":"l2Book"}}"#
    );
}

#[test]
fn uniform_plan_emits_each_global_stream_once() {
    let plan = SubscriptionPlan::new(vec!["@0".to_owned(), "@1".to_owned()])
        .with_streams([StreamKind::AllMids, StreamKind::Candle1m]);

    assert_eq!(plan.stream_count(StreamKind::AllMids), 1);
    assert_eq!(plan.stream_count(StreamKind::Candle1m), 2);
    assert_eq!(plan.subscription_count(), 3);
    plan.validate().expect("global stream is deduplicated");
}
