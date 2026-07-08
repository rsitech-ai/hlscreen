use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn doctor_live_json_includes_low_cardinality_metrics_snapshot() {
    let temp = tempfile::tempdir().expect("tempdir");

    Command::cargo_bin("hls")
        .expect("hls binary")
        .args([
            "doctor",
            "--live",
            "--json",
            "--simulate-health",
            "writer-lag",
            "--data-dir",
        ])
        .arg(temp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""metrics""#))
        .stdout(predicate::str::contains("hls_read_only_safety_ok"))
        .stdout(predicate::str::contains("hls_health_status"))
        .stdout(predicate::str::contains("hls_writer_backlog_events"))
        .stdout(predicate::str::contains("# HELP hls_read_only_safety_ok"))
        .stdout(predicate::str::contains("symbol").not())
        .stdout(predicate::str::contains("run_id").not())
        .stdout(predicate::str::contains("wallet").not())
        .stdout(predicate::str::contains("account").not())
        .stdout(predicate::str::contains("order").not());
}
