use assert_cmd::Command;
use predicates::prelude::*;

fn fixture(path: &str) -> String {
    format!("{}/../../{}", env!("CARGO_MANIFEST_DIR"), path)
}

#[test]
fn analog_command_emits_replay_backed_matches_as_json() {
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
            "analog-cli",
            "--data-dir",
        ])
        .arg(&data_dir)
        .assert()
        .success();

    Command::cargo_bin("hls")
        .expect("hls binary")
        .args([
            "analog",
            "--data-dir",
            data_dir.to_str().expect("utf8 temp path"),
            "--run-id",
            "analog-cli",
            "--symbol",
            "@107",
            "--min-candidates",
            "1",
            "--json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"target_symbol\": \"@107\""))
        .stdout(predicate::str::contains("\"matches\""))
        .stdout(predicate::str::contains("\"drivers\""))
        .stdout(predicate::str::contains("\"insufficient_evidence\": null"))
        .stdout(predicate::str::contains("order").not())
        .stdout(predicate::str::contains("wallet").not())
        .stdout(predicate::str::contains("exchange_action").not());
}

#[test]
fn analog_command_reports_insufficient_evidence_instead_of_fabricating_matches() {
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
            "analog-sparse",
            "--data-dir",
        ])
        .arg(&data_dir)
        .assert()
        .success();

    Command::cargo_bin("hls")
        .expect("hls binary")
        .args([
            "analog",
            "--data-dir",
            data_dir.to_str().expect("utf8 temp path"),
            "--run-id",
            "analog-sparse",
            "--symbol",
            "@107",
            "--min-candidates",
            "999",
            "--json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"matches\": []"))
        .stdout(predicate::str::contains("\"insufficient_evidence\""));
}

#[test]
fn analog_command_writes_and_reuses_local_index() {
    let temp = tempfile::tempdir().expect("tempdir");
    let data_dir = temp.path().join("data");
    let index_path = temp.path().join("analog-index.json");

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
            "analog-indexed",
            "--data-dir",
        ])
        .arg(&data_dir)
        .assert()
        .success();

    Command::cargo_bin("hls")
        .expect("hls binary")
        .args([
            "analog",
            "--data-dir",
            data_dir.to_str().expect("utf8 temp path"),
            "--run-id",
            "analog-indexed",
            "--symbol",
            "@107",
            "--write-index",
            index_path.to_str().expect("utf8 index path"),
            "--min-candidates",
            "1",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("analog_index_file="))
        .stdout(predicate::str::contains("read_only=true"));

    let index = std::fs::read_to_string(&index_path).expect("index file");
    assert!(index.contains(r#""schema_version": 1"#));
    assert!(index.contains(r#""source_run_id": "analog-indexed""#));
    assert!(index.contains(r#""target_symbol": "@107""#));

    Command::cargo_bin("hls")
        .expect("hls binary")
        .args([
            "analog",
            "--index-file",
            index_path.to_str().expect("utf8 index path"),
            "--run-id",
            "ignored-when-index-file-is-used",
            "--symbol",
            "ignored-when-index-file-is-used",
            "--min-candidates",
            "1",
            "--json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"run_id\": \"analog-indexed\""))
        .stdout(predicate::str::contains("\"target_symbol\": \"@107\""))
        .stdout(predicate::str::contains("\"matches\""))
        .stdout(predicate::str::contains("wallet").not())
        .stdout(predicate::str::contains("exchange_action").not());
}
