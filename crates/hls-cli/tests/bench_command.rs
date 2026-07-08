use std::path::PathBuf;

use assert_cmd::Command;
use predicates::prelude::*;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root")
}

#[test]
fn bench_command_validates_public_fixture_pack() {
    let root = repo_root();

    Command::cargo_bin("hls")
        .expect("hls binary")
        .args(["bench", "--manifest"])
        .arg(root.join("tests/fixtures/microstructure/benchmark_gap_replay.json"))
        .args(["--repo-root"])
        .arg(&root)
        .arg("--json")
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""fixture_id": "gap_replay_v1""#))
        .stdout(predicate::str::contains(r#""matched": true"#))
        .stdout(predicate::str::contains(r#""events_read": 4"#))
        .stdout(predicate::str::contains("wallet").not())
        .stdout(predicate::str::contains("account").not())
        .stdout(predicate::str::contains("order").not());
}

#[test]
fn bench_command_rejects_missing_manifest() {
    let root = repo_root();

    Command::cargo_bin("hls")
        .expect("hls binary")
        .args(["bench", "--manifest"])
        .arg(root.join("tests/fixtures/microstructure/missing-pack.json"))
        .args(["--repo-root"])
        .arg(&root)
        .assert()
        .failure()
        .stderr(predicate::str::contains("read benchmark manifest"));
}
