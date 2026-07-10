use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;

fn fixture(path: &str) -> String {
    format!("{}/../../{}", env!("CARGO_MANIFEST_DIR"), path)
}

#[test]
fn alerts_command_emits_local_only_json_from_fixture_rows() {
    Command::cargo_bin("hls")
        .expect("hls binary")
        .args([
            "alerts",
            "--fixture-file",
            &fixture("tests/fixtures/microstructure/thin_brittle_book.ndjson"),
            "--symbol",
            "@107",
            "--min-spread-shock-bps",
            "250",
            "--max-confidence-score",
            "100",
            "--json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"events\""))
        .stdout(predicate::str::contains(
            "\"rule_id\": \"shock-low-confidence\"",
        ))
        .stdout(predicate::str::contains("\"action\": \"local_only\""))
        .stdout(predicate::str::contains("spread shock"))
        .stdout(predicate::str::contains("order").not())
        .stdout(predicate::str::contains("wallet").not())
        .stdout(predicate::str::contains("exchange_action").not());
}

#[test]
fn alerts_command_keeps_default_low_confidence_threshold_conservative() {
    Command::cargo_bin("hls")
        .expect("hls binary")
        .args([
            "alerts",
            "--fixture-file",
            &fixture("tests/fixtures/microstructure/thin_brittle_book.ndjson"),
            "--symbol",
            "@107",
            "--json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"events\": []"))
        .stdout(predicate::str::contains("\"suppressed\": []"));
}

#[test]
fn alerts_command_loads_local_playbook_file() {
    Command::cargo_bin("hls")
        .expect("hls binary")
        .args([
            "alerts",
            "--fixture-file",
            &fixture("tests/fixtures/microstructure/thin_brittle_book.ndjson"),
            "--symbol",
            "@107",
            "--playbook-file",
            &fixture("tests/fixtures/microstructure/alert_playbook_spread_watch.json"),
            "--json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "\"playbook_id\": \"operator-spread-watch\"",
        ))
        .stdout(predicate::str::contains("\"rule_id\": \"custom-shock\""))
        .stdout(predicate::str::contains("\"action\": \"local_only\""))
        .stdout(predicate::str::contains("exchange_action").not())
        .stdout(predicate::str::contains("wallet").not());
}

#[test]
fn alerts_command_loads_threshold_grammar_playbook_file() {
    Command::cargo_bin("hls")
        .expect("hls binary")
        .args([
            "alerts",
            "--fixture-file",
            &fixture("tests/fixtures/microstructure/thin_brittle_book.ndjson"),
            "--symbol",
            "@107",
            "--playbook-file",
            &fixture("tests/fixtures/microstructure/alert_playbook_threshold_watch.json"),
            "--json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "\"playbook_id\": \"operator-threshold-watch\"",
        ))
        .stdout(predicate::str::contains(
            "\"rule_id\": \"shock-threshold-stack\"",
        ))
        .stdout(predicate::str::contains("spread_shock_bps"))
        .stdout(predicate::str::contains("confidence_score"))
        .stdout(predicate::str::contains("\"action\": \"local_only\""))
        .stdout(predicate::str::contains("exchange_action").not())
        .stdout(predicate::str::contains("wallet").not());
}

#[test]
fn alerts_command_loads_boolean_grammar_playbook_file() {
    Command::cargo_bin("hls")
        .expect("hls binary")
        .args([
            "alerts",
            "--fixture-file",
            &fixture("tests/fixtures/microstructure/thin_brittle_book.ndjson"),
            "--symbol",
            "@107",
            "--playbook-file",
            &fixture("tests/fixtures/microstructure/alert_playbook_boolean_watch.json"),
            "--json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "\"playbook_id\": \"operator-boolean-watch\"",
        ))
        .stdout(predicate::str::contains(
            "\"rule_id\": \"shock-or-wide-not-low-confidence\"",
        ))
        .stdout(predicate::str::contains("any("))
        .stdout(predicate::str::contains("not("))
        .stdout(predicate::str::contains("\"action\": \"local_only\""))
        .stdout(predicate::str::contains("exchange_action").not())
        .stdout(predicate::str::contains("wallet").not());
}

