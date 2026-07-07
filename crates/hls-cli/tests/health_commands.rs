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
