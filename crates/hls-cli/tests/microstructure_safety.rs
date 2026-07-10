use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn top_level_help_does_not_expose_private_or_order_capable_commands() {
    Command::cargo_bin("hls")
        .expect("hls binary")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("wallet").not())
        .stdout(predicate::str::contains("private-key").not())
        .stdout(predicate::str::contains("place-order").not())
        .stdout(predicate::str::contains("cancel-order").not())
        .stdout(predicate::str::contains("withdraw").not())
        .stdout(predicate::str::contains("exchange").not());
}

#[test]
fn live_help_keeps_microstructure_fields_read_only() {
    Command::cargo_bin("hls")
        .expect("hls binary")
        .args(["live", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--all-symbols"))
        .stdout(predicate::str::contains("--tui"))
        .stdout(predicate::str::contains("private").not())
        .stdout(predicate::str::contains("order").not())
        .stdout(predicate::str::contains("wallet").not());
}

#[test]
fn tui_is_a_first_class_read_only_command() {
    Command::cargo_bin("hls")
        .expect("hls binary")
        .args(["tui", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--all-symbols"))
        .stdout(predicate::str::contains("--top"))
        .stdout(predicate::str::contains("--duration-secs"))
        .stdout(predicate::str::contains("--refresh-secs"))
        .stdout(predicate::str::contains("private").not())
        .stdout(predicate::str::contains("order").not())
        .stdout(predicate::str::contains("wallet").not());
}
