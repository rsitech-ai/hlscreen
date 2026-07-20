use std::{collections::BTreeMap, future::Future, path::PathBuf, pin::Pin, time::Duration};

use anyhow::{Context, bail};
use clap::Args;
use hls_core::{HlsError, market_state::CandleEvent};
use hls_hyperliquid::rest::{
    HyperliquidRestClient, PublicRestError, validate_public_rest_base_url,
};
use hls_store::backfill::{
    BackfillGapsOptions, BackfillGapsSummary, CandleBackfillRequest, CandleBackfillSource,
    PendingCandleBackfillRequest, backfill_public_gaps, pending_public_candle_requests,
};

use crate::commands::ws_rate_limit::RollingRateLimiter;

pub const DEFAULT_REST_URL: &str = "https://api.hyperliquid.xyz";
const REST_RATE_WINDOW: Duration = Duration::from_secs(60);
// Keep 100 weighted units (8.3%) below Hyperliquid's official 1,200/minute ceiling.
const REST_RATE_BUDGET: usize = 1_100;
const CANDLE_SNAPSHOT_BASE_WEIGHT: usize = 20;
const CANDLES_PER_ADDITIONAL_WEIGHT: usize = 60;
// Hyperliquid documents that only the most recent 5,000 candles are available.
// Treating that availability bound as the maximum possible response is conservative.
const MAX_CANDLE_SNAPSHOT_CANDLES: usize = 5_000;
const MAX_429_RETRIES: usize = 2;
const DEFAULT_RETRY_DELAY: Duration = Duration::from_secs(1);
const MAX_RETRY_DELAY: Duration = Duration::from_secs(30);

type BackfillShutdown = Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send>>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BackfillRunOutcome {
    Completed,
    Stopped,
}

#[derive(Clone, Debug, Args)]
pub struct BackfillArgs {
    /// Recorded run whose unresolved gaps should be examined.
    #[arg(long)]
    pub run_id: String,

    /// Public candle interval used for coarse gap coverage.
    #[arg(long, default_value = "1m")]
    pub interval: String,

    /// HTTPS public REST base URL, or an HTTP loopback URL for tests.
    #[arg(long, default_value = DEFAULT_REST_URL)]
    pub rest_url: String,

    /// Retry gaps that already have an attempt for this candle interval.
    #[arg(long)]
    pub retry: bool,

    /// Local recording directory.
    #[arg(long, default_value = ".hls")]
    pub data_dir: PathBuf,
}

pub async fn run(args: BackfillArgs) -> anyhow::Result<()> {
    run_with_outcome(args).await.map(|_| ())
}

pub(crate) async fn run_with_outcome(args: BackfillArgs) -> anyhow::Result<BackfillRunOutcome> {
    let shutdown = install_backfill_shutdown_signal()?;
    let Some(summary) = execute_with_cancellation(args, shutdown).await? else {
        eprintln!("backfill_run=stopped stop_reason=signal");
        return Ok(BackfillRunOutcome::Stopped);
    };
    print_summary(&summary);
    if summary.requests_failed > 0 {
        bail!(
            "{} public candle request(s) failed; unrepaired attempts were recorded",
            summary.requests_failed
        );
    }
    Ok(BackfillRunOutcome::Completed)
}

pub(crate) async fn execute_with_cancellation<F>(
    args: BackfillArgs,
    cancellation: F,
) -> anyhow::Result<Option<BackfillGapsSummary>>
where
    F: Future<Output = anyhow::Result<()>>,
{
    validate_rest_url(&args.rest_url)?;
    let options = BackfillGapsOptions::new(&args.data_dir, &args.run_id)
        .with_interval(&args.interval)
        .with_retry_existing(args.retry);
    let requests = pending_public_candle_requests(&options)?;
    let Some(source) = collect_public_candles(&args.rest_url, requests, cancellation).await? else {
        return Ok(None);
    };
    Ok(Some(backfill_public_gaps(options, &source)?))
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
    cancellation: impl Future<Output = anyhow::Result<()>>,
) -> anyhow::Result<Option<CollectedCandleSource>> {
    let client = HyperliquidRestClient::new(rest_url);
    let mut source = CollectedCandleSource::default();
    let mut limiter = RollingRateLimiter::new(REST_RATE_BUDGET, REST_RATE_WINDOW);
    tokio::pin!(cancellation);
    for request in requests {
        let weight = candle_snapshot_request_weight(
            &request.interval,
            request.start_time_ms,
            request.end_time_ms,
        )?;
        let mut retry_count = 0_usize;
        let result = loop {
            if let Some(available_at) =
                limiter.next_available_at_for(tokio::time::Instant::now(), weight)?
            {
                if wait_or_cancel(tokio::time::sleep_until(available_at), &mut cancellation)
                    .await?
                    .is_none()
                {
                    return Ok(None);
                }
            }
            limiter.record_weight(tokio::time::Instant::now(), weight)?;

            let attempt = client.candle_snapshot_attempt(
                &request.symbol,
                &request.interval,
                request.start_time_ms,
                request.end_time_ms,
            );
            let Some(attempt) = wait_or_cancel(attempt, &mut cancellation).await? else {
                return Ok(None);
            };
            match attempt {
                Ok(candles) => break CollectedCandleResult::Candles(candles),
                Err(error) if is_too_many_requests(&error) && retry_count < MAX_429_RETRIES => {
                    let delay = retry_delay(error.retry_after(), retry_count);
                    retry_count += 1;
                    if wait_or_cancel(tokio::time::sleep(delay), &mut cancellation)
                        .await?
                        .is_none()
                    {
                        return Ok(None);
                    }
                }
                Err(error) => {
                    break CollectedCandleResult::Failed(format!(
                        "public candleSnapshot failed after {} attempt(s): {error}",
                        retry_count + 1
                    ));
                }
            }
        };
        source.results.insert(RequestKey::from(&request), result);
    }
    Ok(Some(source))
}

