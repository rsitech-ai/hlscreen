use std::{collections::HashMap, net::IpAddr, time::Duration};

use hls_core::{
    HlsError, HlsResult,
    market_state::{CandleCompletion, CandleEvent, CandleProvenance},
    metadata::{MetadataEnrichment, MetadataEnrichmentInput},
    symbol::MarketSymbol,
    time::{now_millis, parse_utc_datetime_millis},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use time::{OffsetDateTime, format_description::well_known::Rfc2822};

const DEFAULT_INFO_BASE_URL: &str = "https://api.hyperliquid.xyz";
const DEFAULT_REST_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug)]
pub enum PublicRestError {
    InvalidRequest(HlsError),
    Transport(String),
    HttpStatus {
        status: reqwest::StatusCode,
        retry_after: Option<Duration>,
    },
    Response(HlsError),
}

impl PublicRestError {
    pub fn status(&self) -> Option<reqwest::StatusCode> {
        match self {
            Self::HttpStatus { status, .. } => Some(*status),
            _ => None,
        }
    }

    pub fn retry_after(&self) -> Option<Duration> {
        match self {
            Self::HttpStatus { retry_after, .. } => *retry_after,
            _ => None,
        }
    }

    pub fn is_too_many_requests(&self) -> bool {
        self.status() == Some(reqwest::StatusCode::TOO_MANY_REQUESTS)
    }

    fn into_hls_error(self) -> HlsError {
        match self {
            Self::InvalidRequest(error) | Self::Response(error) => error,
            error => HlsError::External(error.to_string()),
        }
    }
}

impl std::fmt::Display for PublicRestError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidRequest(error) => {
                write!(formatter, "invalid public REST request: {error}")
            }
            Self::Transport(error) => write!(formatter, "Hyperliquid REST request failed: {error}"),
            Self::HttpStatus {
                status,
                retry_after,
            } => {
                write!(formatter, "Hyperliquid REST returned HTTP {status}")?;
                if let Some(delay) = retry_after {
                    write!(formatter, " (Retry-After {} seconds)", delay.as_secs())?;
                }
                Ok(())
            }
            Self::Response(error) => {
                write!(formatter, "invalid Hyperliquid REST response: {error}")
            }
        }
    }
}

impl std::error::Error for PublicRestError {}

pub fn validate_public_rest_base_url(base_url: &str) -> HlsResult<()> {
    let url = reqwest::Url::parse(base_url)
        .map_err(|err| HlsError::Config(format!("public REST base URL is invalid: {err}")))?;
    if !url.username().is_empty() || url.password().is_some() {
        return Err(HlsError::Config(
            "public REST base URL must not contain credentials".to_owned(),
        ));
    }
    if url.query().is_some() || url.fragment().is_some() {
        return Err(HlsError::Config(
            "public REST base URL must not contain a query or fragment".to_owned(),
        ));
    }

    let host = url
        .host_str()
        .ok_or_else(|| HlsError::Config("public REST base URL must contain a host".to_owned()))?;
    let is_loopback = host.eq_ignore_ascii_case("localhost")
        || host.parse::<IpAddr>().is_ok_and(|ip| ip.is_loopback());
    if url.scheme() != "https" && !(url.scheme() == "http" && is_loopback) {
        return Err(HlsError::Config(
            "--rest-url must use HTTPS or an HTTP loopback address".to_owned(),
        ));
    }
    Ok(())
}

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

    pub async fn candle_snapshot(
        &self,
        coin: &str,
        interval: &str,
        start_time_ms: i64,
        end_time_ms: i64,
    ) -> HlsResult<Vec<CandleEvent>> {
        self.candle_snapshot_attempt(coin, interval, start_time_ms, end_time_ms)
            .await
            .map_err(PublicRestError::into_hls_error)
    }

    pub async fn candle_snapshot_attempt(
        &self,
        coin: &str,
        interval: &str,
        start_time_ms: i64,
        end_time_ms: i64,
    ) -> Result<Vec<CandleEvent>, PublicRestError> {
        if coin.trim().is_empty() {
            return Err(PublicRestError::InvalidRequest(HlsError::Config(
                "candle snapshot coin must not be empty".to_owned(),
            )));
        }
        if interval.trim().is_empty() {
            return Err(PublicRestError::InvalidRequest(HlsError::Config(
                "candle snapshot interval must not be empty".to_owned(),
            )));
        }
        if start_time_ms > end_time_ms {
            return Err(PublicRestError::InvalidRequest(HlsError::Config(
                "candle snapshot start_time_ms must be <= end_time_ms".to_owned(),
            )));
        }

        let body = self
            .post_info_attempt(json!({
                "type": "candleSnapshot",
                "req": {
                    "coin": coin,
                    "interval": interval,
                    "startTime": start_time_ms,
                    "endTime": end_time_ms
                }
            }))
            .await?;
        let candles = parse_candle_snapshot(&body).map_err(PublicRestError::Response)?;
        validate_candle_snapshot_response(&candles, coin, interval, start_time_ms, end_time_ms)
            .map_err(PublicRestError::Response)?;
        Ok(candles)
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
        self.post_info_attempt(request)
            .await
            .map_err(PublicRestError::into_hls_error)
    }

    async fn post_info_attempt(&self, request: Value) -> Result<String, PublicRestError> {
        let url = format!("{}/info", self.base_url.trim_end_matches('/'));
        let response = self
            .client
            .post(url)
            .json(&request)
            .send()
            .await
            .map_err(|err| PublicRestError::Transport(err.to_string()))?;

        if !response.status().is_success() {
            let retry_after = response
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| parse_retry_after_at(value, OffsetDateTime::now_utc()));
            return Err(PublicRestError::HttpStatus {
                status: response.status(),
                retry_after,
            });
        }

        response
            .text()
            .await
            .map_err(|err| PublicRestError::Transport(format!("response read failed: {err}")))
    }
}

