use assert_cmd::Command;
use predicates::prelude::*;

fn fixture(path: &str) -> String {
    format!("{}/../../{}", env!("CARGO_MANIFEST_DIR"), path)
}

#[test]
fn record_then_replay_fixture_without_network() {
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
            "--raw",
            "--normalized",
            "--run-id",
            "test-run",
            "--data-dir",
        ])
        .arg(&data_dir)
        .assert()
        .success()
        .stdout(predicate::str::contains("recording run: test-run"))
        .stdout(predicate::str::contains("clean_shutdown=true"));

    assert!(data_dir.join("hls.sqlite").exists());
    assert!(data_dir.join("raw").exists());
    assert!(data_dir.join("normalized").exists());

    Command::cargo_bin("hls")
        .expect("hls binary")
        .args(["replay", "--data-dir"])
        .arg(&data_dir)
        .args(["--run-id", "test-run"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Hyperliquid Spot Microstructure Workstation",
        ))
        .stdout(predicate::str::contains(
            "filter: READ-ONLY Hyperliquid spot replay",
        ))
        .stdout(predicate::str::contains("@107"))
        .stdout(predicate::str::contains("35.2000"))
        .stdout(predicate::str::contains("Selected: @107"));
}
