use hls_store::raw::{RawMarketMessage, RawWriter, read_raw_file};

#[test]
fn raw_writer_rotates_flushes_and_preserves_payloads() {
    let temp = tempfile::tempdir().expect("tempdir");
    let mut writer = RawWriter::new(temp.path(), "run-raw", 180).expect("writer");
    let fixture = include_str!("../../../tests/fixtures/hyperliquid/ws_mock_live.ndjson");

    for (seq, line) in fixture.lines().enumerate() {
        writer
            .write(
                &RawMarketMessage::from_ws_line(
                    1_780_000_000_000_000_000 + seq as u64,
                    7,
                    seq as u64,
                    line,
                )
                .unwrap(),
            )
            .expect("write raw");
    }

    let files = writer.finish().expect("finish");

    assert!(files.len() > 1, "small max bytes should rotate raw files");
    assert!(files.iter().all(|file| file.path.ends_with(".ndjson.zst")));
    assert_eq!(files.iter().map(|file| file.rows).sum::<u64>(), 6);

    let records = read_raw_file(temp.path().join(&files[0].path)).expect("read raw");
    assert_eq!(records[0].conn_id, 7);
    assert_eq!(records[0].seq, 0);
    assert_eq!(records[0].channel, "trades");
    assert_eq!(records[0].payload["channel"], "trades");
}
