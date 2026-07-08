use assert_cmd::Command;
use predicates::prelude::*;

fn fixture(path: &str) -> String {
    format!("{}/../../{}", env!("CARGO_MANIFEST_DIR"), path)
}

#[test]
fn screen_filters_fixture_rows_with_custom_rule() {
    Command::cargo_bin("hls")
        .expect("hls binary")
        .args([
            "screen",
            "--fixture-file",
            &fixture("tests/fixtures/hyperliquid/ws_mock_live.ndjson"),
            "--where",
            r#"symbol == "@107" and spread_bps < 60"#,
            "--sort",
            "price:desc",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "READ-ONLY Hyperliquid spot screen",
        ))
        .stdout(predicate::str::contains("@107"))
        .stdout(predicate::str::contains("35.2000"))
        .stdout(predicate::str::contains("No wallet"))
        .stdout(predicate::str::contains("no order routes"))
        .stdout(predicate::str::contains("private key").not());
}

#[test]
fn screen_preset_uses_shared_rule_engine() {
    Command::cargo_bin("hls")
        .expect("hls binary")
        .args([
            "screen",
            "--fixture-file",
            &fixture("tests/fixtures/hyperliquid/ws_mock_live.ndjson"),
            "--preset",
            "thin_books",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("@107"));
}