fn parse_retry_after_at(value: &str, now: OffsetDateTime) -> Option<Duration> {
    let value = value.trim();
    if let Ok(seconds) = value.parse::<u64>() {
        return Some(Duration::from_secs(seconds));
    }

    let deadline = OffsetDateTime::parse(value, &Rfc2822).ok()?;
    let remaining_ns = (deadline - now).whole_nanoseconds();
    if remaining_ns <= 0 {
        return Some(Duration::ZERO);
    }
    let rounded_seconds = remaining_ns.saturating_add(999_999_999) / 1_000_000_000;
    u64::try_from(rounded_seconds).ok().map(Duration::from_secs)
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
                .or(numeric_field_non_negative(context, "circulatingSupply")?);

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
                day_ntl_vlm: numeric_field_non_negative(context, "dayNtlVlm")?,
                prev_day_px: numeric_field_positive(context, "prevDayPx")?,
                mark_px: numeric_field_positive(context, "markPx")?,
                mid_px: numeric_field_positive(context, "midPx")?,
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

pub fn parse_candle_snapshot(raw: &str) -> HlsResult<Vec<CandleEvent>> {
    let parsed_at_ms = now_ms_i64()?;
    let candles: Vec<PublicCandleSnapshot> = serde_json::from_str(raw)
        .map_err(|err| HlsError::Parse(format!("invalid candleSnapshot JSON: {err}")))?;

    candles
        .into_iter()
        .map(|candle| {
            let open_ts_ms = required_i64(&candle.open_ts_ms, "candleSnapshot.t")?;
            let close_ts_ms = required_i64(&candle.close_ts_ms, "candleSnapshot.T")?;
            let open = required_f64(&candle.open, "candleSnapshot.o")?;
            let close = required_f64(&candle.close, "candleSnapshot.c")?;
            let high = required_f64(&candle.high, "candleSnapshot.h")?;
            let low = required_f64(&candle.low, "candleSnapshot.l")?;
            let volume_base = required_f64(&candle.volume_base, "candleSnapshot.v")?;
            let trade_count = required_u64(&candle.trade_count, "candleSnapshot.n")?;

            validate_candle_fields(open_ts_ms, close_ts_ms, open, high, low, close)?;

            Ok(CandleEvent {
                recv_ts_ns: 0,
                open_ts_ms,
                close_ts_ms,
                hl_coin: candle.hl_coin,
                interval: candle.interval,
                open,
                high,
                low,
                close,
                volume_base,
                trade_count,
                provenance: CandleProvenance::RestBootstrap,
                completion: if close_ts_ms < parsed_at_ms {
                    CandleCompletion::Closed
                } else {
                    CandleCompletion::Open
                },
            })
        })
        .collect()
}

fn validate_candle_snapshot_response(
    candles: &[CandleEvent],
    coin: &str,
    interval: &str,
    start_time_ms: i64,
    end_time_ms: i64,
) -> HlsResult<()> {
    for candle in candles {
        if candle.hl_coin != coin {
            return Err(HlsError::Parse(format!(
                "candleSnapshot returned coin '{}' for requested coin '{coin}'",
                candle.hl_coin
            )));
        }
        if candle.interval != interval {
            return Err(HlsError::Parse(format!(
                "candleSnapshot returned interval '{}' for requested interval '{interval}'",
                candle.interval
            )));
        }
        if candle.close_ts_ms < start_time_ms || candle.open_ts_ms > end_time_ms {
            return Err(HlsError::Parse(format!(
                "candleSnapshot returned candle [{}..={}] outside requested range [{start_time_ms}..={end_time_ms}]",
                candle.open_ts_ms, candle.close_ts_ms
            )));
        }
    }
    Ok(())
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
        seeded_usdc: numeric_field_non_negative(value, "seededUsdc")?,
        max_supply: numeric_field_non_negative(value, "maxSupply")?,
        circulating_supply: numeric_field_non_negative(value, "circulatingSupply")?,
    })
}

