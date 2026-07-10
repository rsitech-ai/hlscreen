use hls_core::extension::{
    ExtensionEntrypoint, ExtensionInputKind, ExtensionManifest, ExtensionOutputKind,
    ExtensionPermissions, ExtensionRuntime, ExtensionRuntimeLimits, ExtensionWasm,
};
use hls_core::{
    confidence::DataConfidenceSnapshot,
    market_state::{
        AdverseSelectionProxy, FeatureSnapshot, LiquidityResilienceState, StalenessState,
        TradeabilityState,
    },
};
use sha2::{Digest, Sha256};
use std::{fs, path::Path};

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

    let mut traversal = safe_manifest();
    traversal.wasm.path = "extensions/../outside.wasm".to_owned();
    let err = traversal
        .validate()
        .expect_err("parent path components are rejected");
    assert!(err.to_string().contains("relative"));
}

#[test]
fn extension_runtime_executes_safe_wasm_row_annotation_fixture() {
    let temp = tempfile::tempdir().expect("tempdir");
    let wasm_path = temp.path().join("extensions/safe-row-labeler.wasm");
    fs::create_dir_all(wasm_path.parent().expect("wasm parent")).expect("mkdir");
    let wasm = wat::parse_str(include_str!(
        "../../../tests/fixtures/microstructure/plugin_safe_row_labeler.wat"
    ))
    .expect("fixture wat compiles");
    fs::write(&wasm_path, &wasm).expect("write wasm");

    let mut manifest = safe_manifest();
    manifest.wasm.sha256 = sha256_hex(&wasm);

    let runtime = ExtensionRuntime::new(ExtensionRuntimeLimits {
        fuel: 25_000,
        max_input_bytes: 16 * 1024,
        max_output_bytes: 512,
    })
    .expect("runtime initializes");
    let annotations = runtime
        .invoke_row_annotations(&manifest, temp.path(), "annotate_row", &snapshot())
        .expect("plugin invocation succeeds");

    assert_eq!(annotations.len(), 1);
    assert_eq!(annotations[0].symbol, "@107");
    assert_eq!(annotations[0].label, "plugin:gap");
    assert!(annotations[0].detail.contains("read-only"));
}

#[test]
fn extension_runtime_rejects_unsafe_manifest_before_loading_wasm() {
    let manifest: ExtensionManifest = serde_json::from_str(include_str!(
        "../../../tests/fixtures/microstructure/plugin_unsafe_manifest.json"
    ))
    .expect("unsafe fixture manifest parses");
    let runtime = ExtensionRuntime::with_default_limits().expect("runtime initializes");

    let err = runtime
        .invoke_row_annotations(
            &manifest,
            Path::new("/missing"),
            "annotate_row",
            &snapshot(),
        )
        .expect_err("unsafe manifest is rejected before wasm load");

    assert!(err.to_string().contains("network access is not allowed"));
}

#[test]
fn extension_runtime_rejects_wasm_hash_mismatch() {
    let temp = tempfile::tempdir().expect("tempdir");
    let wasm_path = temp.path().join("extensions/safe-row-labeler.wasm");
    fs::create_dir_all(wasm_path.parent().expect("wasm parent")).expect("mkdir");
    fs::write(&wasm_path, b"\0asmnot-real").expect("write wasm");

    let manifest = safe_manifest();
    let runtime = ExtensionRuntime::with_default_limits().expect("runtime initializes");
    let err = runtime
        .invoke_row_annotations(&manifest, temp.path(), "annotate_row", &snapshot())
        .expect_err("hash mismatch is rejected");

    assert!(err.to_string().contains("sha256 mismatch"));
}

#[test]
fn extension_runtime_rejects_oversized_module_before_compilation() {
    let temp = tempfile::tempdir().expect("tempdir");
    let wasm_path = temp.path().join("extensions/safe-row-labeler.wasm");
    fs::create_dir_all(wasm_path.parent().expect("wasm parent")).expect("mkdir");
    fs::write(&wasm_path, vec![0_u8; 9 * 1024 * 1024]).expect("write oversized wasm");

    let manifest = safe_manifest();
    let runtime = ExtensionRuntime::with_default_limits().expect("runtime initializes");
    let err = runtime
        .invoke_row_annotations(&manifest, temp.path(), "annotate_row", &snapshot())
        .expect_err("oversized module is rejected");

    assert!(err.to_string().contains("module limit"));
}

fn safe_manifest() -> ExtensionManifest {
    serde_json::from_str(include_str!(
        "../../../tests/fixtures/microstructure/plugin_safe_manifest.json"
    ))
    .expect("safe fixture manifest parses")
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut encoded = String::from("sha256:");
    for byte in digest {
        encoded.push_str(&format!("{byte:02x}"));
    }
    encoded
}

fn snapshot() -> FeatureSnapshot {
    FeatureSnapshot {
        symbol: "@107".to_owned(),
        confidence: DataConfidenceSnapshot::new("@107"),
        price: Some(35.0),
        mid_px: Some(35.0),
        mark_px: Some(35.0),
        day_ntl_vlm: Some(1_000_000.0),
        bid_px: Some(34.9),
        bid_sz: Some(3.0),
        ask_px: Some(35.1),
        ask_sz: Some(4.0),
        spread_bps: Some(57.14),
        spread_shock_bps: None,
        spread_recovery_ms: None,
        resilience_state: LiquidityResilienceState::Unknown,
        tradeability_state: TradeabilityState::Unknown,
        fee_aware_tradeability: None,
        adverse_selection_proxy: AdverseSelectionProxy::Unknown,
        signed_notional_flow_30s: None,
        bbo_ofi_proxy_30s: None,
        microstructure_metrics: Vec::new(),
        tob_depth_usd: Some(245.1),
        tob_imbalance: Some(-0.14),
        ret_1m: None,
        ret_5m: None,
        ret_1h: None,
        rv_1m: Some(0.0),
        rv_5m: Some(0.0),
        rv_1h: Some(0.0),
        volume_z_1h: Some(0.0),
        trade_count_z_1h: Some(0.0),
        liquidity_score: 2.451,
        momentum_score: 50.0,
        mean_reversion_score: 50.0,
        score_breakdown: None,
        metadata: None,
        updated_ms_ago: Some(0),
        staleness_state: StalenessState::Fresh,
        incomplete_window_reason: None,
    }
}
