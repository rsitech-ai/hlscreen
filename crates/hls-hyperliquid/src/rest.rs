use std::{collections::HashMap, time::Duration};

use hls_core::{
    HlsError, HlsResult,
    metadata::{MetadataEnrichment, MetadataEnrichmentInput},
    symbol::MarketSymbol,
    time::{now_millis, parse_utc_datetime_millis},
};
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
        let fetched_at_ms = now_ms_i64()?;
        parse_spot_meta_and_asset_ctxs_at(&body, fetched_at_ms, fetched_at_ms)
    }

    pub async fn token_details(&self, token_id: &str) -> HlsResult<PublicTokenDetails> {
        let body = self
            .post_info(json!({ "type": "tokenDetails", "tokenId": token_id }))
            .await?;
        parse_token_details(token_id, &body)
    }

    pub async fn spot_metadata_enrichments(
        &self,
        token_detail_limit: usize,
    ) -> HlsResult<Vec<MetadataEnrichment>> {
        let body = self
            .post_info(json!({ "type": "spotMetaAndAssetCtxs" }))
            .await?;
        let fetched_at_ms = now_ms_i64()?;
        let token_ids = base_token_ids_from_spot_meta_and_asset_ctxs(&body)?;
        let mut details = HashMap::new();
        for token_id in token_ids.into_iter().take(token_detail_limit) {
            if let Ok(detail) = self.token_details(&token_id).await {
                details.insert(token_id, detail);
            }
        }
        metadata_enrichments_from_public_info(&body, &details, fetched_at_ms, fetched_at_ms)
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
    pub metadata: MetadataEnrichment,
    pub day_ntl_vlm: Option<f64>,
    pub prev_day_px: Option<f64>,
    pub mark_px: Option<f64>,
    pub mid_px: Option<f64>,
    pub circulating_supply: Option<f64>,
}

