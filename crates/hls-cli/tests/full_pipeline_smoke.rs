use assert_cmd::Command;
use predicates::prelude::*;

fn fixture(path: &str) -> String {
    format!("{}/../../{}", env!("CARGO_MANIFEST_DIR"), path)
}

#[test]
fn fixture_full_pipeline_smoke_covers_live_record_replay_screen_and_health() {
    let temp = tempfile::tempdir().expect("tempdir");
    let data_dir = temp.path().join("data");
    let fixture = fixture("tests/fixtures/hyperliquid/ws_mock_live.ndjson");

    Command::cargo_bin("hls")
        .expect("hls binary")
        .args([
            "live",
            "--symbols",
            "@107",
            "--fixture-file",
            &fixture,
            "--record",
            "--raw",
            "--normalized",
            "--run-id",
            "smoke",
            "--data-dir",
        ])
        .arg(&data_dir)
        .args(["--preset", "thin_books", "--once"])
        .assert()
        .success()
        .stdout(predicate::str::contains("recording run: smoke"))
        .stdout(predicate::str::contains("clean_shutdown=true"))
        .stdout(predicate::str::contains(
            "Hyperliquid Spot Microstructure Workstation",
        ))
        .stdout(predicate::str::contains("REC ●"))
        .stdout(predicate::str::contains("filter: thin_books"))
        .stdout(predicate::str::contains("mode: top-1 by tob_depth_usd asc"))
        .stdout(predicate::str::contains("Selected: @107"))
        .stdout(predicate::str::contains("@107"))
        .stdout(predicate::str::contains("No wallet"))
        .stdout(predicate::str::contains("no order routes"))
        .stdout(predicate::str::contains("private key").not());

    Command::cargo_bin("hls")
        .expect("hls binary")
        .args(["replay", "--data-dir"])
        .arg(&data_dir)
        .args(["--run-id", "smoke"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Hyperliquid Spot Microstructure Workstation",
        ))
        .stdout(predicate::str::contains(
            "filter: READ-ONLY Hyperliquid spot replay",
        ))
        .stdout(predicate::str::contains("@107"))
        .stdout(predicate::str::contains("Selected: @107"));

    Command::cargo_bin("hls")
        .expect("hls binary")
        .args(["screen", "--data-dir"])
        .arg(&data_dir)
        .args([
            "--run-id",
            "smoke",
            "--where",
            r#"symbol == "@107" and spread_bps < 60"#,
            "--sort",
            "price:desc",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "filter: symbol == \"@107\" and spread_bps < 60",
        ))
        .stdout(predicate::str::contains("mode: top-1 by price desc"))
        .stdout(predicate::str::contains("@107"));

    Command::cargo_bin("hls")
        .expect("hls binary")
        .args(["server", "--print-health", "--simulate-health", "healthy"])
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""status":"healthy""#))
        .stdout(predicate::str::contains(r#""read_only":true"#));
}