#[derive(Clone, Debug, Deserialize)]
struct PublicCandleSnapshot {
    #[serde(rename = "t")]
    open_ts_ms: Value,
    #[serde(rename = "T")]
    close_ts_ms: Value,
    #[serde(rename = "s")]
    hl_coin: String,
    #[serde(rename = "i")]
    interval: String,
    #[serde(rename = "o")]
    open: Value,
    #[serde(rename = "h")]
    high: Value,
    #[serde(rename = "l")]
    low: Value,
    #[serde(rename = "c")]
    close: Value,
    #[serde(rename = "v")]
    volume_base: Value,
    #[serde(rename = "n")]
    trade_count: Value,
}

fn required_i64(value: &Value, field: &str) -> HlsResult<i64> {
    match value {
        Value::Number(number) => number
            .as_i64()
            .ok_or_else(|| HlsError::Parse(format!("field '{field}' is not representable as i64"))),
        Value::String(text) => text
            .parse::<i64>()
            .map_err(|err| HlsError::Parse(format!("field '{field}' must be an integer: {err}"))),
        other => Err(HlsError::Parse(format!(
            "field '{field}' must be an integer or integer string, got {other}"
        ))),
    }
}

fn required_u64(value: &Value, field: &str) -> HlsResult<u64> {
    match value {
        Value::Number(number) => number
            .as_u64()
            .ok_or_else(|| HlsError::Parse(format!("field '{field}' is not representable as u64"))),
        Value::String(text) => text.parse::<u64>().map_err(|err| {
            HlsError::Parse(format!(
                "field '{field}' must be an unsigned integer: {err}"
            ))
        }),
        other => Err(HlsError::Parse(format!(
            "field '{field}' must be an unsigned integer or integer string, got {other}"
        ))),
    }
}

fn required_f64(value: &Value, field: &str) -> HlsResult<f64> {
    match value {
        Value::Number(number) => number
            .as_f64()
            .ok_or_else(|| HlsError::Parse(format!("field '{field}' is not representable as f64"))),
        Value::String(text) => text
            .parse::<f64>()
            .map_err(|err| HlsError::Parse(format!("field '{field}' must be numeric: {err}"))),
        other => Err(HlsError::Parse(format!(
            "field '{field}' must be a number or numeric string, got {other}"
        ))),
    }
}

fn validate_candle_fields(
    open_ts_ms: i64,
    close_ts_ms: i64,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
) -> HlsResult<()> {
    if open_ts_ms > close_ts_ms {
        return Err(HlsError::Parse(
            "candleSnapshot open time must be <= close time".to_owned(),
        ));
    }
    if open <= 0.0 || close <= 0.0 || high <= 0.0 || low <= 0.0 {
        return Err(HlsError::Parse(
            "candleSnapshot OHLC values must be positive".to_owned(),
        ));
    }
    if high < low || high < open || high < close || low > open || low > close {
        return Err(HlsError::Parse(
            "candleSnapshot OHLC values are internally inconsistent".to_owned(),
        ));
    }
    Ok(())
}

