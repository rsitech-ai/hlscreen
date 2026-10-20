use hls_core::market_state::MarketEvent;
use hls_hyperliquid::{
    shared::{SharedConsumer, SharedUpdate, parse_shared_ws_message},
    ws::parser::parse_ws_message,
};
use marketfeed_recording::{
    MetadataRecord,
    shared_feed::{FeedProfile, FeedRecord, FeedWriter},
};
const FRAME: &str = r#" { "channel":"trades", "data":[{"coin":"BTC","side":"B","px":"100.25","sz":"0.002","hash":"0xabcdef","time":1700000000000,"tid":17}] } "#;
#[test]
fn shared_consumer_rejects_unprojected_causal_header_before_polling() {
    let temp = tempfile::tempdir().unwrap();
    for (index, profile) in [FeedProfile::EngineCausalV1, FeedProfile::EngineCausalV2]
        .into_iter()
        .enumerate()
    {
        let dir = temp.path().join(format!("causal-{index}"));
        let _writer = FeedWriter::create_profile(&dir, 1_000_000, profile).unwrap();
        // Rejection must occur at negotiation, before consulting a watermark,
        // reading metadata or creating feature windows from unsupported operations.
        std::fs::remove_file(dir.join("committed.json")).unwrap();
        let Err(error) = SharedConsumer::open(&dir) else {
            panic!("causal profile requires an explicit committed-release projection");
        };
        assert!(error.to_string().contains("unsupported shared feed header"));
    }
}
#[test]
fn shared_projection_preserves_legacy_feature_values_and_trade_identity() {
    assert_eq!(
        parse_shared_ws_message(FRAME).unwrap(),
        parse_ws_message(FRAME).unwrap()
    );
    let MarketEvent::Trade(t) = parse_shared_ws_message(FRAME).unwrap().remove(0) else {
        panic!()
    };
    assert_eq!(t.unique_trade_id, "BTC:1700000000000:17");
    assert_eq!(t.notional, 0.2005);
}
#[test]
fn consumer_waits_for_acceptance_and_replays_the_same_committed_input() {
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path().join("shared");
    let mut writer = FeedWriter::create(&dir, 1_000_000).unwrap();
    let metadata: MetadataRecord =
        serde_json::from_str(include_str!("fixtures/shared_perps_session.json")).unwrap();
    let session = metadata.session_id();
    writer.append(FeedRecord::Metadata(metadata)).unwrap();
    let mut consumer = SharedConsumer::open(&dir).unwrap();
    assert!(
        matches!(&consumer.poll(256).unwrap()[0],SharedUpdate::Symbols(s) if s==&["BTC","ETH"])
    );
    writer
        .append(FeedRecord::frame(
            session,
            5,
            1234567890,
            9,
            0,
            1,
            FRAME.as_bytes(),
        ))
        .unwrap();
    assert!(consumer.poll(256).unwrap().is_empty());
    assert!(consumer.caught_up());
    writer
        .append(FeedRecord::FrameAccepted {
            session,
            frame_seq: 5,
        })
        .unwrap();
    let mut updates = consumer.poll(256).unwrap();
    let SharedUpdate::Events(events) = updates.remove(0) else {
        panic!()
    };
    assert_eq!(events[0].recv_ts_ns(), 1234567890);
    let mut restarted = SharedConsumer::open(&dir).unwrap();
    let updates = restarted.poll(256).unwrap();
    let SharedUpdate::Events(replayed) = &updates[1] else {
        panic!()
    };
    assert_eq!(replayed, &events);
    assert_eq!(consumer.cursor(), restarted.cursor());
    writer
        .append(FeedRecord::Lifecycle {
            session,
            now_ns: 2,
            state: "disconnected:RemoteClose;continuity_unverified".into(),
        })
        .unwrap();
    assert!(
        matches!(&consumer.poll(256).unwrap()[0],SharedUpdate::Gap{symbols,..} if symbols.len()==2)
    );
}
#[test]
fn consumer_rejects_acceptance_with_wrong_original_frame() {
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path().join("shared");
    let mut writer = FeedWriter::create(&dir, 1_000_000).unwrap();
    let meta: MetadataRecord =
        serde_json::from_str(include_str!("fixtures/shared_perps_session.json")).unwrap();
    let session = meta.session_id();
    writer.append(FeedRecord::Metadata(meta)).unwrap();
    writer
        .append(FeedRecord::frame(
            session,
            5,
            123,
            1,
            0,
            1,
            FRAME.as_bytes(),
        ))
        .unwrap();
    writer
        .append(FeedRecord::FrameAccepted {
            session,
            frame_seq: 6,
        })
        .unwrap();
    assert!(SharedConsumer::open(&dir).unwrap().poll(256).is_err());
}

#[test]
fn archive_clock_is_bound_to_the_consumed_cursor_and_seal() {
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path().join("feed");
    let mut writer = FeedWriter::create(&dir, 1_000_000).unwrap();
    writer
        .append(FeedRecord::Timer {
            session: marketfeed_recording::MetadataRecord::current_build().session_id(),
            timer_id: 1,
            now_ns: 1_700_000_000_123_000_000,
        })
        .unwrap();
    let mut reader = SharedConsumer::open(&dir).unwrap();
    reader.poll(256).unwrap();
    assert_eq!(reader.as_of_ms(), Some(1_700_000_000_123));
    writer
        .append(FeedRecord::Seal {
            schema_version: 1,
            now_ns: 1_700_000_001_456_000_000,
            reason: "clean".into(),
        })
        .unwrap();
    reader.poll(256).unwrap();
    let mut restart = SharedConsumer::open(dir).unwrap();
    restart.poll(256).unwrap();
    assert!(reader.sealed());
    assert_eq!(reader.as_of_ms(), Some(1_700_000_001_456));
    assert_eq!(reader.as_of_ms(), restart.as_of_ms());
    assert_eq!(reader.cursor(), restart.cursor());
}

#[test]
fn interleaved_session_receipts_cannot_rewind_the_archive_clock() {
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path().join("feed");
    let mut writer = FeedWriter::create(&dir, 1_000_000).unwrap();
    let mut consumer = SharedConsumer::open(&dir).unwrap();
    for (session, now_ns) in [(1u64, 2_000_000i64), (2, 1_000_000), (1, 3_000_000)] {
        let record = serde_json::from_value(serde_json::json!({"kind":"timer","data":{"session":session,"timer_id":1,"now_ns":now_ns}})).unwrap();
        writer.append(record).unwrap();
        consumer.poll(256).unwrap();
        assert_eq!(
            consumer.as_of_ms(),
            Some(if now_ns < 3_000_000 { 2 } else { 3 })
        );
    }
}