async fn wait_or_cancel<T>(
    operation: impl Future<Output = T>,
    cancellation: &mut Pin<&mut impl Future<Output = anyhow::Result<()>>>,
) -> anyhow::Result<Option<T>> {
    tokio::pin!(operation);
    tokio::select! {
        biased;
        result = cancellation.as_mut() => result.map(|()| None),
        value = &mut operation => Ok(Some(value)),
    }
}

fn is_too_many_requests(error: &PublicRestError) -> bool {
    error.is_too_many_requests()
}

fn retry_delay(retry_after: Option<Duration>, retry_count: usize) -> Duration {
    retry_after
        .unwrap_or_else(|| DEFAULT_RETRY_DELAY.saturating_mul(1_u32 << retry_count.min(4)))
        .min(MAX_RETRY_DELAY)
}

fn candle_snapshot_request_weight(
    interval: &str,
    start_time_ms: i64,
    end_time_ms: i64,
) -> anyhow::Result<usize> {
    if start_time_ms > end_time_ms {
        bail!("candle snapshot start_time_ms must be <= end_time_ms");
    }
    let interval_ms = candle_interval_millis(interval)
        .with_context(|| format!("unsupported candle interval '{interval}'"))?;
    let span_ms = i128::from(end_time_ms) - i128::from(start_time_ms);
    let estimated = (span_ms / i128::from(interval_ms) + 1)
        .clamp(1, MAX_CANDLE_SNAPSHOT_CANDLES as i128) as usize;
    Ok(CANDLE_SNAPSHOT_BASE_WEIGHT + estimated.div_ceil(CANDLES_PER_ADDITIONAL_WEIGHT))
}

fn candle_interval_millis(interval: &str) -> Option<i64> {
    let minute = 60_000_i64;
    Some(match interval {
        "1m" => minute,
        "3m" => 3 * minute,
        "5m" => 5 * minute,
        "15m" => 15 * minute,
        "30m" => 30 * minute,
        "1h" => 60 * minute,
        "2h" => 120 * minute,
        "4h" => 240 * minute,
        "8h" => 480 * minute,
        "12h" => 720 * minute,
        "1d" => 1_440 * minute,
        "3d" => 4_320 * minute,
        "1w" => 10_080 * minute,
        // Use the shortest calendar month so the estimate never undercounts.
        "1M" => 40_320 * minute,
        _ => return None,
    })
}

fn install_backfill_shutdown_signal() -> anyhow::Result<BackfillShutdown> {
    #[cfg(unix)]
    {
        let mut interrupt =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())
                .context("install SIGINT listener for backfill")?;
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .context("install SIGTERM listener for backfill")?;
        Ok(Box::pin(async move {
            tokio::select! {
                signal = interrupt.recv() => signal.context("SIGINT listener closed before delivery").map(|_| ()),
                signal = terminate.recv() => signal.context("SIGTERM listener closed before delivery").map(|_| ()),
            }
        }))
    }
    #[cfg(not(unix))]
    {
        Ok(Box::pin(async {
            tokio::signal::ctrl_c()
                .await
                .context("wait for backfill shutdown signal")
        }))
    }
}

