use std::fs;

use assert_cmd::Command;
use predicates::prelude::*;

fn fixture(path: &str) -> String {
    format!("{}/../../{}", env!("CARGO_MANIFEST_DIR"), path)
}

fn assert_unified_ratatui_cockpit_output(output: &str) {
    assert!(output.contains("\u{1b}["));
    assert!(
        output.contains("Hyperliquid Spot Microstructure Workstation")
            || output.contains("LAYOUT DIRECTOR")
            || output.contains("RATATUI"),
        "fixture TUI should render the adaptive Ratatui workstation shell"
    );
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
fn fixture_recording_runs_the_requested_gap_backfill_closeout_hook() {
    let temp = tempfile::tempdir().expect("tempdir");

    Command::cargo_bin("hls")
        .expect("hls binary")
        .args([
            "live",
            "--fixture-file",
            &fixture("tests/fixtures/hyperliquid/ws_mock_live.ndjson"),
            "--once",
            "--record",
            "--normalized",
            "--backfill-gaps",
            "--run-id",
            "fixture-backfill-closeout",
            "--data-dir",
        ])
        .arg(temp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("backfill_run=complete"))
        .stdout(predicate::str::contains("gaps_examined=0"))
        .stdout(predicate::str::contains("requests_failed=0"))
        .stdout(predicate::str::contains("tick_gaps_recovered=0"));
}

#[test]
fn fixture_tui_renders_local_playbook_alert_history() {
    Command::cargo_bin("hls")
        .expect("hls binary")
        .args([
            "tui",
            "--symbols",
            "@107",
            "--fixture-file",
            &fixture("tests/fixtures/microstructure/resilience_shock.ndjson"),
            "--alert-playbook-file",
            &fixture("tests/fixtures/microstructure/alert_playbook_tui_watch.json"),
            "--once",
            "--color",
            "never",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("LOCAL ALERTS 1"))
        .stdout(predicate::str::contains("spread-expansion"))
        .stdout(predicate::str::contains("@107"))
        .stdout(predicate::str::contains("local-only"))
        .stdout(predicate::str::contains("exchange_action").not());
}

#[test]
fn live_rejects_alert_playbook_without_explicit_tui_surface() {
    Command::cargo_bin("hls")
        .expect("hls binary")
        .args([
            "live",
            "--fixture-file",
            &fixture("tests/fixtures/microstructure/resilience_shock.ndjson"),
            "--alert-playbook-file",
            &fixture("tests/fixtures/microstructure/alert_playbook_tui_watch.json"),
            "--once",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "requires the explicit TUI surface",
        ));
}

#[test]
fn tui_rejects_exchange_action_playbook_before_fixture_processing() {
    let temp = tempfile::tempdir().expect("tempdir");
    let playbook_path = temp.path().join("unsafe-playbook.json");
    fs::write(
        &playbook_path,
        r#"{
  "schema_version": 1,
  "id": "unsafe",
  "description": "Must be rejected.",
  "rules": [{
    "id": "exchange-action",
    "description": "Must never run.",
    "severity": "critical",
    "condition": {
      "type": "field_threshold",
      "field": "spread_bps",
      "op": "gte",
      "value": 1.0
    },
    "cooldown_ms": 0,
    "source_interval_ms": 1000,
    "action": "exchange_action"
  }]
}"#,
    )
    .expect("write unsafe playbook");

    Command::cargo_bin("hls")
        .expect("hls binary")
        .args([
            "tui",
            "--fixture-file",
            "/definitely/not/read.ndjson",
            "--alert-playbook-file",
            playbook_path.to_str().expect("utf8 playbook path"),
            "--once",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("exchange actions are not allowed"))
        .stderr(predicate::str::contains("read /definitely/not/read").not());
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

    assert_unified_ratatui_cockpit_output(&output);
}

#[test]
fn tui_command_once_uses_unified_ratatui_cockpit_without_extra_flags() {
    let assert = Command::cargo_bin("hls")
        .expect("hls binary")
        .args([
            "tui",
            "--symbols",
            "@107",
            "--fixture-file",
            &fixture("tests/fixtures/hyperliquid/ws_mock_live.ndjson"),
            "--metadata-file",
            &fixture("tests/fixtures/microstructure/metadata_enrichment.json"),
            "--once",
        ])
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
    let output = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout is utf8");

    assert_unified_ratatui_cockpit_output(&output);
}

#[test]
fn tui_command_once_auto_color_omits_ansi_for_redirected_stdout() {
    let assert = Command::cargo_bin("hls")
        .expect("hls binary")
        .args([
            "tui",
            "--symbols",
            "@107",
            "--fixture-file",
            &fixture("tests/fixtures/hyperliquid/ws_mock_live.ndjson"),
            "--once",
            "--color",
            "auto",
        ])
        .env("TERM", "xterm-256color")
        .env_remove("NO_COLOR")
        .env_remove("HLS_FORCE_COLOR")
        .env_remove("CLICOLOR_FORCE")
        .env_remove("FORCE_COLOR")
        .assert()
        .success();
    let output = String::from_utf8(assert.get_output().stdout.clone()).expect("stdout is utf8");

    assert!(output.contains("RATATUI"));
    assert!(!output.contains("\u{1b}["));
}

#[test]
fn live_once_allows_zero_duration_fixture_without_tty() {
    Command::cargo_bin("hls")
        .expect("hls binary")
        .args([
            "live",
            "--symbols",
            "@107",
            "--fixture-file",
            &fixture("tests/fixtures/hyperliquid/ws_mock_live.ndjson"),
            "--once",
            "--duration-secs",
            "0",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("@107"));
}

#[test]
fn live_rejects_oversized_boolean_filter_chains_without_stack_overflow() {
    for operator in ["and", "or"] {
        // 4096 clauses keep the argument under Linux's 128 KiB per-argument
        // limit (MAX_ARG_STRLEN) while still far exceeding the parser's
        // 256-boolean-operator complexity guard.
        let separator = format!(" {operator} ");
        let filter = std::iter::repeat_n("price > 0", 4_096)
            .collect::<Vec<_>>()
            .join(&separator);

        Command::cargo_bin("hls")
            .expect("hls binary")
            .args([
                "live",
                "--symbols",
                "@107",
                "--fixture-file",
                &fixture("tests/fixtures/hyperliquid/ws_mock_live.ndjson"),
                "--once",
                "--where",
                &filter,
                "--color",
                "never",
            ])
            .assert()
            .failure()
            .stderr(predicate::str::contains("filter complexity exceeds"))
            .stderr(predicate::str::contains("overflowed its stack").not());
    }
}
