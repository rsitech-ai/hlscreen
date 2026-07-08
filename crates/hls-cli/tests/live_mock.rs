use assert_cmd::Command;
use predicates::prelude::*;

fn fixture(path: &str) -> String {
    format!("{}/../../{}", env!("CARGO_MANIFEST_DIR"), path)
}

#[test]
fn live_once_renders_fixture_backed_read_only_table() {
    Command::cargo_bin("hls")
        .expect("hls binary")
        .args([
            "live",
            "--symbols",
            "@107",
            "--fixture-file",
            &fixture("tests/fixtures/hyperliquid/ws_mock_live.ndjson"),
            "--once",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "READ-ONLY Hyperliquid spot live screen",
        ))
        .stdout(predicate::str::contains("@107"))
        .stdout(predicate::str::contains("35.2000"))
        .stdout(predicate::str::contains("fresh"))
        .stdout(predicate::str::contains("wallet").not())
        .stdout(predicate::str::contains("order").not());
}

#[test]
fn live_once_applies_screen_preset_before_rendering() {
    Command::cargo_bin("hls")
        .expect("hls binary")
        .args([
            "live",
            "--symbols",
            "@107",
            "--fixture-file",
            &fixture("tests/fixtures/hyperliquid/ws_mock_live.ndjson"),
            "--preset",
            "thin_books",
            "--once",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "READ-ONLY Hyperliquid spot live screen",
        ))
        .stdout(predicate::str::contains("@107"))
        .stdout(predicate::str::contains("35.2000"));
}
