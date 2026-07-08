use serde::{Deserialize, Serialize};

use crate::error::{HlsError, HlsResult};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MarketSymbol {
    pub symbol_id: u32,
    pub display_name: String,
    pub hl_coin: String,
    pub spot_index: u32,
    pub base_token_index: u32,
    pub quote_token_index: u32,
    pub sz_decimals: u32,
    pub wei_decimals: u32,
    pub is_canonical: bool,
    pub first_seen_ms: Option<i64>,
    pub last_seen_ms: Option<i64>,
}

impl MarketSymbol {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        display_name: impl Into<String>,
        spot_index: u32,
        base_token_index: u32,
        quote_token_index: u32,
        sz_decimals: u32,
        wei_decimals: u32,
        is_canonical: bool,
    ) -> HlsResult<Self> {
        let display_name = display_name.into();
        let trimmed = display_name.trim();

        if trimmed.is_empty() {
            return Err(HlsError::Symbol(
                "display name must not be empty".to_owned(),
            ));
        }

        Ok(Self {
            symbol_id: spot_index,
            display_name: trimmed.to_owned(),
            hl_coin: feed_id_for_spot(trimmed, spot_index),
            spot_index,
            base_token_index,
            quote_token_index,
            sz_decimals,
            wei_decimals,
            is_canonical,
            first_seen_ms: None,
            last_seen_ms: None,
        })
    }

    pub fn matches_selector(&self, selector: &str) -> bool {
        let normalized_selector = selector.trim().replace('-', "/");
        self.display_name.eq_ignore_ascii_case(selector)
            || self.display_name.eq_ignore_ascii_case(&normalized_selector)
            || self.hl_coin.eq_ignore_ascii_case(selector)
    }
}

pub fn feed_id_for_spot(display_name: &str, spot_index: u32) -> String {
    if display_name.eq_ignore_ascii_case("PURR/USDC") {
        "PURR/USDC".to_owned()
    } else {
        format!("@{spot_index}")
    }
}