fn numeric_field(value: &Value, field: &str) -> HlsResult<Option<f64>> {
    let Some(raw) = value.get(field) else {
        return Ok(None);
    };

    let parsed = match raw {
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
    }?;

    match parsed {
        Some(value) if !value.is_finite() => {
            Err(HlsError::Parse(format!("field '{field}' must be finite")))
        }
        _ => Ok(parsed),
    }
}

fn numeric_field_positive(value: &Value, field: &str) -> HlsResult<Option<f64>> {
    let parsed = numeric_field(value, field)?;
    if parsed.is_some_and(|value| value < 0.0) {
        return Err(HlsError::Parse(format!(
            "field '{field}' must be positive or a zero missing-value sentinel"
        )));
    }
    Ok(parsed.filter(|value| *value > 0.0))
}

fn numeric_field_non_negative(value: &Value, field: &str) -> HlsResult<Option<f64>> {
    let parsed = numeric_field(value, field)?;
    if parsed.is_some_and(|value| value < 0.0) {
        return Err(HlsError::Parse(format!(
            "field '{field}' must be non-negative"
        )));
    }
    Ok(parsed)
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

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    use super::{
        parse_candle_snapshot, validate_candle_snapshot_response, validate_public_rest_base_url,
    };

    #[test]
    fn public_rest_base_url_rejects_query_and_fragment_components() {
        for base_url in [
            "https://api.hyperliquid.xyz?redirect=/info",
            "https://api.hyperliquid.xyz#info",
        ] {
            let error = validate_public_rest_base_url(base_url)
                .expect_err("base URL suffix components must fail closed");
            assert!(error.to_string().contains("query or fragment"));
        }
    }

    #[test]
    fn candle_snapshot_response_rejects_rows_outside_requested_window() {
        let candles = parse_candle_snapshot(
            r#"[{"t":1710000000000,"T":1710000059999,"s":"@107","i":"1m","o":"35.0","c":"35.2","h":"35.4","l":"34.9","v":"25.0","n":12}]"#,
        )
        .expect("candle parses");

        let error = validate_candle_snapshot_response(
            &candles,
            "@107",
            "1m",
            1_710_000_060_000,
            1_710_000_120_000,
        )
        .expect_err("out-of-window candle must fail closed");

        assert!(error.to_string().contains("outside requested range"));
    }

    #[test]
    fn retry_after_parses_delta_seconds_and_http_date_against_injected_time() {
        let now = time::OffsetDateTime::parse(
            "Wed, 21 Oct 2015 07:27:53 +0000",
            &time::format_description::well_known::Rfc2822,
        )
        .unwrap();

        assert_eq!(
            super::parse_retry_after_at("7", now),
            Some(Duration::from_secs(7))
        );
        assert_eq!(
            super::parse_retry_after_at("Wed, 21 Oct 2015 07:28:00 GMT", now),
            Some(Duration::from_secs(7))
        );
        assert_eq!(super::parse_retry_after_at("invalid", now), None);
    }

    #[tokio::test]
    async fn loopback_http_date_retry_after_is_preserved_as_typed_metadata() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let deadline = time::OffsetDateTime::now_utc() + time::Duration::seconds(120);
        let retry_after = deadline
            .format(&time::format_description::well_known::Rfc2822)
            .unwrap()
            .replace("+0000", "GMT");
        let response = format!(
            "HTTP/1.1 429 Too Many Requests\r\nRetry-After: {retry_after}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        );
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = vec![0_u8; 8_192];
            let _ = stream.read(&mut request).await.unwrap();
            stream.write_all(response.as_bytes()).await.unwrap();
            stream.shutdown().await.unwrap();
        });

        let error = super::HyperliquidRestClient::new(format!("http://{address}"))
            .candle_snapshot_attempt("@107", "1m", 0, 0)
            .await
            .expect_err("loopback returns 429");
        server.await.unwrap();

        let delay = error.retry_after().expect("HTTP-date is typed");
        assert!(delay > Duration::from_secs(60), "delay was {delay:?}");
        assert!(delay <= Duration::from_secs(120), "delay was {delay:?}");
    }
}
