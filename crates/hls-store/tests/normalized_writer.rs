use hls_hyperliquid::ws::parser::parse_ws_ndjson;
use hls_store::normalized::{NormalizedWriter, read_normalized_events};

#[test]
fn normalized_writer_persists_replayable_market_events() {
    let temp = tempfile::tempdir().expect("tempdir");
    let events = parse_ws_ndjson(include_str!(
        "../../../tests/fixtures/hyperliquid/ws_mock_live.ndjson"
    ))
    .expect("fixture parses");

    let mut writer = NormalizedWriter::new(temp.path(), "run-normalized").expect("writer");
    let file = writer.write_events(&events).expect("write normalized");
    writer.finish().expect("finish normalized");

    assert_eq!(file.event_type, "normalized_jsonl");
    assert_eq!(file.rows, events.len() as u64);
    assert!(file.path.ends_with(".ndjson"));

    let replayed = read_normalized_events(temp.path().join(&file.path)).expect("read normalized");
    assert_eq!(replayed, events);
}
