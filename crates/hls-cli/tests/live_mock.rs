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
    let assert = Command::cargo_bin("hls")
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
        .success();
    let output = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout is utf8");

    assert!(output.contains("\u{1b}["));
    assert!(
        output.contains("Hyperliquid Spot Microstructure Workstation")
            || output.contains("LAYOUT DIRECTOR"),
        "fixture TUI should render the adaptive Ratatui workstation shell"
    );
    assert!(output.contains("layout "));
    assert!(output.contains("resize-safe"));
    assert!(output.contains("STATUS"));
    assert!(output.contains("REC ready"));
    assert!(output.contains("WATCHLIST"));
    assert!(output.contains("ALGO SCAN"));
    assert!(output.contains("DETAIL") || output.contains("MICROSTRUCTURE"));
    assert!(output.contains("BBO"));
    assert!(output.contains("HYPE/USDC"));
    assert!(output.contains("read-only"));
    assert!(!output.contains("private key"));
}
