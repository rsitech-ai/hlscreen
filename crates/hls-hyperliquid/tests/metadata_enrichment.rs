use hls_core::metadata::{COHORT_FRESH_LIQUIDITY, COHORT_NEW_LISTING, COHORT_UNKNOWN_METADATA};
use hls_hyperliquid::rest::{
    metadata_enrichments_from_public_info, parse_metadata_enrichment_bundle, parse_token_details,
};
use serde_json::Value;
use std::collections::HashMap;

#[test]
fn parses_public_metadata_bundle_into_complete_and_partial_enrichments() {
    let metadata = parse_metadata_enrichment_bundle(include_str!(
        "../../../tests/fixtures/microstructure/metadata_enrichment.json"
    ))
    .expect("metadata fixture parses");

    let hype = metadata
        .iter()
        .find(|metadata| metadata.feed_identifier == "@107")
        .expect("HYPE metadata exists");
    let partial = metadata
        .iter()
        .find(|metadata| metadata.feed_identifier == "@404")
        .expect("partial metadata exists");

    assert_eq!(metadata.len(), 2);
    assert_eq!(hype.display_name, "HYPE/USDC");
    assert_eq!(
        hype.deployer.as_deref(),
        Some("0x1234567890abcdef1234567890abcdef12345678")
    );
    assert_eq!(hype.seeded_usdc, Some(1_250_000.0));
    assert_eq!(hype.max_supply, Some(1_000_000_000.0));
    assert_eq!(hype.circulating_supply, Some(100_000_000.0));
    assert!(hype.has_tag(COHORT_NEW_LISTING));
    assert!(hype.has_tag(COHORT_FRESH_LIQUIDITY));
    assert!(!hype.has_tag(COHORT_UNKNOWN_METADATA));

    assert_eq!(partial.display_name, "PARTIAL/USDC");
    assert_eq!(partial.deployer, None);
    assert!(partial.has_tag(COHORT_UNKNOWN_METADATA));
    assert!(partial.unknown_fields.contains(&"deployer".to_owned()));
    assert!(partial.unknown_fields.contains(&"seeded_usdc".to_owned()));
}

#[test]
fn metadata_enrichment_tolerates_missing_token_details() {
    let bundle: Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/microstructure/metadata_enrichment.json"
    ))
    .expect("fixture json");
    let raw_spot = bundle["spotMetaAndAssetCtxs"].to_string();

    let metadata = metadata_enrichments_from_public_info(
        &raw_spot,
        &HashMap::new(),
        1_710_000_100_000,
        1_710_000_100_000,
    )
    .expect("partial metadata still parses");

    assert_eq!(metadata.len(), 2);
    assert!(
        metadata
            .iter()
            .all(|row| row.has_tag(COHORT_UNKNOWN_METADATA))
    );
}

#[test]
fn parses_public_token_details_numeric_strings_and_deploy_time() {
    let details = parse_token_details(
        "0x0000000000000000000000000000000000000150",
        r#"{
          "name": "HYPE",
          "maxSupply": "1000000000.0",
          "circulatingSupply": "100000000.0",
          "deployer": "0x1234567890abcdef1234567890abcdef12345678",
          "deployTime": "2024-03-03T09:46:40.000",
          "seededUsdc": "1250000.0"
        }"#,
    )
    .expect("token details parse");

    assert_eq!(details.name.as_deref(), Some("HYPE"));
    assert_eq!(details.max_supply, Some(1_000_000_000.0));
    assert_eq!(details.circulating_supply, Some(100_000_000.0));
    assert_eq!(details.seeded_usdc, Some(1_250_000.0));
    assert_eq!(details.deploy_time_ms, Some(1_709_459_200_000));
}
