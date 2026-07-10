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
            "Hyperliquid Spot Microstructure Workstation",
        ))
        .stdout(predicate::str::contains(
            "filter: symbol == \"@107\" and spread_bps < 60",
        ))
        .stdout(predicate::str::contains("mode: top-1 by price desc"))
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
        .stdout(predicate::str::contains("filter: thin_books"))
        .stdout(predicate::str::contains("mode: top-1 by tob_depth_usd asc"))
        .stdout(predicate::str::contains("@107"));
}

#[test]
fn screen_applies_explicit_fee_profile_file_to_fee_aware_fields() {
    Command::cargo_bin("hls")
        .expect("hls binary")
        .args([
            "screen",
            "--fixture-file",
            &fixture("tests/fixtures/microstructure/resilience_shock.ndjson"),
            "--fee-profile-file",
            &fixture("tests/fixtures/microstructure/fee_profile_high_cost.json"),
            "--where",
            r#"fee_tradeability_state == "costly" and fee_profile == "manual-high-fee""#,
            "--sort",
            "fee_expected_round_trip_cost_bps:desc",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("filter: fee_tradeability_state"))
        .stdout(predicate::str::contains(
            "mode: top-1 by fee_expected_round_trip_cost_bps desc",
        ))
        .stdout(predicate::str::contains("@107"))
        .stdout(predicate::str::contains("No wallet"))
        .stdout(predicate::str::contains("order routes"))
        .stdout(predicate::str::contains("private key").not());
}

#[test]
fn screen_applies_blended_maker_taker_fee_profile_file() {
    Command::cargo_bin("hls")
        .expect("hls binary")
        .args([
            "screen",
            "--fixture-file",
            &fixture("tests/fixtures/microstructure/resilience_shock.ndjson"),
            "--fee-profile-file",
            &fixture("tests/fixtures/microstructure/fee_profile_blended.json"),
            "--where",
            r#"fee_tradeability_state == "tradeable" and fee_profile == "manual-blended-fee" and fee_expected_round_trip_cost_bps < 60"#,
            "--sort",
            "fee_expected_round_trip_cost_bps:asc",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("filter: fee_tradeability_state"))
        .stdout(predicate::str::contains("@107"))
        .stdout(predicate::str::contains("No wallet, no private streams"))
        .stdout(predicate::str::contains("exchange_action").not());
}

#[test]
fn screen_rejects_invalid_fee_profile_file() {
    Command::cargo_bin("hls")
        .expect("hls binary")
        .args([
            "screen",
            "--fixture-file",
            &fixture("tests/fixtures/microstructure/resilience_shock.ndjson"),
            "--fee-profile-file",
            &fixture("tests/fixtures/microstructure/fee_profile_invalid_thresholds.json"),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "fee profile thresholds must be ordered tradeable <= costly",
        ));
}
