use std::{fs, path::PathBuf};

use anyhow::Context;
use hls_core::{fees::FeeProfile, market_state::FeatureSnapshot};
use hls_features::{
    engine::FeatureEngine,
    tradeability::{FeeAwareTradeabilityInput, classify_fee_aware_tradeability},
};

pub(crate) fn load_fee_profile(path: Option<&PathBuf>) -> anyhow::Result<Option<FeeProfile>> {
    let Some(path) = path else {
        return Ok(None);
    };
    let raw = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("json");
    let profile: FeeProfile = if extension.eq_ignore_ascii_case("toml") {
        toml::from_str(&raw).with_context(|| format!("parse {}", path.display()))?
    } else {
        serde_json::from_str(&raw).with_context(|| format!("parse {}", path.display()))?
    };
    profile.validate()?;
    Ok(Some(profile))
}

pub(crate) fn feature_engine(fee_profile: Option<&FeeProfile>) -> FeatureEngine {
    match fee_profile {
        Some(profile) => FeatureEngine::default().with_fee_profile(profile.clone()),
        None => FeatureEngine::default(),
    }
}

pub(crate) fn apply_fee_profile(
    snapshots: &mut [FeatureSnapshot],
    fee_profile: Option<&FeeProfile>,
) {
    let Some(profile) = fee_profile else {
        return;
    };
    for snapshot in snapshots {
        snapshot.fee_aware_tradeability =
            classify_fee_aware_tradeability(FeeAwareTradeabilityInput {
                spread_bps: snapshot.spread_bps,
                base_state: snapshot.tradeability_state,
                profile,
            });
    }
}
