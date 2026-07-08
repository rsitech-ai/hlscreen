use std::{collections::HashMap, fs, path::PathBuf};

use anyhow::Context;
use hls_core::{market_state::FeatureSnapshot, metadata::MetadataEnrichment};
use hls_hyperliquid::rest::parse_metadata_enrichment_bundle;

pub(crate) fn load_metadata_enrichments(
    path: Option<&PathBuf>,
) -> anyhow::Result<Vec<MetadataEnrichment>> {
    let Some(path) = path else {
        return Ok(Vec::new());
    };
    let raw = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    Ok(parse_metadata_enrichment_bundle(&raw)?)
}

pub(crate) fn attach_metadata(
    snapshots: &mut [FeatureSnapshot],
    metadata: impl IntoIterator<Item = MetadataEnrichment>,
) {
    let mut by_symbol = HashMap::new();
    for item in metadata {
        by_symbol.insert(item.feed_identifier.clone(), item.clone());
        by_symbol.insert(item.display_name.clone(), item);
    }

    for snapshot in snapshots {
        if let Some(metadata) = by_symbol.get(&snapshot.symbol) {
            snapshot.metadata = Some(metadata.clone());
        }
    }
}