#[test]
fn alerts_command_rejects_exchange_action_playbook_file() {
    let temp = tempfile::tempdir().expect("tempdir");
    let playbook_path = temp.path().join("unsafe-playbook.json");
    fs::write(
        &playbook_path,
        r#"{
  "schema_version": 1,
  "id": "unsafe",
  "description": "Unsafe playbook.",
  "rules": [
    {
      "id": "unsafe-action",
      "description": "Must fail.",
      "severity": "critical",
      "condition": {
        "type": "spread_shock_and_low_confidence",
        "min_spread_shock_bps": 1.0,
        "max_confidence_score": 100
      },
      "cooldown_ms": 0,
      "source_interval_ms": 30000,
      "action": "exchange_action"
    }
  ]
}"#,
    )
    .expect("write unsafe playbook");

    Command::cargo_bin("hls")
        .expect("hls binary")
        .args([
            "alerts",
            "--fixture-file",
            &fixture("tests/fixtures/microstructure/thin_brittle_book.ndjson"),
            "--symbol",
            "@107",
            "--playbook-file",
            playbook_path.to_str().expect("utf8 playbook path"),
            "--json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("exchange actions are not allowed"));
}

#[test]
fn alerts_command_writes_local_history_jsonl() {
    let temp = tempfile::tempdir().expect("tempdir");
    let history_path = temp.path().join("nested").join("alerts.jsonl");

    Command::cargo_bin("hls")
        .expect("hls binary")
        .args([
            "alerts",
            "--fixture-file",
            &fixture("tests/fixtures/microstructure/thin_brittle_book.ndjson"),
            "--symbol",
            "@107",
            "--playbook-file",
            &fixture("tests/fixtures/microstructure/alert_playbook_threshold_watch.json"),
            "--alert-history-file",
            history_path.to_str().expect("utf8 history path"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("alerts local_only events=1"));

    let history = fs::read_to_string(history_path).expect("history file");
    assert!(history.contains(r#""kind":"event""#));
    assert!(history.contains(r#""playbook_id":"operator-threshold-watch""#));
    assert!(history.contains(r#""rule_id":"shock-threshold-stack""#));
    assert!(history.contains(r#""action":"local_only""#));
    assert!(!history.contains("exchange_action"));
    assert!(!history.contains("wallet"));
}

#[test]
fn alerts_command_lists_local_history_jsonl() {
    let temp = tempfile::tempdir().expect("tempdir");
    let history_path = temp.path().join("alerts.jsonl");

    Command::cargo_bin("hls")
        .expect("hls binary")
        .args([
            "alerts",
            "--fixture-file",
            &fixture("tests/fixtures/microstructure/thin_brittle_book.ndjson"),
            "--symbol",
            "@107",
            "--playbook-file",
            &fixture("tests/fixtures/microstructure/alert_playbook_threshold_watch.json"),
            "--alert-history-file",
            history_path.to_str().expect("utf8 history path"),
        ])
        .assert()
        .success();

    Command::cargo_bin("hls")
        .expect("hls binary")
        .args([
            "alerts",
            "--history-file",
            history_path.to_str().expect("utf8 history path"),
            "--limit",
            "1",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("alert_history records=1"))
        .stdout(predicate::str::contains("operator-threshold-watch"))
        .stdout(predicate::str::contains("shock-threshold-stack"))
        .stdout(predicate::str::contains("@107"))
        .stdout(predicate::str::contains("local_only"))
        .stdout(predicate::str::contains("exchange_action").not())
        .stdout(predicate::str::contains("wallet").not());

    Command::cargo_bin("hls")
        .expect("hls binary")
        .args([
            "alerts",
            "--history-file",
            history_path.to_str().expect("utf8 history path"),
            "--symbol",
            "missing-symbol",
            "--json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"records\": []"));
}
