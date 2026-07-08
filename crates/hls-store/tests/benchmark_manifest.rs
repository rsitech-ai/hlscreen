use hls_store::benchmark::BenchmarkManifest;

#[test]
fn benchmark_manifest_parses_and_validates_public_fixture_pack() {
    let manifest = BenchmarkManifest::from_json(
        r#"{
          "schema_version": 1,
          "fixture_id": "gap_replay_v1",
          "description": "Public reconnect gap replay fixture",
          "input_files": ["tests/fixtures/microstructure/gap_replay.ndjson"],
          "expected_hash": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
          "max_feature_latency_us": 1000,
          "tags": ["public", "gap", "replay"]
        }"#,
    )
    .expect("manifest parses");

    manifest.validate().expect("manifest is valid");
    assert_eq!(manifest.fixture_id, "gap_replay_v1");
    assert!(manifest.tags.iter().any(|tag| tag == "public"));
}

#[test]
fn benchmark_manifest_rejects_absolute_or_private_inputs() {
    let manifest = BenchmarkManifest::from_json(
        r#"{
          "schema_version": 1,
          "fixture_id": "unsafe_private_fixture",
          "description": "Should fail",
          "input_files": ["/tmp/private-account.ndjson"],
          "expected_hash": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
          "max_feature_latency_us": 1000,
          "tags": ["private"]
        }"#,
    )
    .expect("manifest parses");

    let err = manifest
        .validate()
        .expect_err("absolute private fixture inputs are rejected");

    assert!(err.to_string().contains("relative public fixture"));
}
