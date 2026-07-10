use hls_core::{
    confidence::{ConfidenceLevel, ConfidenceReason, DataConfidenceSnapshot},
    data_gap::DataGap,
};
use hls_store::{
    metadata::MetadataRegistry,
    parquet::export_normalized_events_to_parquet,
    recorder::{RecordOptions, record_fixture_ndjson},
    replay::{ReplayInputFormat, ReplayOptions, replay_run, verify_or_insert_confidence_parity},
};

#[test]
fn replay_parity_writes_then_matches_confidence_baseline() {
    let temp = tempfile::tempdir().expect("tempdir");
    let data_dir = temp.path().join("data");
    record_fixture_ndjson(
        include_str!("../../../tests/fixtures/hyperliquid/ws_mock_live.ndjson"),
        RecordOptions::new(&data_dir, "parity-ok", vec!["@107".to_owned()], false, true),
    )
    .expect("record fixture");
    let options = ReplayOptions::new(&data_dir, "parity-ok", Vec::new());
    let summary = replay_run(options.clone()).expect("replay run");

    let first_report =
        verify_or_insert_confidence_parity(&options, &summary).expect("write baseline");
    assert!(first_report.matched);
    assert!(first_report.baseline_written);
    assert_eq!(first_report.replay_count, 1);

    let second_summary = replay_run(options.clone()).expect("replay again");
    let second_report =
        verify_or_insert_confidence_parity(&options, &second_summary).expect("match baseline");
    assert!(second_report.matched);
    assert!(!second_report.baseline_written);
    assert_eq!(second_report.drift_count, 0);
    assert_eq!(second_report.missing_count, 0);
    assert_eq!(second_report.extra_count, 0);
}

#[test]
fn replay_parity_reports_confidence_drift() {
    let temp = tempfile::tempdir().expect("tempdir");
    let data_dir = temp.path().join("data");
    record_fixture_ndjson(
        include_str!("../../../tests/fixtures/hyperliquid/ws_mock_live.ndjson"),
        RecordOptions::new(
            &data_dir,
            "parity-drift",
            vec!["@107".to_owned()],
            false,
            true,
        ),
    )
    .expect("record fixture");
    let options = ReplayOptions::new(&data_dir, "parity-drift", Vec::new());
    let summary = replay_run(options.clone()).expect("replay run");
    verify_or_insert_confidence_parity(&options, &summary).expect("write baseline");

    let registry = MetadataRegistry::open(data_dir.join("hls.sqlite")).expect("registry");
    let drifted = DataConfidenceSnapshot::new("@107")
        .with_reason(ConfidenceReason::ReconnectGap)
        .with_reason(ConfidenceReason::ParserDrops);
    assert_eq!(drifted.level, ConfidenceLevel::Low);
    registry
        .insert_confidence_snapshot("parity-drift", summary.snapshot_ts_ms, &drifted)
        .expect("tamper baseline");

    let drift_summary = replay_run(options.clone()).expect("replay drift");
    let drift_report =
        verify_or_insert_confidence_parity(&options, &drift_summary).expect("compare baseline");

    assert!(!drift_report.matched);
    assert_eq!(drift_report.drift_count, 1);
    assert!(drift_report.details[0].contains("@107"));
}

#[test]
fn replay_applies_recorded_data_gaps_to_confidence() {
    let temp = tempfile::tempdir().expect("tempdir");
    let data_dir = temp.path().join("data");
    record_fixture_ndjson(
        include_str!("../../../tests/fixtures/microstructure/gap_replay.ndjson"),
        RecordOptions::new(
            &data_dir,
            "gap-confidence",
            vec!["@107".to_owned()],
            false,
            true,
        ),
    )
    .expect("record fixture");
    let registry = MetadataRegistry::open(data_dir.join("hls.sqlite")).expect("registry");
    registry
        .insert_gap(&DataGap::new(
            "gap-confidence",
            1,
            1_710_000_060_000_000_000,
            1_710_000_061_000_000_000,
            "fixture reconnect gap",
            vec!["@107".to_owned()],
            false,
        ))
        .expect("insert gap");

    let summary = replay_run(ReplayOptions::new(&data_dir, "gap-confidence", Vec::new()))
        .expect("replay run");
    let snapshot = summary
        .snapshots
        .iter()
        .find(|snapshot| snapshot.symbol == "@107")
        .expect("snapshot");

    assert!(
        snapshot
            .confidence
            .has_reason(ConfidenceReason::ReconnectGap)
    );
    assert_eq!(snapshot.confidence.level, ConfidenceLevel::Low);
}

#[test]
fn replay_can_use_normalized_event_parquet_as_input() {
    let temp = tempfile::tempdir().expect("tempdir");
    let data_dir = temp.path().join("data");
    record_fixture_ndjson(
        include_str!("../../../tests/fixtures/hyperliquid/ws_mock_live.ndjson"),
        RecordOptions::new(
            &data_dir,
            "parquet-replay",
            vec!["@107".to_owned()],
            false,
            true,
        ),
    )
    .expect("record fixture");
    export_normalized_events_to_parquet(&data_dir, "parquet-replay")
        .expect("export normalized parquet");

    let jsonl_summary = replay_run(ReplayOptions::new(&data_dir, "parquet-replay", Vec::new()))
        .expect("jsonl replay");
    let parquet_summary = replay_run(
        ReplayOptions::new(&data_dir, "parquet-replay", Vec::new())
            .with_input_format(ReplayInputFormat::Parquet),
    )
    .expect("parquet replay");

    assert_eq!(parquet_summary.events_read, jsonl_summary.events_read);
    assert_eq!(parquet_summary.snapshot_ts_ms, jsonl_summary.snapshot_ts_ms);
    assert_eq!(
        parquet_summary.snapshots.len(),
        jsonl_summary.snapshots.len()
    );
    assert_eq!(parquet_summary.snapshots, jsonl_summary.snapshots);
}

#[test]
fn parquet_replay_requires_exported_schema_manifest() {
    let temp = tempfile::tempdir().expect("tempdir");
    let data_dir = temp.path().join("data");
    record_fixture_ndjson(
        include_str!("../../../tests/fixtures/hyperliquid/ws_mock_live.ndjson"),
        RecordOptions::new(
            &data_dir,
            "missing-parquet",
            vec!["@107".to_owned()],
            false,
            true,
        ),
    )
    .expect("record fixture");

    let err = replay_run(
        ReplayOptions::new(&data_dir, "missing-parquet", Vec::new())
            .with_input_format(ReplayInputFormat::Parquet),
    )
    .expect_err("parquet replay should require an exported manifest");

    assert!(
        err.to_string()
            .contains("no normalized-event Parquet schema manifest")
    );
}
