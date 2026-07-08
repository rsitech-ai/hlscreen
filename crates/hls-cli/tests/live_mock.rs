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
            "Hyperliquid Spot Microstructure Workstation",
        ))
        .stdout(predicate::str::contains("REC ready"))
        .stdout(predicate::str::contains(
            "filter: READ-ONLY Hyperliquid spot live screen",
        ))
        .stdout(predicate::str::contains("@107"))
        .stdout(predicate::str::contains("35.2000"))
        .stdout(predicate::str::contains("Selected: @107"))
        .stdout(predicate::str::contains("Bid/Ask"))
        .stdout(predicate::str::contains("No wallet"))
        .stdout(predicate::str::contains("no order routes"))
        .stdout(predicate::str::contains("private key").not());
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
            "Hyperliquid Spot Microstructure Workstation",
        ))
        .stdout(predicate::str::contains("REC ready"))
        .stdout(predicate::str::contains("filter: thin_books"))
        .stdout(predicate::str::contains("mode: top-1 by tob_depth_usd asc"))
        .stdout(predicate::str::contains("@107"))
        .stdout(predicate::str::contains("35.2000"));
}

#[test]
fn live_once_tui_uses_unified_ratatui_cockpit() {
    Command::cargo_bin("hls")
        .expect("hls binary")
        .args([
            "live",
            "--symbols",
            "@107",
            "--fixture-file",
            &fixture("tests/fixtures/hyperliquid/ws_mock_live.ndjson"),
            "--metadata-file",
            &fixture("tests/fixtures/microstructure/metadata_enrichment.json"),
            "--once",
            "--tui",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("WATCHLIST"))
        .stdout(predicate::str::contains("MICROSTRUCTURE"))
        .stdout(predicate::str::contains("CHART"))
        .stdout(predicate::str::contains("TAPE"))
        .stdout(predicate::str::contains("BOOK"))
        .stdout(predicate::str::contains("HYPE/USDC"))
        .stdout(predicate::str::contains("No wallet"))
        .stdout(predicate::str::contains("private key").not());
}
