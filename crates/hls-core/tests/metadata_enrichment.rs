use hls_core::metadata::{
    COHORT_FRESH_LIQUIDITY, COHORT_LOW_FLOAT, COHORT_NEW_LISTING, COHORT_UNKNOWN_METADATA,
    MetadataEnrichment, MetadataEnrichmentInput,
};

#[test]
fn complete_public_metadata_computes_listing_and_liquidity_tags() {
    let metadata = MetadataEnrichment::from_public_input(MetadataEnrichmentInput {
        symbol: "@107".to_owned(),
        display_name: "HYPE/USDC".to_owned(),
        feed_identifier: "@107".to_owned(),
        spot_index: 107,
        base_token_index: 150,
        quote_token_index: 0,
        metadata_source: "spotMetaAndAssetCtxs+tokenDetails".to_owned(),
        metadata_fetched_at_ms: 1_710_000_100_000,
        deploy_time_ms: Some(1_709_400_000_000),
        deployer: Some("0x1234567890abcdef1234567890abcdef12345678".to_owned()),
        seeded_usdc: Some(1_250_000.0),
        max_supply: Some(1_000_000_000.0),
        circulating_supply: Some(100_000_000.0),
        now_ms: 1_710_000_100_000,
    });

    assert_eq!(metadata.listing_age_ms, Some(600_100_000));
    assert!(metadata.has_tag(COHORT_NEW_LISTING));
    assert!(metadata.has_tag(COHORT_FRESH_LIQUIDITY));
    assert!(metadata.has_tag(COHORT_LOW_FLOAT));
    assert!(!metadata.has_tag(COHORT_UNKNOWN_METADATA));
    assert!(metadata.is_complete());
}

#[test]
fn partial_public_metadata_marks_unknown_fields_explicitly() {
    let metadata = MetadataEnrichment::from_public_input(MetadataEnrichmentInput {
        symbol: "@999".to_owned(),
        display_name: "PARTIAL/USDC".to_owned(),
        feed_identifier: "@999".to_owned(),
        spot_index: 999,
        base_token_index: 999,
        quote_token_index: 0,
        metadata_source: "spotMetaAndAssetCtxs".to_owned(),
        metadata_fetched_at_ms: 1_710_000_100_000,
        deploy_time_ms: None,
        deployer: None,
        seeded_usdc: None,
        max_supply: None,
        circulating_supply: Some(42_000.0),
        now_ms: 1_710_000_100_000,
    });

    assert_eq!(metadata.listing_age_ms, None);
    assert!(metadata.has_tag(COHORT_UNKNOWN_METADATA));
    assert!(!metadata.is_complete());
    assert_eq!(
        metadata.unknown_fields,
        vec!["deployer", "deploy_time_ms", "seeded_usdc", "max_supply"]
    );
}
