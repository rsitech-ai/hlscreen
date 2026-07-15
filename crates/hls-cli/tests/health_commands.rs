use assert_cmd::Command;
use predicates::prelude::*;

fn fixture(path: &str) -> String {
    format!("{}/../../{}", env!("CARGO_MANIFEST_DIR"), path)
}

#[test]
fn doctor_live_json_reports_simulated_health() {
    Command::cargo_bin("hls")
        .expect("hls binary")
        .args([
            "doctor",
            "--live",
            "--json",
            "--simulate-health",
            "writer-lag",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""status": "degraded""#))
        .stdout(predicate::str::contains(r#""read_only": true"#))
        .stdout(predicate::str::contains("writer backlog high"))
        .stdout(predicate::str::contains("wallet").not())
        .stdout(predicate::str::contains("order").not());
}

#[test]
fn doctor_live_text_renders_next_gen_health_panel() {
    let temp = tempfile::tempdir().expect("tempdir");

    Command::cargo_bin("hls")
        .expect("hls binary")
        .args([
            "doctor",
            "--live",
            "--simulate-health",
            "writer-lag",
            "--data-dir",
        ])
        .arg(temp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Operations Command Center"))
        .stdout(predicate::str::contains("DEGRADED"))
        .stdout(predicate::str::contains("SAFETY"))
        .stdout(predicate::str::contains("CONNECTION"))
        .stdout(predicate::str::contains("RECORDER"))
        .stdout(predicate::str::contains("writer backlog 250/100"))
        .stdout(predicate::str::contains("attention queue"))
        .stdout(predicate::str::contains("wallet").not())
        .stdout(predicate::str::contains("order").not());
}

#[test]
fn server_print_health_outputs_read_only_local_api_payload() {
    Command::cargo_bin("hls")
        .expect("hls binary")
        .args(["server", "--print-health", "--simulate-health", "healthy"])
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""status":"healthy""#))
        .stdout(predicate::str::contains(r#""read_only":true"#))
        .stdout(predicate::str::contains("wallet").not())
        .stdout(predicate::str::contains("order").not());
}

#[test]
fn server_rejects_non_loopback_bind_before_starting() {
    Command::cargo_bin("hls")
        .expect("hls binary")
        .args(["server", "--bind", "0.0.0.0:8787"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("loopback address"));
}

#[test]
fn server_live_fixture_publishes_read_only_market_rows() {
    Command::cargo_bin("hls")
        .expect("hls binary")
        .args([
            "server",
            "--live",
            "--bind",
            "127.0.0.1:0",
            "--symbols",
            "@107",
            "--fixture-file",
            &fixture("tests/fixtures/hyperliquid/ws_mock_live.ndjson"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("server_live_run=complete"))
        .stdout(predicate::str::contains("symbols=1"))
        .stdout(predicate::str::contains("rows=1"))
        .stdout(predicate::str::contains("market_events="))
        .stdout(predicate::str::contains("wallet").not())
        .stdout(predicate::str::contains("order").not())
        .stderr(predicate::str::contains("hls live server listening"));
}

#[test]
fn server_live_rejects_zero_refresh_interval_before_starting() {
    Command::cargo_bin("hls")
        .expect("hls binary")
        .args([
            "server",
            "--live",
            "--refresh-secs",
            "0",
            "--fixture-file",
            &fixture("tests/fixtures/hyperliquid/ws_mock_live.ndjson"),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "--refresh-secs must be greater than zero",
        ))
        .stderr(predicate::str::contains("panicked").not())
        .stderr(predicate::str::contains("listening").not());
}
