use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs;

fn fixture(path: &str) -> String {
    format!("{}/../../{}", env!("CARGO_MANIFEST_DIR"), path)
}

#[test]
fn extension_command_executes_safe_plugin_against_fixture_row() {
    let temp = tempfile::tempdir().expect("tempdir");
    let manifest_path = write_safe_plugin_fixture(temp.path());

    Command::cargo_bin("hls")
        .expect("hls binary")
        .args([
            "extension",
            "--manifest",
            manifest_path.to_str().expect("utf8 manifest path"),
            "--fixture-file",
            &fixture("tests/fixtures/hyperliquid/ws_mock_live.ndjson"),
            "--symbol",
            "@107",
            "--json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "\"manifest\": \"safe-row-labeler\"",
        ))
        .stdout(predicate::str::contains("\"entrypoint\": \"annotate_row\""))
        .stdout(predicate::str::contains("\"label\": \"plugin:gap\""))
        .stdout(predicate::str::contains("\"read_only\": true"))
        .stdout(predicate::str::contains("wallet").not())
        .stdout(predicate::str::contains("exchange_action").not())
        .stdout(predicate::str::contains("place trade").not());
}

#[test]
fn extension_command_rejects_unsafe_manifest_before_loading_wasm() {
    Command::cargo_bin("hls")
        .expect("hls binary")
        .args([
            "extension",
            "--manifest",
            &fixture("tests/fixtures/microstructure/plugin_unsafe_manifest.json"),
            "--fixture-file",
            &fixture("tests/fixtures/hyperliquid/ws_mock_live.ndjson"),
            "--symbol",
            "@107",
            "--json",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("network access is not allowed"));
}

fn write_safe_plugin_fixture(temp: &std::path::Path) -> std::path::PathBuf {
    let extensions_dir = temp.join("extensions");
    fs::create_dir_all(&extensions_dir).expect("create extensions dir");
    let wasm = wat::parse_str(include_str!(
        "../../../tests/fixtures/microstructure/plugin_safe_row_labeler.wat"
    ))
    .expect("fixture wat compiles");
    fs::write(extensions_dir.join("safe-row-labeler.wasm"), &wasm).expect("write wasm");

    let mut manifest: Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/microstructure/plugin_safe_manifest.json"
    ))
    .expect("safe manifest parses");
    manifest["wasm"]["sha256"] = Value::String(sha256_hex(&wasm));
    let manifest_path = temp.join("plugin_safe_manifest.json");
    fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&manifest).expect("manifest serializes"),
    )
    .expect("write manifest");
    manifest_path
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut encoded = String::from("sha256:");
    for byte in digest {
        encoded.push_str(&format!("{byte:02x}"));
    }
    encoded
}
