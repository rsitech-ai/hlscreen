use assert_cmd::Command;
use predicates::prelude::*;

fn fixture(path: &str) -> String {
    format!("{}/../../{}", env!("CARGO_MANIFEST_DIR"), path)
}

#[test]
fn explain_renders_why_ranked_pane_from_recorded_replay_data() {
    let temp = tempfile::tempdir().expect("tempdir");
    let data_dir = temp.path().join("data");

    Command::cargo_bin("hls")
        .expect("hls binary")
        .args([
            "record",
            "--symbols",
            "@107",
            "--fixture-file",
            &fixture("tests/fixtures/microstructure/resilience_shock.ndjson"),
            "--normalized",
            "--run-id",
            "explain-run",
            "--data-dir",
        ])
        .arg(&data_dir)
        .assert()
        .success();

    Command::cargo_bin("hls")
        .expect("hls binary")
        .args(["explain", "--data-dir"])
        .arg(&data_dir)
        .args(["--run-id", "explain-run", "--symbol", "@107"])
        .assert()
        .success()
        .stdout(predicate::str::contains("WHY RANKED"))
        .stdout(predicate::str::contains("liquidity_resilience"))
        .stdout(predicate::str::contains("spread_cost"))
        .stdout(predicate::str::contains("confidence 100"))
        .stdout(predicate::str::contains("screen heuristic, not advice"))
        .stdout(predicate::str::contains("private key").not());
}

#[test]
fn explain_can_emit_json_score_breakdown() {
    Command::cargo_bin("hls")
        .expect("hls binary")
        .args([
            "explain",
            "--fixture-file",
            &fixture("tests/fixtures/microstructure/resilience_shock.ndjson"),
            "--symbol",
            "@107",
            "--json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"score_breakdown\""))
        .stdout(predicate::str::contains("\"liquidity_resilience\""))
        .stdout(predicate::str::contains("\"signed_contribution\""))
        .stdout(predicate::str::contains("\"unavailable_evidence\""));
}
