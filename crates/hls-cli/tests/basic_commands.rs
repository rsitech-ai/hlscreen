use std::fs;

use assert_cmd::Command;
use predicates::prelude::*;

fn fixture(path: &str) -> String {
    format!("{}/../../{}", env!("CARGO_MANIFEST_DIR"), path)
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