pub(crate) fn validate_rest_url(rest_url: &str) -> anyhow::Result<()> {
    validate_public_rest_base_url(rest_url)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    fn request() -> PendingCandleBackfillRequest {
        PendingCandleBackfillRequest {
            gap_id: "gap-1".to_owned(),
            symbol: "@107".to_owned(),
            interval: "1m".to_owned(),
            start_time_ms: 0,
            end_time_ms: 0,
        }
    }

    async fn loopback_server(
        responses: Vec<&'static str>,
    ) -> (String, Arc<AtomicUsize>, tokio::task::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let requests = Arc::new(AtomicUsize::new(0));
        let request_count = Arc::clone(&requests);
        let task = tokio::spawn(async move {
            for response in responses {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut request = vec![0_u8; 8_192];
                let _ = stream.read(&mut request).await.unwrap();
                request_count.fetch_add(1, Ordering::SeqCst);
                stream.write_all(response.as_bytes()).await.unwrap();
                stream.shutdown().await.unwrap();
            }
        });
        (format!("http://{address}"), requests, task)
    }

    #[test]
    fn candle_snapshot_weight_counts_interval_boundaries_conservatively() {
        let minute_ms = 60_000;

        assert_eq!(candle_snapshot_request_weight("1m", 0, 0).unwrap(), 21);
        assert_eq!(
            candle_snapshot_request_weight("1m", 0, 59 * minute_ms).unwrap(),
            21
        );
        assert_eq!(
            candle_snapshot_request_weight("1m", 0, 60 * minute_ms).unwrap(),
            22
        );
    }

    #[test]
    fn candle_snapshot_weight_caps_the_worst_case_at_five_thousand_candles() {
        assert_eq!(
            candle_snapshot_request_weight("1m", 0, i64::MAX).unwrap(),
            104
        );
        assert!(candle_snapshot_request_weight("not-an-interval", 0, 1).is_err());
        assert!(candle_snapshot_request_weight("1m", 2, 1).is_err());
    }

    #[test]
    fn retry_after_is_bounded_even_when_the_server_requests_a_long_delay() {
        assert_eq!(
            retry_delay(Some(Duration::from_secs(1_000)), 0),
            MAX_RETRY_DELAY
        );
        assert_eq!(retry_delay(None, 0), DEFAULT_RETRY_DELAY);
        assert_eq!(
            retry_delay(Some(Duration::from_secs(7)), 0),
            Duration::from_secs(7)
        );
    }

    #[tokio::test]
    async fn loopback_429_retries_are_bounded_and_other_4xx_are_not_retried() {
        const RATE_LIMITED: &str = "HTTP/1.1 429 Too Many Requests\r\nRetry-After: 0\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
        const BAD_REQUEST: &str =
            "HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";

        let (url, count, server) = loopback_server(vec![RATE_LIMITED; 3]).await;
        let source = collect_public_candles(&url, vec![request()], std::future::pending())
            .await
            .unwrap()
            .unwrap();
        server.await.unwrap();
        assert_eq!(count.load(Ordering::SeqCst), MAX_429_RETRIES + 1);
        assert!(matches!(
            source.results.values().next(),
            Some(CollectedCandleResult::Failed(error)) if error.contains("after 3 attempt(s)")
        ));

        let (url, count, server) = loopback_server(vec![BAD_REQUEST]).await;
        let _ = collect_public_candles(&url, vec![request()], std::future::pending())
            .await
            .unwrap()
            .unwrap();
        server.await.unwrap();
        assert_eq!(count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn loopback_429_retry_can_recover_and_each_attempt_reaches_the_server() {
        const RATE_LIMITED: &str = "HTTP/1.1 429 Too Many Requests\r\nRetry-After: 0\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
        const OK: &str = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n[]";
        let (url, count, server) = loopback_server(vec![RATE_LIMITED, OK]).await;

        let source = collect_public_candles(&url, vec![request()], std::future::pending())
            .await
            .unwrap()
            .unwrap();
        server.await.unwrap();

        assert_eq!(count.load(Ordering::SeqCst), 2);
        assert!(matches!(
            source.results.values().next(),
            Some(CollectedCandleResult::Candles(candles)) if candles.is_empty()
        ));
    }

    #[tokio::test(start_paused = true)]
    async fn retry_wait_exits_immediately_on_shutdown() {
        let started = tokio::time::Instant::now();
        let mut cancellation = Box::pin(std::future::ready(Ok(())));
        let outcome = wait_or_cancel(
            tokio::time::sleep(Duration::from_secs(60)),
            &mut cancellation.as_mut(),
        )
        .await
        .unwrap();

        assert!(outcome.is_none());
        assert_eq!(tokio::time::Instant::now(), started);
    }

    #[tokio::test(start_paused = true)]
    async fn meaningful_retry_after_delay_is_honored_without_wall_clock_sleep() {
        let started = tokio::time::Instant::now();
        let mut cancellation = Box::pin(std::future::pending::<anyhow::Result<()>>());
        let mut cancellation = cancellation.as_mut();
        let wait = wait_or_cancel(
            tokio::time::sleep(retry_delay(Some(Duration::from_secs(7)), 0)),
            &mut cancellation,
        );
        tokio::pin!(wait);
        tokio::select! {
            result = &mut wait => panic!("Retry-After released early: {result:?}"),
            _ = tokio::time::sleep(Duration::from_secs(6)) => {}
        }
        assert!(wait.await.unwrap().is_some());
        assert_eq!(
            tokio::time::Instant::now(),
            started + Duration::from_secs(7)
        );
    }

    #[tokio::test]
    async fn loopback_retry_after_delta_seconds_is_preserved_as_typed_metadata() {
        const RATE_LIMITED: &str = "HTTP/1.1 429 Too Many Requests\r\nRetry-After: 7\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
        let (url, count, server) = loopback_server(vec![RATE_LIMITED]).await;

        let error = HyperliquidRestClient::new(url)
            .candle_snapshot_attempt("@107", "1m", 0, 0)
            .await
            .expect_err("loopback returns 429");
        server.await.unwrap();

        assert_eq!(count.load(Ordering::SeqCst), 1);
        assert_eq!(error.retry_after(), Some(Duration::from_secs(7)));
    }
}
