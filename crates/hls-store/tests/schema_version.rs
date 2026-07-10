use hls_store::schema::{
    CURRENT_NORMALIZED_EVENT_SCHEMA_VERSION, CURRENT_PARQUET_EVENT_SCHEMA_VERSION,
    CURRENT_SQLITE_SCHEMA_VERSION, StorageSchemaManifest,
};

#[test]
fn supported_schema_manifest_validates_and_round_trips() {
    let manifest: StorageSchemaManifest = serde_json::from_str(include_str!(
        "../../../tests/fixtures/microstructure/schema_manifest_supported.json"
    ))
    .expect("supported schema fixture parses");

    manifest
        .validate_supported()
        .expect("supported schema validates");
    assert_eq!(
        manifest.normalized_event_schema_version,
        CURRENT_NORMALIZED_EVENT_SCHEMA_VERSION
    );
    assert_eq!(
        manifest.sqlite_schema_version,
        CURRENT_SQLITE_SCHEMA_VERSION
    );
    assert_eq!(
        manifest.parquet_event_schema_version,
        Some(CURRENT_PARQUET_EVENT_SCHEMA_VERSION)
    );

    let temp = tempfile::tempdir().expect("temp dir");
    let path = temp.path().join("schema.json");
    manifest
        .write_to_path(&path)
        .expect("write schema manifest");
    let decoded = StorageSchemaManifest::read_from_path(&path).expect("read schema manifest");
    assert_eq!(decoded, manifest);
    decoded
        .validate_supported()
        .expect("round-tripped schema validates");
}

#[test]
fn unsupported_schema_manifest_fails_with_actionable_version_error() {
    let manifest: StorageSchemaManifest = serde_json::from_str(include_str!(
        "../../../tests/fixtures/microstructure/schema_manifest_unsupported.json"
    ))
    .expect("unsupported schema fixture parses");

    let err = manifest
        .validate_supported()
        .expect_err("unsupported version fails closed");

    assert!(
        err.to_string()
            .contains("unsupported normalized event schema version 99")
    );
    assert!(err.to_string().contains("expected 1"));
}
