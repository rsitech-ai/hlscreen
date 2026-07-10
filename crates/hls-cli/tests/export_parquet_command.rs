use assert_cmd::Command;
use predicates::prelude::*;

fn fixture(path: &str) -> String {
    format!("{}/../../{}", env!("CARGO_MANIFEST_DIR"), path)
}

#[test]
fn export_parquet_writes_registered_file_from_normalized_run() {
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
            "parquet-cli",
            "--data-dir",
        ])
        .arg(&data_dir)
        .assert()
        .success();

    Command::cargo_bin("hls")
        .expect("hls binary")
        .args(["export-parquet", "--run-id", "parquet-cli", "--data-dir"])
        .arg(&data_dir)
        .assert()
        .success()
        .stdout(predicate::str::contains("parquet_run=parquet-cli"))
        .stdout(predicate::str::contains("event_type=normalized_parquet"))
        .stdout(predicate::str::contains("rows=6"))
        .stdout(predicate::str::contains(
            "parquet/events/run=parquet-cli/part-000000.parquet",
        ));

    assert!(
        data_dir
            .join("parquet/events/run=parquet-cli/part-000000.parquet")
            .exists()
    );
    assert!(
        data_dir
            .join("parquet/events/run=parquet-cli/schema.json")
            .exists()
    );
}

#[test]
fn export_parquet_can_write_feature_confidence_dataset() {
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
            "feature-parquet-cli",
            "--data-dir",
        ])
        .arg(&data_dir)
        .assert()
        .success();

    Command::cargo_bin("hls")
        .expect("hls binary")
        .args([
            "export-parquet",
            "--dataset",
            "features",
            "--run-id",
            "feature-parquet-cli",
            "--data-dir",
        ])
        .arg(&data_dir)
        .assert()
        .success()
        .stdout(predicate::str::contains("parquet_run=feature-parquet-cli"))
        .stdout(predicate::str::contains(
            "event_type=feature_snapshot_parquet",
        ))
        .stdout(predicate::str::contains(
            "parquet/features/run=feature-parquet-cli/part-000000.parquet",
        ));

    assert!(
        data_dir
            .join("parquet/features/run=feature-parquet-cli/part-000000.parquet")
            .exists()
    );
    assert!(
        data_dir
            .join("parquet/features/run=feature-parquet-cli/schema.json")
            .exists()
    );
}

#[test]
fn export_parquet_all_writes_events_and_feature_datasets() {
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
            "all-parquet-cli",
            "--data-dir",
        ])
        .arg(&data_dir)
        .assert()
        .success();

    Command::cargo_bin("hls")
        .expect("hls binary")
        .args([
            "export-parquet",
            "--dataset",
            "all",
            "--run-id",
            "all-parquet-cli",
            "--data-dir",
        ])
        .arg(&data_dir)
        .assert()
        .success()
        .stdout(predicate::str::contains("event_type=normalized_parquet"))
        .stdout(predicate::str::contains(
            "event_type=feature_snapshot_parquet",
        ));

    assert!(
        data_dir
            .join("parquet/events/run=all-parquet-cli/part-000000.parquet")
            .exists()
    );
    assert!(
        data_dir
            .join("parquet/features/run=all-parquet-cli/part-000000.parquet")
            .exists()
    );
}

#[test]
fn record_parquet_exports_after_normalizing_fixture_run() {
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
            "--parquet",
            "--run-id",
            "record-parquet",
            "--data-dir",
        ])
        .arg(&data_dir)
        .assert()
        .success()
        .stdout(predicate::str::contains("parquet_file="))
        .stdout(predicate::str::contains("parquet_rows=6"));

    assert!(
        data_dir
            .join("parquet/events/run=record-parquet/part-000000.parquet")
            .exists()
    );
    assert!(
        data_dir
            .join("parquet/events/run=record-parquet/schema.json")
            .exists()
    );
}

#[test]
fn fixture_live_parquet_exports_after_bounded_recording() {
    let temp = tempfile::tempdir().expect("tempdir");
    let data_dir = temp.path().join("data");

    Command::cargo_bin("hls")
        .expect("hls binary")
        .args([
            "live",
            "--symbols",
            "@107",
            "--fixture-file",
            &fixture("tests/fixtures/hyperliquid/ws_mock_live.ndjson"),
            "--once",
            "--record",
            "--parquet",
            "--run-id",
            "live-parquet",
            "--data-dir",
        ])
        .arg(&data_dir)
        .assert()
        .success()
        .stdout(predicate::str::contains("parquet_file="))
        .stdout(predicate::str::contains("parquet_rows=6"));

    assert!(
        data_dir
            .join("parquet/events/run=live-parquet/part-000000.parquet")
            .exists()
    );
    assert!(
        data_dir
            .join("parquet/events/run=live-parquet/schema.json")
            .exists()
    );
}
