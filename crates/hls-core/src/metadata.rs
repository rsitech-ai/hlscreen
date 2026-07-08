use serde::{Deserialize, Serialize};

pub const COHORT_NEW_LISTING: &str = "new_listing";
pub const COHORT_FRESH_LIQUIDITY: &str = "fresh_liquidity";
pub const COHORT_LOW_FLOAT: &str = "low_float";
pub const COHORT_UNKNOWN_METADATA: &str = "unknown_metadata";

const NEW_LISTING_WINDOW_MS: i64 = 14 * 24 * 60 * 60 * 1_000;
const LOW_FLOAT_RATIO: f64 = 0.25;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct MetadataEnrichment {
    pub symbol: String,
    pub display_name: String,
    pub feed_identifier: String,
    pub spot_index: u32,
    pub base_token_index: u32,
    pub quote_token_index: u32,
    pub metadata_source: String,
    pub metadata_fetched_at_ms: i64,
    pub listing_age_ms: Option<i64>,
    pub deployer: Option<String>,
    pub deploy_time_ms: Option<i64>,
    pub seeded_usdc: Option<f64>,
    pub max_supply: Option<f64>,
    pub circulating_supply: Option<f64>,
    pub cohort_tags: Vec<String>,
    pub unknown_fields: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MetadataEnrichmentInput {
    pub symbol: String,
    pub display_name: String,
    pub feed_identifier: String,
    pub spot_index: u32,
    pub base_token_index: u32,
    pub quote_token_index: u32,
    pub metadata_source: String,
    pub metadata_fetched_at_ms: i64,
    pub deploy_time_ms: Option<i64>,
    pub deployer: Option<String>,
    pub seeded_usdc: Option<f64>,
    pub max_supply: Option<f64>,
    pub circulating_supply: Option<f64>,
    pub now_ms: i64,
}

impl MetadataEnrichment {
    pub fn from_public_input(input: MetadataEnrichmentInput) -> Self {
        let listing_age_ms = input
            .deploy_time_ms
            .map(|deploy_time_ms| input.now_ms.saturating_sub(deploy_time_ms).max(0));
        let unknown_fields = unknown_fields(&input);
        let mut cohort_tags = cohort_tags(
            listing_age_ms,
            input.seeded_usdc,
            input.max_supply,
            input.circulating_supply,
            unknown_fields.is_empty(),
        );

        cohort_tags.sort();
        cohort_tags.dedup();

        Self {
            symbol: input.symbol,
            display_name: input.display_name,
            feed_identifier: input.feed_identifier,
            spot_index: input.spot_index,
            base_token_index: input.base_token_index,
            quote_token_index: input.quote_token_index,
            metadata_source: input.metadata_source,
            metadata_fetched_at_ms: input.metadata_fetched_at_ms,
            listing_age_ms,
            deployer: input.deployer,
            deploy_time_ms: input.deploy_time_ms,
            seeded_usdc: input.seeded_usdc,
            max_supply: input.max_supply,
            circulating_supply: input.circulating_supply,
            cohort_tags,
            unknown_fields,
        }
    }

    pub fn has_tag(&self, tag: &str) -> bool {
        self.cohort_tags.iter().any(|candidate| candidate == tag)
    }

    pub fn is_complete(&self) -> bool {
        self.unknown_fields.is_empty()
    }

    pub fn cohort_label(&self) -> String {
        if self.cohort_tags.is_empty() {
            COHORT_UNKNOWN_METADATA.to_owned()
        } else {
            self.cohort_tags.join(",")
        }
    }
}

fn unknown_fields(input: &MetadataEnrichmentInput) -> Vec<String> {
    let mut fields = Vec::new();
    if input.deployer.as_deref().is_none_or(str::is_empty) {
        fields.push("deployer".to_owned());
    }
    if input.deploy_time_ms.is_none() {
        fields.push("deploy_time_ms".to_owned());
    }
    if input.seeded_usdc.is_none() {
        fields.push("seeded_usdc".to_owned());
    }
    if input.max_supply.is_none() {
        fields.push("max_supply".to_owned());
    }
    if input.circulating_supply.is_none() {
        fields.push("circulating_supply".to_owned());
    }
    fields
}

fn cohort_tags(
    listing_age_ms: Option<i64>,
    seeded_usdc: Option<f64>,
    max_supply: Option<f64>,
    circulating_supply: Option<f64>,
    metadata_complete: bool,
) -> Vec<String> {
    let mut tags = Vec::new();

    if listing_age_ms.is_some_and(|age| age <= NEW_LISTING_WINDOW_MS) {
        tags.push(COHORT_NEW_LISTING.to_owned());
    }
    if seeded_usdc.is_some_and(|seeded| seeded > 0.0) {
        tags.push(COHORT_FRESH_LIQUIDITY.to_owned());
    }
    if let (Some(circulating), Some(max)) = (circulating_supply, max_supply)
        && max > 0.0
        && (circulating / max) <= LOW_FLOAT_RATIO
    {
        tags.push(COHORT_LOW_FLOAT.to_owned());
    }
    if !metadata_complete || tags.is_empty() {
        tags.push(COHORT_UNKNOWN_METADATA.to_owned());
    }

    tags
}
