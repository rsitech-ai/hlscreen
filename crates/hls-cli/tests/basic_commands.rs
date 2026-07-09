use std::fs;

use assert_cmd::Command;
use predicates::prelude::*;

fn fixture(path: &str) -> String {
    format!("{}/../../{}", env!("CARGO_MANIFEST_DIR"), path)
}

#[test]
fn version_identifies_the_ratatui_workstation_binary() {
    Command::cargo_bin("hls")
        .expect("hls binary")
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")))
        .stdout(predicate::str::contains("ratatui-workstation"));
}

#[test]
fn terminal_doctor_is_read_only_and_explains_color_detection() {
    let temp = tempfile::tempdir().expect("tempdir");
    let untouched_data_dir = temp.path().join("must-not-be-created");

    Command::cargo_bin("hls")
        .expect("hls binary")
        .args(["doctor", "--terminal", "--data-dir"])
        .arg(&untouched_data_dir)
        .env("TERM", "dumb")
        .env("COLORTERM", "truecolor")
        .env("TMUX", "/tmp/tmux-test,1,0")
        .env("NO_COLOR", "1")
        .env_remove("HLS_FORCE_COLOR")
        .env_remove("CLICOLOR_FORCE")
        .env_remove("FORCE_COLOR")
        .assert()
        .success()
        .stdout(predicate::str::contains("renderer: ratatui-workstation"))
        .stdout(predicate::str::contains("stdin tty: false"))
        .stdout(predicate::str::contains("stderr tty: false"))
        .stdout(predicate::str::contains("TERM: dumb"))
        .stdout(predicate::str::contains("COLORTERM: truecolor"))
        .stdout(predicate::str::contains("NO_COLOR: 1"))
        .stdout(predicate::str::contains("force-color override: disabled"))
        .stdout(predicate::str::contains("auto-color detection: disabled"));

    assert!(
        !untouched_data_dir.exists(),
        "terminal diagnostics must not create the data directory"
    );
}

#[test]
fn terminal_doctor_json_reports_force_color_precedence() {
    let temp = tempfile::tempdir().expect("tempdir");
    let untouched_data_dir = temp.path().join("must-not-be-created");

    Command::cargo_bin("hls")
        .expect("hls binary")
        .args(["doctor", "--terminal", "--json", "--data-dir"])
        .arg(&untouched_data_dir)
        .env("TERM", "dumb")
        .env("NO_COLOR", "1")
        .env("HLS_FORCE_COLOR", "1")
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""force_color": true"#))
        .stdout(predicate::str::contains(r#""auto_color": false"#))
        .stdout(predicate::str::contains(r#""effective_auto_color": true"#));

    assert!(
        !untouched_data_dir.exists(),
        "JSON terminal diagnostics must not create the data directory"
    );
}

#[test]
fn terminal_doctor_json_disables_auto_color_for_non_tty_stderr() {
    Command::cargo_bin("hls")
        .expect("hls binary")
        .args(["doctor", "--terminal", "--json"])
        .env("TERM", "xterm-256color")
        .env_remove("NO_COLOR")
        .env_remove("HLS_FORCE_COLOR")
        .env_remove("CLICOLOR_FORCE")
        .env_remove("FORCE_COLOR")
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""stderr_tty": false"#))
        .stdout(predicate::str::contains(r#""auto_color": false"#))
        .stdout(predicate::str::contains(r#""effective_auto_color": false"#));
}

#[test]
fn init_creates_config_under_requested_data_dir() {
    let temp = tempfile::tempdir().expect("tempdir");
    let data_dir = temp.path().join("hls-data");

    Command::cargo_bin("hls")
        .expect("hls binary")
        .args(["init", "--data-dir"])
        .arg(&data_dir)
        .assert()
        .success()
        .stdout(predicate::str::contains("read_only=true"));

    assert!(data_dir.join("config.toml").exists());
}

#[test]
fn doctor_reports_read_only_status_for_local_config() {
    let temp = tempfile::tempdir().expect("tempdir");
    let data_dir = temp.path().join("hls-data");

    Command::cargo_bin("hls")
        .expect("hls binary")
        .args(["init", "--data-dir"])
        .arg(&data_dir)
        .assert()
        .success();

    Command::cargo_bin("hls")
        .expect("hls binary")
        .args(["doctor", "--data-dir"])
        .arg(&data_dir)
        .assert()
        .success()
        .stdout(predicate::str::contains("read-only safety: ok"));
}

#[test]
fn doctor_reports_invalid_existing_config_as_safety_failure() {
    let temp = tempfile::tempdir().expect("tempdir");
    let data_dir = temp.path().join("hls-data");
    fs::create_dir_all(&data_dir).expect("data dir");
    fs::write(
        data_dir.join("config.toml"),
        r#"
[safety]
read_only = false
wallet_enabled = true
trading_enabled = true
"#,
    )
    .expect("unsafe config");

    Command::cargo_bin("hls")
        .expect("hls binary")
        .args(["doctor", "--data-dir"])
        .arg(&data_dir)
        .assert()
        .success()
        .stdout(predicate::str::contains("config readable: fail"))
        .stdout(predicate::str::contains("config error:"))
        .stdout(predicate::str::contains("read-only safety: fail"));
}

#[test]
fn symbols_prints_fixture_backed_volume_ranked_markets() {
    Command::cargo_bin("hls")
        .expect("hls binary")
        .args([
            "symbols",
            "--top",
            "2",
            "--asset-contexts-file",
            &fixture("tests/fixtures/hyperliquid/spot_meta_and_asset_ctxs.json"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("READ-ONLY"))
        .stdout(predicate::str::contains("HYPE/USDC"))
        .stdout(predicate::str::contains("@107"))
        .stdout(predicate::str::contains("PURR/USDC"));
}
