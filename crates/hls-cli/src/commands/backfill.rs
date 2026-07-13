use std::{collections::BTreeMap, path::PathBuf};

use anyhow::bail;
use clap::Args;
use hls_core::{HlsError, market_state::CandleEvent};
use hls_hyperliquid::rest::HyperliquidRestClient;
use hls_store::backfill::{
    BackfillGapsOptions, BackfillGapsSummary, CandleBackfillRequest, CandleBackfillSource,
    PendingCandleBackfillRequest, backfill_public_gaps, pending_public_candle_requests,
};

pub const DEFAULT_REST_URL: &str = "https://api.hyperliquid.xyz";

#[derive(Clone, Debug, Args)]
pub struct BackfillArgs {
    #[arg(long)]
    pub run_id: String,

    #[arg(long, default_value = "1m")]
    pub interval: String,

    #[arg(long, default_value = DEFAULT_REST_URL)]
    pub rest_url: String,

    /// Retry gaps that already have an attempt for this candle interval.
    #[arg(long)]
    pub retry: bool,

    #[arg(long, default_value = ".hls")]
    pub data_dir: PathBuf,
}

pub async fn run(args: BackfillArgs) -> anyhow::Result<()> {
    let summary = execute(args).await?;
    print_summary(&summary);
    if summary.requests_failed > 0 {
        bail!(
            "{} public candle request(s) failed; unrepaired attempts were recorded",
            summary.requests_failed
        );
    }
    Ok(())
}

pub async fn execute(args: BackfillArgs) -> anyhow::Result<BackfillGapsSummary> {
    validate_rest_url(&args.rest_url)?;
    let options = BackfillGapsOptions::new(&args.data_dir, &args.run_id)
        .with_interval(&args.interval)
        .with_retry_existing(args.retry);
    let requests = pending_public_candle_requests(&options)?;
    let source = collect_public_candles(&args.rest_url, requests).await;
    Ok(backfill_public_gaps(options, &source)?)
}

pub fn print_summary(summary: &BackfillGapsSummary) {
    println!("backfill_run=complete");
    println!("run_id={}", summary.run_id);
    println!("gaps_examined={}", summary.gaps_examined);
    println!("gaps_repaired={}", summary.gaps_repaired);
    println!(
        "gaps_partially_repaired={}",
        summary.gaps_partially_repaired
    );
    println!("gaps_unrepaired={}", summary.gaps_unrepaired);
    println!("gaps_skipped={}", summary.gaps_skipped);
    println!("rows_written={}", summary.rows_written);
    println!("requests_failed={}", summary.requests_failed);
    println!("tick_gaps_recovered=0");
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct RequestKey {
    symbol: String,
    interval: String,
    start_time_ms: i64,
    end_time_ms: i64,
}

impl From<&PendingCandleBackfillRequest> for RequestKey {
    fn from(request: &PendingCandleBackfillRequest) -> Self {
        Self {
            symbol: request.symbol.clone(),
            interval: request.interval.clone(),
            start_time_ms: request.start_time_ms,
            end_time_ms: request.end_time_ms,
        }
    }
}

impl From<&CandleBackfillRequest<'_>> for RequestKey {
    fn from(request: &CandleBackfillRequest<'_>) -> Self {
        Self {
            symbol: request.symbol.to_owned(),
            interval: request.interval.to_owned(),
            start_time_ms: request.start_time_ms,
            end_time_ms: request.end_time_ms,
        }
    }
}

#[derive(Clone, Debug)]
enum CollectedCandleResult {
    Candles(Vec<CandleEvent>),
    Failed(String),
}

#[derive(Clone, Debug, Default)]
struct CollectedCandleSource {
    results: BTreeMap<RequestKey, CollectedCandleResult>,
}

impl CandleBackfillSource for CollectedCandleSource {
    fn candle_snapshot(
        &self,
        request: &CandleBackfillRequest<'_>,
    ) -> hls_core::HlsResult<Vec<CandleEvent>> {
        match self.results.get(&RequestKey::from(request)) {
            Some(CollectedCandleResult::Candles(candles)) => Ok(candles.clone()),
            Some(CollectedCandleResult::Failed(error)) => Err(HlsError::External(error.clone())),
            None => Err(HlsError::External(format!(
                "no collected candle response for {} {} [{}..={}]",
                request.symbol, request.interval, request.start_time_ms, request.end_time_ms
            ))),
        }
    }
}

async fn collect_public_candles(
    rest_url: &str,
    requests: Vec<PendingCandleBackfillRequest>,
) -> CollectedCandleSource {
    let client = HyperliquidRestClient::new(rest_url);
    let mut source = CollectedCandleSource::default();
    for request in requests {
        let result = client
            .candle_snapshot(
                &request.symbol,
                &request.interval,
                request.start_time_ms,
                request.end_time_ms,
            )
            .await
            .map(CollectedCandleResult::Candles)
            .unwrap_or_else(|error| CollectedCandleResult::Failed(error.to_string()));
        source.results.insert(RequestKey::from(&request), result);
    }
    source
}

pub(crate) fn validate_rest_url(rest_url: &str) -> anyhow::Result<()> {
    let allowed = rest_url.starts_with("https://")
        || rest_url.starts_with("http://127.0.0.1")
        || rest_url.starts_with("http://localhost")
        || rest_url.starts_with("http://[::1]");
    if !allowed {
        bail!("--rest-url must use HTTPS or an HTTP loopback address");
    }
    Ok(())
}
