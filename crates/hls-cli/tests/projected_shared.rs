use assert_cmd::Command;
#[path = "../../hls-hyperliquid/tests/support/mfop.rs"]
mod mfop;

#[test]
fn cli_consumes_actual_typed_spot_perps_and_reports_original_identity_and_cursor_pair() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    let target = temp.path().join("target");
    mfop::record(&source, false);
    mfop::project(&source, &target);
    let output = Command::cargo_bin("hls")
        .unwrap()
        .args(["live", "--shared-feed"])
        .arg(&target)
        .args(["--once", "--color", "never"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let summary: serde_json::Value = serde_json::from_str(
        stdout
            .lines()
            .find_map(|l| l.strip_prefix("shared_feed_summary="))
            .unwrap(),
    )
    .unwrap();
    assert_eq!(summary["sealed"], true);
    assert_eq!(summary["events"]["trade"], 4);
    assert_eq!(summary["events"]["bbo"], 4);
    assert_eq!(summary["events"]["book"], 2);
    assert_eq!(summary["events"]["candle"], 2);
    assert_eq!(summary["events"]["context"], 2);
    assert_eq!(
        summary["clock_policy"],
        "max_checked_owner_approximation_physical_order_v1"
    );
    assert!(summary["availability_time_ns"].is_null());
    assert!(summary["source_cursor"]["sequence"].as_u64().unwrap() > 0);
    let instruments = summary["instruments"].as_array().unwrap();
    assert_eq!(instruments.len(), 2);
    assert!(instruments.iter().any(|i| i["display_key"] == "spot:@12"
        && i["venue"] == 21
        && i["metadata"]["key"]["native_symbol"] == "@12"));
    assert!(
        instruments
            .iter()
            .any(|i| i["display_key"] == "perps:BTC" && i["venue"] == 22)
    );
    assert_ne!(
        summary["as_of_ms"],
        i64::MAX / 1_000_000,
        "seal clock must not enter feature time"
    );
}