#[derive(Clone, Debug, Deserialize)]
struct SpotMetaResponse {
    tokens: Vec<SpotToken>,
    universe: Vec<SpotUniverseEntry>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SpotToken {
    name: String,
    index: u32,
    sz_decimals: u32,
    wei_decimals: u32,
    #[serde(default)]
    is_canonical: bool,
    #[serde(default)]
    token_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SpotUniverseEntry {
    name: String,
    tokens: Vec<u32>,
    index: u32,
    #[serde(default)]
    is_canonical: bool,
}

#[derive(Debug, Deserialize)]
struct MetadataEnrichmentBundle {
    #[serde(alias = "fetchedAtMs")]
    fetched_at_ms: i64,
    #[serde(alias = "nowMs")]
    now_ms: i64,
    #[serde(rename = "spotMetaAndAssetCtxs", alias = "spot_meta_and_asset_ctxs")]
    spot_meta_and_asset_ctxs: Value,
    #[serde(
        default,
        rename = "tokenDetailsByTokenId",
        alias = "token_details_by_token_id"
    )]
    token_details_by_token_id: HashMap<String, Value>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PublicTokenDetails {
    pub token_id: String,
    pub name: Option<String>,
    pub deployer: Option<String>,
    pub deploy_time_ms: Option<i64>,
    pub seeded_usdc: Option<f64>,
    pub max_supply: Option<f64>,
    pub circulating_supply: Option<f64>,
}

pub fn parse_spot_meta(raw: &str) -> HlsResult<Vec<MarketSymbol>> {
    let meta: SpotMetaResponse = serde_json::from_str(raw)
        .map_err(|err| HlsError::Parse(format!("invalid spotMeta JSON: {err}")))?;
    symbols_from_meta(meta)
}

pub fn parse_spot_meta_and_asset_ctxs(raw: &str) -> HlsResult<Vec<SpotMarketContext>> {
    parse_spot_meta_and_asset_ctxs_at(raw, 0, 0)
}

pub fn parse_spot_meta_and_asset_ctxs_at(
    raw: &str,
    fetched_at_ms: i64,
    now_ms: i64,
) -> HlsResult<Vec<SpotMarketContext>> {
    parse_spot_meta_and_asset_ctxs_with_details(raw, &HashMap::new(), fetched_at_ms, now_ms)
}

pub fn parse_spot_meta_and_asset_ctxs_with_details(
    raw: &str,
    token_details: &HashMap<String, PublicTokenDetails>,
    fetched_at_ms: i64,
    now_ms: i64,
) -> HlsResult<Vec<SpotMarketContext>> {
    let root: Value = serde_json::from_str(raw)
        .map_err(|err| HlsError::Parse(format!("invalid spotMetaAndAssetCtxs JSON: {err}")))?;
    let (meta, context_values) = spot_meta_and_contexts_from_value(&root)?;
    let symbols = symbols_from_meta(meta.clone())?;
    let contexts_have_coin = context_values
        .iter()
        .any(|context| context.get("coin").and_then(Value::as_str).is_some());
    let contexts_by_coin: HashMap<String, Value> = context_values
        .iter()
        .filter_map(|context| {
            context
                .get("coin")
                .and_then(Value::as_str)
                .map(|coin| (coin.to_owned(), context.clone()))
        })
        .collect();
    let tokens_by_index: HashMap<u32, SpotToken> = meta
        .tokens
        .into_iter()
        .map(|token| (token.index, token))
        .collect();

    symbols
        .into_iter()
        .enumerate()
        .map(|(index, symbol)| {
            let context = if contexts_have_coin {
                contexts_by_coin
                    .get(&symbol.hl_coin)
                    .unwrap_or(&Value::Null)
            } else {
                context_values.get(index).unwrap_or(&Value::Null)
            };
            let entry = meta.universe.get(index).ok_or_else(|| {
                HlsError::Parse(format!("missing universe entry for context index {index}"))
            })?;
            let base_token = tokens_by_index
                .get(&symbol.base_token_index)
                .ok_or_else(|| {
                    HlsError::Parse(format!(
                        "spot universe entry '{}' references unknown base token {}",
                        entry.name, symbol.base_token_index
                    ))
                })?;
            let detail = base_token
                .token_id
                .as_ref()
                .and_then(|token_id| token_details.get(token_id));
            let circulating_supply = detail
                .and_then(|detail| detail.circulating_supply)
                .or(numeric_field(context, "circulatingSupply")?);

            Ok(SpotMarketContext {
                metadata: MetadataEnrichment::from_public_input(MetadataEnrichmentInput {
                    symbol: symbol.hl_coin.clone(),
                    display_name: symbol.display_name.clone(),
                    feed_identifier: symbol.hl_coin.clone(),
                    spot_index: symbol.spot_index,
                    base_token_index: symbol.base_token_index,
                    quote_token_index: symbol.quote_token_index,
                    metadata_source: if detail.is_some() {
                        "spotMetaAndAssetCtxs+tokenDetails".to_owned()
                    } else {
                        "spotMetaAndAssetCtxs".to_owned()
                    },
                    metadata_fetched_at_ms: fetched_at_ms,
                    deploy_time_ms: detail.and_then(|detail| detail.deploy_time_ms),
                    deployer: detail.and_then(|detail| detail.deployer.clone()),
                    seeded_usdc: detail.and_then(|detail| detail.seeded_usdc),
                    max_supply: detail.and_then(|detail| detail.max_supply),
                    circulating_supply,
                    now_ms,
                }),
                symbol,
                day_ntl_vlm: numeric_field(context, "dayNtlVlm")?,
                prev_day_px: numeric_field(context, "prevDayPx")?,
                mark_px: numeric_field(context, "markPx")?,
                mid_px: numeric_field(context, "midPx")?,
                circulating_supply,
            })
        })
        .collect()
}

pub fn metadata_enrichments_from_public_info(
    raw_spot_meta_and_asset_ctxs: &str,
    token_details: &HashMap<String, PublicTokenDetails>,
    fetched_at_ms: i64,
    now_ms: i64,
) -> HlsResult<Vec<MetadataEnrichment>> {
    Ok(parse_spot_meta_and_asset_ctxs_with_details(
        raw_spot_meta_and_asset_ctxs,
        token_details,
        fetched_at_ms,
        now_ms,
    )?
    .into_iter()
    .map(|market| market.metadata)
    .collect())
}

pub fn parse_metadata_enrichment_bundle(raw: &str) -> HlsResult<Vec<MetadataEnrichment>> {
    let root: MetadataEnrichmentBundle = serde_json::from_str(raw)
        .map_err(|err| HlsError::Parse(format!("invalid metadata enrichment bundle: {err}")))?;
    let mut token_details = HashMap::new();
    for (token_id, value) in root.token_details_by_token_id {
        let detail = parse_token_details_value(&token_id, &value)?;
        token_details.insert(token_id, detail);
    }
    metadata_enrichments_from_public_info(
        &root.spot_meta_and_asset_ctxs.to_string(),
        &token_details,
        root.fetched_at_ms,
        root.now_ms,
    )
}

pub fn parse_token_details(token_id: &str, raw: &str) -> HlsResult<PublicTokenDetails> {
    let value: Value = serde_json::from_str(raw)
        .map_err(|err| HlsError::Parse(format!("invalid tokenDetails JSON: {err}")))?;
    parse_token_details_value(token_id, &value)
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
            let quote_token = tokens_by_index.get(&quote_token_index).ok_or_else(|| {
                HlsError::Parse(format!(
                    "spot universe entry '{}' references unknown quote token {}",
                    entry.name, quote_token_index
                ))
            })?;
            let display_name = format!("{}/{}", base_token.name, quote_token.name);

            MarketSymbol::new(
                display_name,
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

fn spot_meta_and_contexts_from_value(root: &Value) -> HlsResult<(SpotMetaResponse, Vec<Value>)> {
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
    Ok((meta, context_values.clone()))
}

fn base_token_ids_from_spot_meta_and_asset_ctxs(raw: &str) -> HlsResult<Vec<String>> {
    let root: Value = serde_json::from_str(raw)
        .map_err(|err| HlsError::Parse(format!("invalid spotMetaAndAssetCtxs JSON: {err}")))?;
    let (meta, _) = spot_meta_and_contexts_from_value(&root)?;
    let tokens_by_index: HashMap<u32, SpotToken> = meta
        .tokens
        .into_iter()
        .map(|token| (token.index, token))
        .collect();
    let mut token_ids = Vec::new();

    for entry in meta.universe {
        let Some(base_token_index) = entry.tokens.first() else {
            continue;
        };
        if let Some(token_id) = tokens_by_index
            .get(base_token_index)
            .and_then(|token| token.token_id.clone())
        {
            token_ids.push(token_id);
        }
    }

    token_ids.sort();
    token_ids.dedup();
    Ok(token_ids)
}

fn parse_token_details_value(token_id: &str, value: &Value) -> HlsResult<PublicTokenDetails> {
    Ok(PublicTokenDetails {
        token_id: token_id.to_owned(),
        name: value
            .get("name")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        deployer: value
            .get("deployer")
            .and_then(Value::as_str)
            .filter(|text| !text.trim().is_empty())
            .map(ToOwned::to_owned),
        deploy_time_ms: match value.get("deployTime").and_then(Value::as_str) {
            Some(text) if !text.trim().is_empty() => Some(
                i64::try_from(
                    parse_utc_datetime_millis(text)
                        .map_err(|err| HlsError::Parse(format!("invalid deployTime: {err}")))?,
                )
                .map_err(|err| HlsError::Parse(format!("deployTime out of range: {err}")))?,
            ),
            _ => None,
        },
        seeded_usdc: numeric_field(value, "seededUsdc")?,
        max_supply: numeric_field(value, "maxSupply")?,
        circulating_supply: numeric_field(value, "circulatingSupply")?,
    })
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

fn now_ms_i64() -> HlsResult<i64> {
    i64::try_from(now_millis()?)
        .map_err(|err| HlsError::Time(format!("current timestamp exceeds i64: {err}")))
}

fn matches_any(symbol: &MarketSymbol, selectors: &[String]) -> bool {
    selectors
        .iter()
        .any(|selector| symbol.matches_selector(selector))
}
