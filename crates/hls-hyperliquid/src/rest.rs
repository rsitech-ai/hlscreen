use std::{collections::HashMap, time::Duration};

use hls_core::{HlsError, HlsResult, symbol::MarketSymbol};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

const DEFAULT_INFO_BASE_URL: &str = "https://api.hyperliquid.xyz";
const DEFAULT_REST_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Clone, Debug)]
pub struct HyperliquidRestClient {
    base_url: String,
    client: reqwest::Client,
}

impl Default for HyperliquidRestClient {
    fn default() -> Self {
        Self::new(DEFAULT_INFO_BASE_URL)
    }
}

impl HyperliquidRestClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            client: default_http_client(),
        }
    }

    pub async fn spot_meta(&self) -> HlsResult<Vec<MarketSymbol>> {
        let body = self.post_info(json!({ "type": "spotMeta" })).await?;
        parse_spot_meta(&body)
    }

    pub async fn spot_meta_and_asset_ctxs(&self) -> HlsResult<Vec<SpotMarketContext>> {
        let body = self
            .post_info(json!({ "type": "spotMetaAndAssetCtxs" }))
            .await?;
        parse_spot_meta_and_asset_ctxs(&body)
    }

    async fn post_info(&self, request: Value) -> HlsResult<String> {
        let url = format!("{}/info", self.base_url.trim_end_matches('/'));
        let response = self
            .client
            .post(url)
            .json(&request)
            .send()
            .await
            .map_err(|err| HlsError::External(format!("Hyperliquid REST request failed: {err}")))?
            .error_for_status()
            .map_err(|err| {
                HlsError::External(format!("Hyperliquid REST returned an error: {err}"))
            })?;

        response.text().await.map_err(|err| {
            HlsError::External(format!("Hyperliquid REST response read failed: {err}"))
        })
    }
}

