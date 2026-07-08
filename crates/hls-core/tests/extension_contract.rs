use hls_core::extension::{
    ExtensionEntrypoint, ExtensionInputKind, ExtensionManifest, ExtensionOutputKind,
    ExtensionPermissions, ExtensionWasm,
};

fn hash() -> String {
    "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_owned()
}

#[test]
fn extension_manifest_accepts_read_only_wasm_contract() {
    let manifest = ExtensionManifest {
        schema_version: 1,
        name: "gap-labeler".to_owned(),
        version: "0.1.0".to_owned(),
        description: "Annotates rows with local replay-gap context.".to_owned(),
        wasm: ExtensionWasm {
            path: "extensions/gap-labeler.wasm".to_owned(),
            sha256: hash(),
            memory_max_pages: 16,
        },
        permissions: ExtensionPermissions::read_only(),
        entrypoints: vec![ExtensionEntrypoint::new(
            "annotate_row",
            ExtensionInputKind::FeatureSnapshot,
            ExtensionOutputKind::RowAnnotations,
        )],
    };

    manifest.validate().expect("manifest is valid");

    let encoded = serde_json::to_string(&manifest).expect("manifest serializes");
    assert!(encoded.contains("gap-labeler"));
    assert!(!encoded.contains("wallet"));
    assert!(!encoded.contains("order"));
}

#[test]
fn extension_manifest_rejects_network_filesystem_private_or_trading_permissions() {
    let mut permissions = ExtensionPermissions::read_only();
    permissions.network = true;
    permissions.filesystem = true;
    permissions.private_data = true;
    permissions.trading = true;
    permissions.allowed_hosts.push("api.example.com".to_owned());
    permissions.allowed_paths.push("/tmp".to_owned());

    let manifest = ExtensionManifest {
        schema_version: 1,
        name: "unsafe-extension".to_owned(),
        version: "0.1.0".to_owned(),
        description: "Should fail.".to_owned(),
        wasm: ExtensionWasm {
            path: "extensions/unsafe.wasm".to_owned(),
            sha256: hash(),
            memory_max_pages: 16,
        },
        permissions,
        entrypoints: vec![ExtensionEntrypoint::new(
            "annotate_row",
            ExtensionInputKind::FeatureSnapshot,
            ExtensionOutputKind::RowAnnotations,
        )],
    };

    let err = manifest
        .validate()
        .expect_err("unsafe permissions are rejected");

    assert!(err.to_string().contains("network access is not allowed"));
}

#[test]
fn extension_manifest_requires_relative_wasm_path_and_hash() {
    let manifest = ExtensionManifest {
        schema_version: 1,
        name: "bad-path".to_owned(),
        version: "0.1.0".to_owned(),
        description: "Should fail.".to_owned(),
        wasm: ExtensionWasm {
            path: "/tmp/bad.wasm".to_owned(),
            sha256: "missing-prefix".to_owned(),
            memory_max_pages: 16,
        },
        permissions: ExtensionPermissions::read_only(),
        entrypoints: vec![ExtensionEntrypoint::new(
            "annotate_row",
            ExtensionInputKind::FeatureSnapshot,
            ExtensionOutputKind::RowAnnotations,
        )],
    };

    let err = manifest
        .validate()
        .expect_err("absolute wasm path is rejected");

    assert!(err.to_string().contains("relative"));
}
