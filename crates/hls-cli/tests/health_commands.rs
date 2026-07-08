use assert_cmd::Command;
use predicates::prelude::*;

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
        .stdout(predicate::str::contains("INGEST"))
        .stdout(predicate::str::contains("STORAGE"))
        .stdout(predicate::str::contains("writer backlog: 250"))
        .stdout(predicate::str::contains("reasons requiring attention"))
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