fn default_http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(DEFAULT_REST_TIMEOUT)
        .build()
        .expect("static reqwest client timeout configuration should build")
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SpotMarketContext {
    pub symbol: MarketSymbol,
    pub day_ntl_vlm: Option<f64>,
    pub prev_day_px: Option<f64>,
    pub mark_px: Option<f64>,
    pub mid_px: Option<f64>,
    pub circulating_supply: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct SpotMetaResponse {
    tokens: Vec<SpotToken>,
    universe: Vec<SpotUniverseEntry>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SpotToken {
    index: u32,
    sz_decimals: u32,
    wei_decimals: u32,
    #[serde(default)]
    is_canonical: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SpotUniverseEntry {
    name: String,
    tokens: Vec<u32>,
    index: u32,
    #[serde(default)]
    is_canonical: bool,
}

pub fn parse_spot_meta(raw: &str) -> HlsResult<Vec<MarketSymbol>> {
    let meta: SpotMetaResponse = serde_json::from_str(raw)
        .map_err(|err| HlsError::Parse(format!("invalid spotMeta JSON: {err}")))?;
    symbols_from_meta(meta)
}

pub fn parse_spot_meta_and_asset_ctxs(raw: &str) -> HlsResult<Vec<SpotMarketContext>> {
    let root: Value = serde_json::from_str(raw)
        .map_err(|err| HlsError::Parse(format!("invalid spotMetaAndAssetCtxs JSON: {err}")))?;
    let parts = root.as_array().ok_or_else(|| {
        HlsError::Parse("spotMetaAndAssetCtxs response must be a two-element array".to_owned())
    })?;
    let meta_value = parts.first().ok_or_else(|| {
        HlsError::Parse("spotMetaAndAssetCtxs response is missing spot metadata".to_owned())
    })?;
    let context_values = parts.get(1).and_then(Value::as_array).ok_or_else(|| {
        HlsError::Parse("spotMetaAndAssetCtxs response is missing asset contexts".to_owned())
    })?;
    let meta: SpotMetaResponse = serde_json::from_value(meta_value.clone())
        .map_err(|err| HlsError::Parse(format!("invalid embedded spot metadata: {err}")))?;
    let symbols = symbols_from_meta(meta)?;

    symbols
        .into_iter()
        .enumerate()
        .map(|(index, symbol)| {
            let context = context_values.get(index).unwrap_or(&Value::Null);

            Ok(SpotMarketContext {
                symbol,
                day_ntl_vlm: numeric_field(context, "dayNtlVlm")?,
                prev_day_px: numeric_field(context, "prevDayPx")?,
                mark_px: numeric_field(context, "markPx")?,
                mid_px: numeric_field(context, "midPx")?,
                circulating_supply: numeric_field(context, "circulatingSupply")?,
            })
        })
        .collect()
}

pub fn select_universe(
    markets: &[SpotMarketContext],
    top_n: usize,
    include: &[String],
    exclude: &[String],
) -> HlsResult<Vec<SpotMarketContext>> {
    if top_n == 0 {
        return Err(HlsError::Config(
            "top_n must be greater than zero".to_owned(),
        ));
    }

    let mut selected: Vec<SpotMarketContext> = markets
        .iter()
        .filter(|market| !matches_any(&market.symbol, exclude))
        .filter(|market| include.is_empty() || matches_any(&market.symbol, include))
        .cloned()
        .collect();

    selected.sort_by(|left, right| {
        let left_volume = left.day_ntl_vlm.unwrap_or(f64::NEG_INFINITY);
        let right_volume = right.day_ntl_vlm.unwrap_or(f64::NEG_INFINITY);

        right_volume.total_cmp(&left_volume)
    });
    selected.truncate(top_n);

    Ok(selected)
}

fn symbols_from_meta(meta: SpotMetaResponse) -> HlsResult<Vec<MarketSymbol>> {
    let tokens_by_index: HashMap<u32, SpotToken> = meta
        .tokens
        .into_iter()
        .map(|token| (token.index, token))
        .collect();

    meta.universe
        .into_iter()
        .map(|entry| {
            let base_token_index = *entry.tokens.first().ok_or_else(|| {
                HlsError::Parse(format!(
                    "spot universe entry '{}' is missing base token",
                    entry.name
                ))
            })?;
            let quote_token_index = *entry.tokens.get(1).ok_or_else(|| {
                HlsError::Parse(format!(
                    "spot universe entry '{}' is missing quote token",
                    entry.name
                ))
            })?;
            let base_token = tokens_by_index.get(&base_token_index).ok_or_else(|| {
                HlsError::Parse(format!(
                    "spot universe entry '{}' references unknown base token {}",
                    entry.name, base_token_index
                ))
            })?;

            MarketSymbol::new(
                entry.name,
                entry.index,
                base_token_index,
                quote_token_index,
                base_token.sz_decimals,
                base_token.wei_decimals,
                entry.is_canonical && base_token.is_canonical,
            )
        })
        .collect()
}

fn numeric_field(value: &Value, field: &str) -> HlsResult<Option<f64>> {
    let Some(raw) = value.get(field) else {
        return Ok(None);
    };

    match raw {
        Value::Null => Ok(None),
        Value::Number(number) => number
            .as_f64()
            .ok_or_else(|| HlsError::Parse(format!("field '{field}' is not representable as f64")))
            .map(Some),
        Value::String(text) if text.trim().is_empty() => Ok(None),
        Value::String(text) => text.parse::<f64>().map(Some).map_err(|err| {
            HlsError::Parse(format!(
                "field '{field}' has invalid numeric value '{text}': {err}"
            ))
        }),
        other => Err(HlsError::Parse(format!(
            "field '{field}' must be a number or numeric string, got {other}"
        ))),
    }
}

fn matches_any(symbol: &MarketSymbol, selectors: &[String]) -> bool {
    selectors
        .iter()
        .any(|selector| symbol.matches_selector(selector))
}
