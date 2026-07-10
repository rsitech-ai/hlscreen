use assert_cmd::Command;
use hls_core::confidence::{ConfidenceReason, DataConfidenceSnapshot};
use hls_store::metadata::MetadataRegistry;
use predicates::prelude::*;

fn fixture(path: &str) -> String {
    format!("{}/../../{}", env!("CARGO_MANIFEST_DIR"), path)
}

#[test]
fn replay_verify_parity_writes_matches_and_fails_on_confidence_drift() {
    let temp = tempfile::tempdir().expect("tempdir");
    let data_dir = temp.path().join("data");

    Command::cargo_bin("hls")
        .expect("hls binary")
        .args([
            "record",
            "--symbols",
            "@107",
            "--fixture-file",
            &fixture("tests/fixtures/hyperliquid/ws_mock_live.ndjson"),
            "--normalized",
            "--run-id",
            "parity-cli",
            "--data-dir",
        ])
        .arg(&data_dir)
        .assert()
        .success();

    Command::cargo_bin("hls")
        .expect("hls binary")
        .args(["replay", "--data-dir"])
        .arg(&data_dir)
        .args(["--run-id", "parity-cli", "--verify-parity"])
        .assert()
        .success()
        .stdout(predicate::str::contains("replay_parity=baseline_written"))
        .stdout(predicate::str::contains("confidence_drift=0"))
        .stdout(predicate::str::contains(
            "Hyperliquid Spot Microstructure Workstation",
        ));

    Command::cargo_bin("hls")
        .expect("hls binary")
        .args(["replay", "--data-dir"])
        .arg(&data_dir)
        .args(["--run-id", "parity-cli", "--verify-parity"])
        .assert()
        .success()
        .stdout(predicate::str::contains("replay_parity=passed"))
        .stdout(predicate::str::contains("confidence_drift=0"));

    let registry = MetadataRegistry::open(data_dir.join("hls.sqlite")).expect("registry");
    let baselines = registry
        .list_confidence_snapshots("parity-cli")
        .expect("confidence baselines");
    assert_eq!(baselines.len(), 1);
    let drifted = DataConfidenceSnapshot::new("@107")
        .with_reason(ConfidenceReason::ReconnectGap)
        .with_reason(ConfidenceReason::ParserDrops);
    registry
        .insert_confidence_snapshot("parity-cli", baselines[0].snapshot_ts_ms, &drifted)
        .expect("tamper baseline");

    Command::cargo_bin("hls")
        .expect("hls binary")
        .args(["replay", "--data-dir"])
        .arg(&data_dir)
        .args(["--run-id", "parity-cli", "--verify-parity"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("replay_parity=drifted"))
        .stdout(predicate::str::contains("confidence_drift=1"))
        .stderr(predicate::str::contains("replay parity drift detected"));
}

#[test]
fn replay_command_can_read_normalized_event_parquet() {
    let temp = tempfile::tempdir().expect("tempdir");
    let data_dir = temp.path().join("data");

    Command::cargo_bin("hls")
        .expect("hls binary")
        .args([
            "record",
            "--symbols",
            "@107",
            "--fixture-file",
            &fixture("tests/fixtures/hyperliquid/ws_mock_live.ndjson"),
            "--normalized",
            "--run-id",
            "parquet-replay-cli",
            "--data-dir",
        ])
        .arg(&data_dir)
        .assert()
        .success();

    Command::cargo_bin("hls")
        .expect("hls binary")
        .args([
            "export-parquet",
            "--run-id",
            "parquet-replay-cli",
            "--data-dir",
        ])
        .arg(&data_dir)
        .assert()
        .success()
        .stdout(predicate::str::contains("event_type=normalized_parquet"));

    Command::cargo_bin("hls")
        .expect("hls binary")
        .args(["replay", "--data-dir"])
        .arg(&data_dir)
        .args([
            "--run-id",
            "parquet-replay-cli",
            "--input",
            "parquet",
            "--verify-parity",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("replay_parity=baseline_written"))
        .stdout(predicate::str::contains(
            "READ-ONLY Hyperliquid spot replay",
        ))
        .stdout(predicate::str::contains("@107"))
        .stdout(predicate::str::contains("Confidence"));
}
