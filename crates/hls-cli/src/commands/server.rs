use std::{fs, future::Future, io, net::SocketAddr, path::PathBuf, pin::Pin, time::Duration};

use anyhow::{Context, bail};
use clap::Args;
use futures_util::{Sink, SinkExt, StreamExt};
use hls_core::{
    HlsError,
    health::{
        ConnectionHealth, ConnectionState, HealthInputs, ReadOnlySafety, RecordingHealth,
        WriterHealth,
    },
    market_state::{LiveMarketState, MarketEvent},
    time::now_millis,
};
use hls_hyperliquid::{
    rest::{HyperliquidRestClient, SpotMarketContext, select_universe},
    ws::{
        connection::ReconnectPolicy,
        parser::{parse_ws_message, parse_ws_ndjson},
        subscriptions::{
            OFFICIAL_WS_SUBSCRIPTION_LIMIT, StreamKind, SubscriptionPlan, ping_message,
        },
        validate_public_ws_url,
    },
};
use hls_server::{ApiState, SharedApiState, handle_get, serve_shared_until_shutdown};
use tokio::{
    net::TcpListener,
    sync::oneshot,
    time::{MissedTickBehavior, interval},
};
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::commands::{
    fees::{feature_engine, load_fee_profile},
    health::simulated_health,
    metadata::{attach_metadata, load_metadata_enrichments},
    record::parse_symbols,
    ws_rate_limit::{RollingMessageRateLimiter, RollingRateLimiter, WS_OUTBOUND_RATE_WINDOW},
};

const DEFAULT_WS_URL: &str = "wss://api.hyperliquid.xyz/ws";
const DEFAULT_LIVE_DURATION_SECS: u64 = 60;
const DEFAULT_REFRESH_SECS: u64 = 5;
const DEFAULT_MAX_SUBSCRIPTIONS: usize = 980;
const LIVE_API_PUBLISH_INTERVAL_MS: u64 = 250;
const SERVER_RECONNECT_INITIAL_BACKOFF_MS: u64 = 1_000;
const SERVER_RECONNECT_MAX_BACKOFF_MS: u64 = 30_000;
const SERVER_CONNECTION_RATE_BUDGET: usize = 29;

type ShutdownSignal = Pin<Box<dyn Future<Output = anyhow::Result<ServerStopReason>> + Send>>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ServerStopReason {
    Signal(&'static str),
}

impl ServerStopReason {
    fn label(self) -> &'static str {
        match self {
            Self::Signal(signal) => signal,
        }
    }
}

#[derive(Debug, Args)]
pub struct ServerArgs {
    /// Address for the read-only localhost API server.
    #[arg(long, default_value = "127.0.0.1:0")]
    pub bind: String,

    /// Print the /health response once instead of starting the server loop.
    #[arg(long)]
    pub print_health: bool,

    /// Start a bounded live-updating read-only API backed by public market data.
    #[arg(long)]
    pub live: bool,

    /// Comma-separated display names or feed identifiers for the live preview.
    #[arg(long)]
    pub symbols: Option<String>,

    /// Maximum number of volume-ranked symbols when explicit symbols are absent.
    #[arg(long, default_value_t = 50)]
    pub top: usize,

    /// Stream every currently available spot symbol within subscription limits.
    #[arg(long)]
    pub all_symbols: bool,

    /// Stop a live preview after this many seconds; zero runs until a signal.
    #[arg(long, default_value_t = DEFAULT_LIVE_DURATION_SECS)]
    pub duration_secs: u64,

    /// Seconds between live health refreshes.
    #[arg(long, default_value_t = DEFAULT_REFRESH_SECS)]
    pub refresh_secs: u64,

    /// Maximum public WebSocket subscriptions allowed in the selected plan.
    #[arg(long, default_value_t = DEFAULT_MAX_SUBSCRIPTIONS)]
    pub max_subscriptions: usize,

    /// Secure public WebSocket URL, or a cleartext loopback URL for tests.
    #[arg(long, default_value = DEFAULT_WS_URL)]
    pub ws_url: String,

    #[arg(long, hide = true)]
    pub fixture_file: Option<PathBuf>,

    #[arg(long, hide = true)]
    pub metadata_file: Option<PathBuf>,

    /// Apply an explicit local JSON/TOML fee profile to live API screen rows.
    #[arg(long)]
    pub fee_profile_file: Option<PathBuf>,

    #[arg(long, hide = true)]
    pub simulate_health: Option<String>,
}

pub async fn run(args: ServerArgs) -> anyhow::Result<()> {
    let health = simulated_health(args.simulate_health.as_deref())?;
    if args.print_health {
        let state = ApiState::new(health, Vec::new());
        let response = handle_get("/health", "", &state)?;
        println!("{}", response.body);
        return Ok(());
    }
    let bind = parse_loopback_bind(&args.bind)?;

    if args.live {
        return run_live_server(args, bind).await;
    }

    let shutdown_signal = install_shutdown_signal()?;
    let listener = TcpListener::bind(bind).await?;
    let address = listener.local_addr()?;
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let mut server = tokio::spawn(serve_shared_until_shutdown(
        listener,
        SharedApiState::new(ApiState::new(health, Vec::new())),
        async move {
            let _ = shutdown_rx.await;
        },
    ));
    eprintln!("hls server listening on http://{address} (read-only, SIGINT/SIGTERM to stop)");

    let stop_result = tokio::select! {
        signal_result = shutdown_signal => signal_result,
        joined = &mut server => {
            map_http_server_join(joined)?;
            bail!("HTTP API server stopped before a shutdown signal was delivered");
        }
    };
    let _ = shutdown_tx.send(());
    let server_result = await_http_server(server).await;
    let stop_reason = combine_signal_and_server_result(stop_result, server_result)?;
    eprintln!("hls server stopped cleanly after {}", stop_reason.label());
    Ok(())
}

fn shutdown_listener_setup<T>(result: io::Result<T>) -> anyhow::Result<T> {
    result.context("install server shutdown signal listener")
}

fn signal_stop_reason(
    signal: &'static str,
    delivery: Option<()>,
) -> anyhow::Result<ServerStopReason> {
    delivery
        .with_context(|| format!("{signal} server shutdown listener closed before delivery"))
        .map(|()| ServerStopReason::Signal(signal))
}

fn install_shutdown_signal() -> anyhow::Result<ShutdownSignal> {
    #[cfg(unix)]
    {
        let mut interrupt = shutdown_listener_setup(tokio::signal::unix::signal(
            tokio::signal::unix::SignalKind::interrupt(),
        ))?;
        let mut terminate = shutdown_listener_setup(tokio::signal::unix::signal(
            tokio::signal::unix::SignalKind::terminate(),
        ))?;
        Ok(Box::pin(async move {
            tokio::select! {
                delivery = interrupt.recv() => signal_stop_reason("SIGINT", delivery),
                delivery = terminate.recv() => signal_stop_reason("SIGTERM", delivery),
            }
        }))
    }

    #[cfg(windows)]
    {
        let mut interrupt = shutdown_listener_setup(tokio::signal::windows::ctrl_c())?;
        Ok(Box::pin(async move {
            signal_stop_reason("CTRL-C", interrupt.recv().await)
        }))
    }

    #[cfg(not(any(unix, windows)))]
    {
        Ok(Box::pin(async {
            tokio::signal::ctrl_c()
                .await
                .context("wait for server CTRL-C shutdown signal")?;
            Ok(ServerStopReason::Signal("CTRL-C"))
        }))
    }
}

async fn await_http_server(
    server: tokio::task::JoinHandle<hls_core::HlsResult<()>>,
) -> anyhow::Result<()> {
    map_http_server_join(server.await)
}

fn map_http_server_join(
    joined: Result<hls_core::HlsResult<()>, tokio::task::JoinError>,
) -> anyhow::Result<()> {
    joined
        .map_err(|err| HlsError::External(format!("HTTP API server task failed: {err}")))?
        .map_err(anyhow::Error::from)
}

fn combine_signal_and_server_result(
    signal_result: anyhow::Result<ServerStopReason>,
    server_result: anyhow::Result<()>,
) -> anyhow::Result<ServerStopReason> {
    match (signal_result, server_result) {
        (Ok(reason), Ok(())) => Ok(reason),
        (Ok(_), Err(server_error)) => Err(server_error),
        (Err(signal_error), Ok(())) => Err(signal_error),
        (Err(signal_error), Err(server_error)) => {
            Err(signal_error.context(format!("HTTP server shutdown also failed: {server_error}")))
        }
    }
}

fn parse_loopback_bind(raw: &str) -> anyhow::Result<SocketAddr> {
    let address = raw
        .parse::<SocketAddr>()
        .with_context(|| format!("parse --bind address '{raw}'"))?;
    if !address.ip().is_loopback() {
        bail!("--bind must use a loopback address; non-loopback exposure is unsupported");
    }
    Ok(address)
}

async fn run_live_server(args: ServerArgs, bind: SocketAddr) -> anyhow::Result<()> {
    validate_live_server_args(&args)?;
    let fee_profile = load_fee_profile(args.fee_profile_file.as_ref())?;
    let shutdown_signal = install_shutdown_signal()?;
    let listener = TcpListener::bind(bind).await?;
    let address = listener.local_addr()?;
    let shared = SharedApiState::new(ApiState::new(connecting_health().snapshot(), Vec::new()));
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let mut server = tokio::spawn(serve_shared_until_shutdown(
        listener,
        shared.clone(),
        async move {
            let _ = shutdown_rx.await;
        },
    ));

    eprintln!(
        "hls live server listening on http://{address} (read-only, duration_secs={})",
        args.duration_secs
    );

    let publisher = async {
        if let Some(fixture_file) = &args.fixture_file {
            publish_fixture_live_api(&args, fixture_file, &shared, fee_profile.as_ref())
        } else {
            publish_network_live_api(&args, &shared, fee_profile.as_ref()).await
        }
    };
    let completion = wait_for_live_completion(publisher, shutdown_signal, &mut server).await;

    let _ = shutdown_tx.send(());
    let summary = match completion {
        LiveServerCompletion::Published(publish_result) => {
            let server_result = await_http_server(server).await;
            match (publish_result, server_result) {
                (Ok(summary), Ok(())) => summary,
                (Ok(_), Err(server_error)) => return Err(server_error),
                (Err(publish_error), Ok(())) => return Err(publish_error),
                (Err(publish_error), Err(server_error)) => {
                    return Err(publish_error.context(format!(
                        "live API server shutdown also failed: {server_error}"
                    )));
                }
            }
        }
        LiveServerCompletion::Signal(signal_result) => {
            let server_result = await_http_server(server).await;
            let stop_reason = combine_signal_and_server_result(signal_result, server_result)?;
            eprintln!(
                "hls live server stopped cleanly after {}",
                stop_reason.label()
            );
            println!("server_live_run=stopped");
            println!("listen=http://{address}");
            println!("stop_reason={}", stop_reason.label());
            return Ok(());
        }
        LiveServerCompletion::HttpStopped(server_result) => {
            server_result?;
            bail!("HTTP API server stopped before live publication completed");
        }
    };

    println!("server_live_run=complete");
    println!("listen=http://{address}");
    println!("symbols={}", summary.symbols);
    println!("subscriptions={}", summary.subscriptions);
    println!("ws_messages={}", summary.ws_messages);
    println!("market_events={}", summary.market_events);
    println!("rows={}", summary.rows);
    println!("reconnects={}", summary.reconnects);
    println!("data_gaps={}", summary.data_gaps);
    Ok(())
}

enum LiveServerCompletion {
    Published(anyhow::Result<ServerLiveSummary>),
    Signal(anyhow::Result<ServerStopReason>),
    HttpStopped(anyhow::Result<()>),
}

async fn wait_for_live_completion<P>(
    publisher: P,
    mut shutdown_signal: ShutdownSignal,
    server: &mut tokio::task::JoinHandle<hls_core::HlsResult<()>>,
) -> LiveServerCompletion
where
    P: Future<Output = anyhow::Result<ServerLiveSummary>>,
{
    tokio::pin!(publisher);
    tokio::select! {
        biased;
        joined = server => LiveServerCompletion::HttpStopped(map_http_server_join(joined)),
        signal_result = shutdown_signal.as_mut() => LiveServerCompletion::Signal(signal_result),
        publish_result = &mut publisher => LiveServerCompletion::Published(publish_result),
    }
}

fn validate_live_server_args(args: &ServerArgs) -> anyhow::Result<()> {
    if args.duration_secs == 0 {
        bail!("--duration-secs must be greater than zero");
    }
    if args.refresh_secs == 0 {
        bail!("--refresh-secs must be greater than zero");
    }
    if args.top == 0 {
        bail!("--top must be greater than zero");
    }
    if args.max_subscriptions == 0 {
        bail!("--max-subscriptions must be greater than zero");
    }
    if args.max_subscriptions > OFFICIAL_WS_SUBSCRIPTION_LIMIT {
        bail!(
            "--max-subscriptions cannot exceed the official IP-wide limit of {OFFICIAL_WS_SUBSCRIPTION_LIMIT}"
        );
    }
    if args.all_symbols && args.symbols.is_some() {
        bail!("--symbols and --all-symbols are mutually exclusive");
    }
    if args
        .symbols
        .as_deref()
        .is_some_and(|symbols| parse_symbols(Some(symbols)).is_empty())
    {
        bail!("--symbols must contain at least one non-empty selector");
    }
    if args.fixture_file.is_none() {
        validate_public_ws_url(&args.ws_url)?;
    }
    if tokio::time::Instant::now()
        .checked_add(Duration::from_secs(args.duration_secs))
        .is_none()
    {
        bail!(
            "--duration-secs value {} is too large for this runtime",
            args.duration_secs
        );
    }
    Ok(())
}

fn publish_fixture_live_api(
    args: &ServerArgs,
    fixture_file: &PathBuf,
    shared: &SharedApiState,
    fee_profile: Option<&hls_core::fees::FeeProfile>,
) -> anyhow::Result<ServerLiveSummary> {
    let raw = fs::read_to_string(fixture_file)
        .with_context(|| format!("read {}", fixture_file.display()))?;
    let events = parse_ws_ndjson(&raw)?;
    let symbols = fixture_symbols(args, &events);
    let market_events = events.len() as u64;
    let mut state = LiveMarketState::new(symbols.clone());
    for event in events {
        state.apply(event)?;
    }
    let metadata = load_metadata_enrichments(args.metadata_file.as_ref())?;
    let mut summary = ServerLiveSummary {
        symbols: symbols.len(),
        subscriptions: symbols.len(),
        ws_messages: raw.lines().filter(|line| !line.trim().is_empty()).count() as u64,
        market_events,
        ..ServerLiveSummary::default()
    };
    publish_api_snapshot(
        shared,
        &state,
        &metadata,
        fee_profile,
        ServerHealthStats {
            connection_state: ConnectionState::Connected,
            subscriptions: summary.subscriptions,
            last_message_age_ms: Some(0),
            reconnects: 0,
            data_gaps: 0,
            last_reconnect_backoff_ms: None,
            rows_written: summary.market_events,
        },
        &mut summary,
    )?;
    summary.require_market_data()
}

async fn publish_network_live_api(
    args: &ServerArgs,
    shared: &SharedApiState,
    fee_profile: Option<&hls_core::fees::FeeProfile>,
) -> anyhow::Result<ServerLiveSummary> {
    let selection = load_server_live_symbols(args).await?;
    let symbols = selection.symbols;
    let plan = server_subscription_plan(symbols.clone(), args.all_symbols, args.max_subscriptions);
    let subscription_messages = plan.subscribe_messages()?;
    let mut metadata = selection.metadata;
    metadata.extend(load_metadata_enrichments(args.metadata_file.as_ref())?);
    let mut state = LiveMarketState::new(symbols.clone());
    let mut summary = ServerLiveSummary {
        symbols: symbols.len(),
        subscriptions: subscription_messages.len(),
        ..ServerLiveSummary::default()
    };
    let deadline = tokio::time::Instant::now() + Duration::from_secs(args.duration_secs);
    let reconnect_policy = ReconnectPolicy {
        initial_backoff_ms: SERVER_RECONNECT_INITIAL_BACKOFF_MS,
        max_backoff_ms: SERVER_RECONNECT_MAX_BACKOFF_MS,
        multiplier: 2,
    };
    let mut reconnect_attempt = 0;
    let mut outbound_rate_limiter = RollingMessageRateLimiter::default();
    let mut connection_rate_limiter =
        RollingRateLimiter::new(SERVER_CONNECTION_RATE_BUDGET, WS_OUTBOUND_RATE_WINDOW);

    while tokio::time::Instant::now() < deadline {
        let market_events_before_connection = summary.market_events;
        match drive_server_live_connection(
            args,
            &subscription_messages,
            shared,
            &mut state,
            &metadata,
            fee_profile,
            deadline,
            &mut summary,
            &mut outbound_rate_limiter,
            &mut connection_rate_limiter,
        )
        .await
        {
            Ok(()) => break,
            Err(error) => {
                summary.reconnects = summary.reconnects.saturating_add(1);
                summary.data_gaps = summary.data_gaps.saturating_add(1);
                let recovered_market_data = summary.market_events > market_events_before_connection;
                let backoff_ms = next_server_reconnect_backoff(
                    reconnect_policy,
                    &mut reconnect_attempt,
                    recovered_market_data,
                );
                summary.last_reconnect_backoff_ms = Some(backoff_ms);
                eprintln!(
                    "live API reconnect: reason={} reconnects={} data_gaps={} backoff_ms={}",
                    error, summary.reconnects, summary.data_gaps, backoff_ms
                );
                publish_api_snapshot(
                    shared,
                    &state,
                    &metadata,
                    fee_profile,
                    ServerHealthStats {
                        connection_state: ConnectionState::Reconnecting,
                        subscriptions: summary.subscriptions,
                        last_message_age_ms: None,
                        reconnects: summary.reconnects,
                        data_gaps: summary.data_gaps,
                        last_reconnect_backoff_ms: summary.last_reconnect_backoff_ms,
                        rows_written: summary.market_events,
                    },
                    &mut summary,
                )?;
                if !sleep_before_deadline(Duration::from_millis(backoff_ms), deadline).await {
                    break;
                }
            }
        }
    }

    summary.require_market_data()
}

fn server_subscription_plan(
    symbols: Vec<String>,
    all_symbols: bool,
    max_subscriptions: usize,
) -> SubscriptionPlan {
    let mut plan = SubscriptionPlan::new(symbols).with_max_subscriptions(max_subscriptions);
    if all_symbols && plan.subscription_count() > max_subscriptions {
        plan = plan.with_streams([
            StreamKind::AllMids,
            StreamKind::Trades,
            StreamKind::Bbo,
            StreamKind::ActiveAssetCtx,
        ]);
        if plan.subscription_count() > max_subscriptions {
            plan = plan.with_streams([StreamKind::AllMids, StreamKind::ActiveAssetCtx]);
            if plan.subscription_count() > max_subscriptions {
                plan = plan.with_streams([StreamKind::AllMids]);
            }
        }
    }
    plan
}

fn next_server_reconnect_backoff(
    policy: ReconnectPolicy,
    attempt: &mut u64,
    recovered_market_data: bool,
) -> u64 {
    if recovered_market_data {
        *attempt = 0;
    }
    let backoff_ms = policy.backoff_ms(*attempt);
    *attempt = attempt.saturating_add(1);
    backoff_ms
}

#[allow(clippy::too_many_arguments)]
async fn drive_server_live_connection(
    args: &ServerArgs,
    subscription_messages: &[String],
    shared: &SharedApiState,
    state: &mut LiveMarketState,
    metadata: &[hls_core::metadata::MetadataEnrichment],
    fee_profile: Option<&hls_core::fees::FeeProfile>,
    deadline: tokio::time::Instant,
    summary: &mut ServerLiveSummary,
    outbound_rate_limiter: &mut RollingMessageRateLimiter,
    connection_rate_limiter: &mut RollingRateLimiter,
) -> anyhow::Result<()> {
    if !wait_for_rate_limit_slot(connection_rate_limiter, deadline).await {
        publish_final_api_snapshot(
            shared,
            state,
            metadata,
            fee_profile,
            summary,
            None,
            ConnectionState::Connecting,
        )?;
        return Ok(());
    }
    connection_rate_limiter.record(tokio::time::Instant::now());
    let connected = tokio::select! {
        connected = connect_async(&args.ws_url) => Some(connected),
        _ = tokio::time::sleep_until(deadline) => None,
    };
    let Some(connected) = connected else {
        publish_final_api_snapshot(
            shared,
            state,
            metadata,
            fee_profile,
            summary,
            None,
            ConnectionState::Connecting,
        )?;
        return Ok(());
    };
    let (ws, _) = connected.with_context(|| format!("connect {}", args.ws_url))?;
    let connected_at_ms = now_ms_i64()?;
    let (mut write, mut read) = ws.split();
    for message in subscription_messages {
        if !send_rate_limited(
            &mut write,
            Message::Text(message.clone().into()),
            outbound_rate_limiter,
            deadline,
            "send live API subscription",
        )
        .await?
        {
            publish_final_api_snapshot(
                shared,
                state,
                metadata,
                fee_profile,
                summary,
                None,
                ConnectionState::Connected,
            )?;
            return Ok(());
        }
    }

    let mut heartbeat = interval(Duration::from_secs(20));
    heartbeat.tick().await;
    let mut refresh = interval(Duration::from_secs(args.refresh_secs.max(1)));
    refresh.tick().await;
    let mut publish = live_api_publish_interval();
    publish.tick().await;
    let mut last_message_at_ms = Some(connected_at_ms);
    let mut dirty = false;

    loop {
        tokio::select! {
            _ = tokio::time::sleep_until(deadline) => {
                let age = last_message_at_ms.and_then(|last| message_age_ms(last).ok());
                publish_api_snapshot(
                    shared,
                    state,
                    metadata,
                    fee_profile,
                    ServerHealthStats {
                        connection_state: ConnectionState::Connected,
                        subscriptions: summary.subscriptions,
                        last_message_age_ms: age,
                        reconnects: summary.reconnects,
                        data_gaps: summary.data_gaps,
                        last_reconnect_backoff_ms: summary.last_reconnect_backoff_ms,
                        rows_written: summary.market_events,
                    },
                    summary,
                )?;
                return Ok(());
            },
            _ = heartbeat.tick() => {
                if !send_rate_limited(
                    &mut write,
                    Message::Text(ping_message().to_owned().into()),
                    outbound_rate_limiter,
                    deadline,
                    "send live API heartbeat",
                ).await? {
                    publish_final_api_snapshot(
                        shared,
                        state,
                        metadata,
                        fee_profile,
                        summary,
                        last_message_at_ms,
                        ConnectionState::Connected,
                    )?;
                    return Ok(());
                }
            }
            _ = refresh.tick() => {
                let age = last_message_at_ms.and_then(|last| message_age_ms(last).ok());
                publish_api_snapshot(
                    shared,
                    state,
                    metadata,
                    fee_profile,
                    ServerHealthStats {
                        connection_state: ConnectionState::Connected,
                        subscriptions: summary.subscriptions,
                        last_message_age_ms: age,
                        reconnects: summary.reconnects,
                        data_gaps: summary.data_gaps,
                        last_reconnect_backoff_ms: summary.last_reconnect_backoff_ms,
                        rows_written: summary.market_events,
                    },
                    summary,
                )?;
                dirty = false;
            }
            _ = publish.tick(), if dirty => {
                let age = last_message_at_ms.and_then(|last| message_age_ms(last).ok());
                publish_api_snapshot(
                    shared,
                    state,
                    metadata,
                    fee_profile,
                    ServerHealthStats {
                        connection_state: ConnectionState::Connected,
                        subscriptions: summary.subscriptions,
                        last_message_age_ms: age,
                        reconnects: summary.reconnects,
                        data_gaps: summary.data_gaps,
                        last_reconnect_backoff_ms: summary.last_reconnect_backoff_ms,
                        rows_written: summary.market_events,
                    },
                    summary,
                )?;
                dirty = false;
            }
            next = read.next() => {
                let Some(next) = next else {
                    bail!("Hyperliquid WebSocket stream ended");
                };
                let message = next.context("read live API WebSocket message")?;
                match message {
                    Message::Text(text) => {
                        dirty |= ingest_server_live_text(&text, state, summary)?;
                        last_message_at_ms = Some(now_ms_i64()?);
                    }
                    Message::Binary(bytes) => {
                        let text = String::from_utf8(bytes.to_vec())
                            .map_err(|err| HlsError::Parse(format!("binary WebSocket message was not UTF-8: {err}")))?;
                        dirty |= ingest_server_live_text(&text, state, summary)?;
                        last_message_at_ms = Some(now_ms_i64()?);
                    }
                    Message::Ping(payload) => {
                        if !send_rate_limited(
                            &mut write,
                            Message::Pong(payload),
                            outbound_rate_limiter,
                            deadline,
                            "send live API pong",
                        ).await? {
                            publish_final_api_snapshot(
                                shared,
                                state,
                                metadata,
                                fee_profile,
                                summary,
                                last_message_at_ms,
                                ConnectionState::Connected,
                            )?;
                            return Ok(());
                        }
                    }
                    Message::Close(frame) => bail!("Hyperliquid WebSocket closed: {frame:?}"),
                    Message::Pong(_) | Message::Frame(_) => {}
                }
            }
        }
    }
}

async fn send_rate_limited<S>(
    sink: &mut S,
    message: Message,
    limiter: &mut RollingMessageRateLimiter,
    deadline: tokio::time::Instant,
    context: &'static str,
) -> anyhow::Result<bool>
where
    S: Sink<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin,
{
    if !wait_for_rate_limit_slot(limiter, deadline).await {
        return Ok(false);
    }
    // Reserve before the write so an ambiguous failed or partial send still
    // consumes conservative outbound-message budget.
    limiter.record(tokio::time::Instant::now());
    tokio::select! {
        result = sink.send(message) => result.context(context)?,
        _ = tokio::time::sleep_until(deadline) => return Ok(false),
    }
    Ok(true)
}

async fn wait_for_rate_limit_slot(
    limiter: &mut RollingRateLimiter,
    deadline: tokio::time::Instant,
) -> bool {
    let now = tokio::time::Instant::now();
    let Some(available_at) = limiter.next_available_at(now) else {
        return true;
    };
    tokio::select! {
        _ = tokio::time::sleep_until(available_at) => true,
        _ = tokio::time::sleep_until(deadline) => false,
    }
}

async fn sleep_before_deadline(duration: Duration, deadline: tokio::time::Instant) -> bool {
    tokio::select! {
        _ = tokio::time::sleep(duration) => true,
        _ = tokio::time::sleep_until(deadline) => false,
    }
}

fn publish_final_api_snapshot(
    shared: &SharedApiState,
    state: &LiveMarketState,
    metadata: &[hls_core::metadata::MetadataEnrichment],
    fee_profile: Option<&hls_core::fees::FeeProfile>,
    summary: &mut ServerLiveSummary,
    last_message_at_ms: Option<i64>,
    connection_state: ConnectionState,
) -> anyhow::Result<()> {
    let age = last_message_at_ms.and_then(|last| message_age_ms(last).ok());
    publish_api_snapshot(
        shared,
        state,
        metadata,
        fee_profile,
        ServerHealthStats {
            connection_state,
            subscriptions: summary.subscriptions,
            last_message_age_ms: age,
            reconnects: summary.reconnects,
            data_gaps: summary.data_gaps,
            last_reconnect_backoff_ms: summary.last_reconnect_backoff_ms,
            rows_written: summary.market_events,
        },
        summary,
    )
}

fn live_api_publish_interval() -> tokio::time::Interval {
    let mut publish = interval(Duration::from_millis(LIVE_API_PUBLISH_INTERVAL_MS));
    publish.set_missed_tick_behavior(MissedTickBehavior::Skip);
    publish
}

fn ingest_server_live_text(
    line: &str,
    state: &mut LiveMarketState,
    summary: &mut ServerLiveSummary,
) -> anyhow::Result<bool> {
    summary.ws_messages = summary.ws_messages.saturating_add(1);
    let events = parse_ws_message(line)?;
    let changed = !events.is_empty();
    summary.market_events = summary.market_events.saturating_add(events.len() as u64);
    for event in events {
        state.apply(event.with_recv_ts_ns(now_ns_u64()?))?;
    }
    Ok(changed)
}

fn publish_api_snapshot(
    shared: &SharedApiState,
    state: &LiveMarketState,
    metadata: &[hls_core::metadata::MetadataEnrichment],
    fee_profile: Option<&hls_core::fees::FeeProfile>,
    health: ServerHealthStats,
    summary: &mut ServerLiveSummary,
) -> anyhow::Result<()> {
    let mut snapshots = feature_engine(fee_profile).snapshots(state, now_ms_i64()?);
    attach_metadata(&mut snapshots, metadata.to_vec());
    summary.rows = snapshots.len();
    summary.api_publishes = summary.api_publishes.saturating_add(1);
    shared.replace(ApiState::new(health.snapshot(), snapshots))?;
    Ok(())
}

#[derive(Clone, Debug, Default)]
struct ServerLiveSummary {
    symbols: usize,
    subscriptions: usize,
    ws_messages: u64,
    market_events: u64,
    rows: usize,
    reconnects: u64,
    data_gaps: u64,
    api_publishes: u64,
    last_reconnect_backoff_ms: Option<u64>,
}

impl ServerLiveSummary {
    fn require_market_data(self) -> anyhow::Result<Self> {
        if self.market_events == 0 {
            return Err(HlsError::External(format!(
                "live server run ended without market-data events after {} reconnect(s)",
                self.reconnects
            ))
            .into());
        }
        Ok(self)
    }
}

#[derive(Clone, Copy, Debug)]
struct ServerHealthStats {
    connection_state: ConnectionState,
    subscriptions: usize,
    last_message_age_ms: Option<u64>,
    reconnects: u64,
    data_gaps: u64,
    last_reconnect_backoff_ms: Option<u64>,
    rows_written: u64,
}

impl ServerHealthStats {
    fn snapshot(self) -> hls_core::health::HealthSnapshot {
        HealthInputs {
            safety: ReadOnlySafety::read_only(),
            connection: ConnectionHealth {
                state: self.connection_state,
                connected_at_ms: None,
                last_message_at_ms: None,
                reconnect_count: self.reconnects,
                last_reconnect_backoff_ms: self.last_reconnect_backoff_ms,
                gap_count: self.data_gaps,
            },
            subscription_count: self.subscriptions as u64,
            last_message_age_ms: self.last_message_age_ms,
            lag_ms: self.last_message_age_ms,
            writer: WriterHealth {
                backlog: 0,
                warn_at: 100,
                rows_written: self.rows_written,
            },
            recording: RecordingHealth {
                enabled: false,
                clean_shutdown: None,
            },
            gap_count: self.data_gaps,
        }
        .snapshot()
    }
}

#[derive(Clone, Debug, Default)]
struct ServerSymbolSelection {
    symbols: Vec<String>,
    metadata: Vec<hls_core::metadata::MetadataEnrichment>,
}

async fn load_server_live_symbols(args: &ServerArgs) -> anyhow::Result<ServerSymbolSelection> {
    let explicit_symbols = parse_symbols(args.symbols.as_deref());
    let markets = HyperliquidRestClient::default()
        .spot_meta_and_asset_ctxs()
        .await?;
    if !explicit_symbols.is_empty() {
        return resolve_server_symbols(&markets, &explicit_symbols);
    }

    let top_n = if args.all_symbols {
        markets.len()
    } else {
        args.top
    };
    let selected = select_universe(&markets, top_n, &[], &[])?;
    Ok(ServerSymbolSelection {
        symbols: selected
            .iter()
            .map(|market| market.symbol.hl_coin.clone())
            .collect(),
        metadata: selected.into_iter().map(|market| market.metadata).collect(),
    })
}

fn resolve_server_symbols(
    markets: &[SpotMarketContext],
    selectors: &[String],
) -> anyhow::Result<ServerSymbolSelection> {
    let mut symbols = Vec::new();
    let mut metadata = Vec::new();

    for selector in selectors {
        let market = markets
            .iter()
            .find(|market| market.symbol.matches_selector(selector))
            .with_context(|| {
                format!(
                    "unknown Hyperliquid spot symbol '{selector}'; run `hls symbols --top 50` to inspect display names and feed IDs"
                )
            })?;
        if symbols
            .iter()
            .any(|symbol| symbol == &market.symbol.hl_coin)
        {
            continue;
        }
        symbols.push(market.symbol.hl_coin.clone());
        metadata.push(market.metadata.clone());
    }

    Ok(ServerSymbolSelection { symbols, metadata })
}

fn fixture_symbols(args: &ServerArgs, events: &[MarketEvent]) -> Vec<String> {
    let explicit = parse_symbols(args.symbols.as_deref());
    if !explicit.is_empty() {
        return explicit;
    }
    let mut symbols: Vec<String> = events
        .iter()
        .filter_map(MarketEvent::hl_coin)
        .map(ToOwned::to_owned)
        .collect();
    symbols.sort();
    symbols.dedup();
    symbols.truncate(args.top);
    symbols
}

fn connecting_health() -> HealthInputs {
    HealthInputs {
        safety: ReadOnlySafety::read_only(),
        connection: ConnectionHealth {
            state: ConnectionState::Connecting,
            connected_at_ms: None,
            last_message_at_ms: None,
            reconnect_count: 0,
            last_reconnect_backoff_ms: None,
            gap_count: 0,
        },
        subscription_count: 0,
        last_message_age_ms: None,
        lag_ms: None,
        writer: WriterHealth {
            backlog: 0,
            warn_at: 100,
            rows_written: 0,
        },
        recording: RecordingHealth {
            enabled: false,
            clean_shutdown: None,
        },
        gap_count: 0,
    }
}

fn message_age_ms(last_message_at_ms: i64) -> anyhow::Result<u64> {
    let now = now_ms_i64()?;
    Ok(now.saturating_sub(last_message_at_ms).max(0) as u64)
}

fn now_ms_i64() -> anyhow::Result<i64> {
    i64::try_from(now_millis()?)
        .map_err(|_| HlsError::Time("current time overflowed i64 milliseconds".to_owned()).into())
}

fn now_ns_u64() -> anyhow::Result<u64> {
    let millis = now_millis()?;
    let nanos = millis
        .checked_mul(1_000_000)
        .ok_or_else(|| HlsError::Time("current time overflowed u64 nanoseconds".to_owned()))?;
    u64::try_from(nanos)
        .map_err(|_| HlsError::Time("current time overflowed u64 nanoseconds".to_owned()).into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        pin::Pin,
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
        task::{Context, Poll},
    };
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio_tungstenite::accept_async;

    #[test]
    fn server_shutdown_signal_mapping_is_successful_and_fail_closed() {
        assert_eq!(
            signal_stop_reason("SIGTERM", Some(())).expect("delivered SIGTERM"),
            ServerStopReason::Signal("SIGTERM")
        );

        let delivery_error = signal_stop_reason("SIGINT", None)
            .expect_err("closed signal listener must not report a clean stop");
        assert!(delivery_error.to_string().contains("SIGINT"));

        let setup_error = shutdown_listener_setup::<()>(Err(std::io::Error::other("setup failed")))
            .expect_err("listener setup failure must propagate");
        assert!(
            setup_error
                .to_string()
                .contains("install server shutdown signal listener")
        );
    }

    #[tokio::test]
    async fn live_server_signal_cancels_a_pending_publisher() {
        struct PendingPublisher(Arc<AtomicBool>);

        impl Future for PendingPublisher {
            type Output = anyhow::Result<ServerLiveSummary>;

            fn poll(self: Pin<&mut Self>, _context: &mut Context<'_>) -> Poll<Self::Output> {
                Poll::Pending
            }
        }

        impl Drop for PendingPublisher {
            fn drop(&mut self) {
                self.0.store(true, Ordering::SeqCst);
            }
        }

        let dropped = Arc::new(AtomicBool::new(false));
        let signal: ShutdownSignal = Box::pin(async { Ok(ServerStopReason::Signal("SIGTERM")) });
        let mut http_server = tokio::spawn(std::future::pending::<hls_core::HlsResult<()>>());

        let completion =
            wait_for_live_completion(PendingPublisher(dropped.clone()), signal, &mut http_server)
                .await;

        assert!(matches!(
            completion,
            LiveServerCompletion::Signal(Ok(ServerStopReason::Signal("SIGTERM")))
        ));
        assert!(dropped.load(Ordering::SeqCst));
        http_server.abort();
    }

    #[tokio::test]
    async fn live_server_signal_precedes_a_simultaneously_ready_publisher() {
        for _ in 0..128 {
            let publisher = async { Ok(ServerLiveSummary::default()) };
            let signal: ShutdownSignal =
                Box::pin(async { Ok(ServerStopReason::Signal("SIGTERM")) });
            let mut http_server = tokio::spawn(std::future::pending::<hls_core::HlsResult<()>>());

            let completion = wait_for_live_completion(publisher, signal, &mut http_server).await;

            assert!(matches!(
                completion,
                LiveServerCompletion::Signal(Ok(ServerStopReason::Signal("SIGTERM")))
            ));
            http_server.abort();
        }
    }

    #[tokio::test]
    async fn live_server_http_termination_precedes_other_ready_outcomes() {
        let publisher = async { Ok(ServerLiveSummary::default()) };
        let signal: ShutdownSignal = Box::pin(async { Ok(ServerStopReason::Signal("SIGTERM")) });
        let mut http_server = tokio::spawn(async { Ok(()) });
        while !http_server.is_finished() {
            tokio::task::yield_now().await;
        }

        let completion = wait_for_live_completion(publisher, signal, &mut http_server).await;

        assert!(matches!(
            completion,
            LiveServerCompletion::HttpStopped(Ok(()))
        ));
    }

    #[tokio::test]
    async fn live_server_releases_listener_when_publication_fails() {
        let reservation = std::net::TcpListener::bind("127.0.0.1:0").expect("reserve port");
        let address = reservation.local_addr().expect("reserved address");
        drop(reservation);
        let args = ServerArgs {
            bind: address.to_string(),
            print_health: false,
            live: true,
            symbols: Some("@107".to_owned()),
            top: 1,
            all_symbols: false,
            duration_secs: 1,
            refresh_secs: 1,
            max_subscriptions: 10,
            ws_url: DEFAULT_WS_URL.to_owned(),
            fixture_file: Some(PathBuf::from("missing-live-server-fixture.ndjson")),
            metadata_file: None,
            fee_profile_file: None,
            simulate_health: None,
        };

        run_live_server(args, address)
            .await
            .expect_err("missing fixture must fail");

        TcpListener::bind(address)
            .await
            .expect("failed live server must release its listener");
    }

    #[test]
    fn live_server_summary_rejects_runs_without_market_data() {
        let error = ServerLiveSummary {
            symbols: 1,
            subscriptions: 3,
            ws_messages: 4,
            ..ServerLiveSummary::default()
        }
        .require_market_data()
        .expect_err("control frames cannot make a live server run successful");

        assert!(error.to_string().contains("without market-data events"));
    }

    #[test]
    fn all_symbol_server_plan_stays_within_subscription_budget() {
        let symbols = (0..1_000).map(|index| format!("@{index}")).collect();

        let plan = server_subscription_plan(symbols, true, OFFICIAL_WS_SUBSCRIPTION_LIMIT);

        assert!(plan.subscription_count() <= OFFICIAL_WS_SUBSCRIPTION_LIMIT);
        assert_eq!(plan.streams(), &[StreamKind::AllMids]);
    }

    #[tokio::test]
    async fn idle_then_burst_is_coalesced_and_published_before_deadline() {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("listener");
        let address = listener.local_addr().expect("listener address");
        let peer = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("accept");
            let mut websocket = accept_async(stream).await.expect("websocket accept");
            websocket
                .next()
                .await
                .expect("subscription")
                .expect("message");
            let trade = include_str!("../../../../tests/fixtures/hyperliquid/ws_mock_live.ndjson")
                .lines()
                .next()
                .expect("fixture trade")
                .to_owned();
            tokio::time::sleep(Duration::from_millis(750)).await;
            for _ in 0..50 {
                websocket
                    .send(Message::Text(trade.clone().into()))
                    .await
                    .expect("send fixture event");
            }
            tokio::time::sleep(Duration::from_millis(1_250)).await;
        });
        let args = ServerArgs {
            bind: "127.0.0.1:0".to_owned(),
            print_health: false,
            live: true,
            symbols: Some("@107".to_owned()),
            top: 1,
            all_symbols: false,
            duration_secs: 2,
            refresh_secs: 60,
            max_subscriptions: 10,
            ws_url: format!("ws://{address}"),
            fixture_file: None,
            metadata_file: None,
            fee_profile_file: None,
            simulate_health: None,
        };
        let shared = SharedApiState::new(ApiState::new(connecting_health().snapshot(), Vec::new()));
        let api_listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("API listener");
        let api_address = api_listener.local_addr().expect("API address");
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let api_server = tokio::spawn(serve_shared_until_shutdown(
            api_listener,
            shared.clone(),
            async move {
                let _ = shutdown_rx.await;
            },
        ));
        let probe = tokio::spawn(async move {
            let probe_deadline = tokio::time::Instant::now() + Duration::from_millis(1_400);
            loop {
                let mut stream = tokio::net::TcpStream::connect(api_address)
                    .await
                    .expect("connect API");
                stream
                    .write_all(
                        b"GET /symbols HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
                    )
                    .await
                    .expect("write request");
                let mut response = Vec::new();
                stream
                    .read_to_end(&mut response)
                    .await
                    .expect("read response");
                let response = String::from_utf8(response).expect("UTF-8 response");
                if response.contains("@107") {
                    return response;
                }
                assert!(
                    tokio::time::Instant::now() < probe_deadline,
                    "HTTP API did not expose the ingested row before the run deadline: {response}"
                );
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        });
        let mut state = LiveMarketState::new(["@107".to_owned()]);
        let mut summary = ServerLiveSummary {
            symbols: 1,
            subscriptions: 1,
            ..ServerLiveSummary::default()
        };
        let mut outbound_rate_limiter = RollingMessageRateLimiter::default();
        let mut connection_rate_limiter =
            RollingRateLimiter::new(SERVER_CONNECTION_RATE_BUDGET, WS_OUTBOUND_RATE_WINDOW);

        drive_server_live_connection(
            &args,
            &[SubscriptionPlan::new(vec!["@107".to_owned()])
                .with_streams([StreamKind::Trades])
                .subscribe_messages()
                .expect("subscription messages")[0]
                .clone()],
            &shared,
            &mut state,
            &[],
            None,
            tokio::time::Instant::now() + Duration::from_secs(2),
            &mut summary,
            &mut outbound_rate_limiter,
            &mut connection_rate_limiter,
        )
        .await
        .expect("bounded live connection");

        assert!(summary.market_events > 0);
        assert_eq!(summary.rows, 1, "deadline must publish the final state");
        assert!(
            summary.api_publishes <= 4,
            "traffic after an idle period must be coalesced instead of consuming catch-up ticks: {} publishes",
            summary.api_publishes
        );
        assert!(probe.await.expect("probe task").contains("@107"));
        let _ = shutdown_tx.send(());
        api_server
            .await
            .expect("API server task")
            .expect("API server result");
        peer.await.expect("peer task");
    }

    #[tokio::test]
    async fn live_api_publish_interval_skips_missed_ticks() {
        assert_eq!(
            live_api_publish_interval().missed_tick_behavior(),
            MissedTickBehavior::Skip
        );
    }

    #[tokio::test]
    async fn outbound_messages_wait_for_the_rolling_rate_window() {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("listener");
        let address = listener.local_addr().expect("listener address");
        let peer = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("accept");
            let mut websocket = accept_async(stream).await.expect("websocket accept");
            websocket
                .next()
                .await
                .expect("first message")
                .expect("first");
            let first = tokio::time::Instant::now();
            websocket
                .next()
                .await
                .expect("second message")
                .expect("second");
            tokio::time::Instant::now().saturating_duration_since(first)
        });
        let (websocket, _) = connect_async(format!("ws://{address}"))
            .await
            .expect("connect");
        let (mut write, _) = websocket.split();
        let window = Duration::from_millis(100);
        let mut limiter = RollingMessageRateLimiter::new(1, window);
        let deadline = tokio::time::Instant::now() + Duration::from_secs(1);

        assert!(
            send_rate_limited(
                &mut write,
                Message::Text("first".into()),
                &mut limiter,
                deadline,
                "send first",
            )
            .await
            .expect("first send")
        );
        assert!(
            send_rate_limited(
                &mut write,
                Message::Text("second".into()),
                &mut limiter,
                deadline,
                "send second",
            )
            .await
            .expect("second send")
        );

        assert!(peer.await.expect("peer task") >= window);
    }

    #[test]
    fn server_reconnect_backoff_caps_at_thirty_seconds() {
        let policy = ReconnectPolicy {
            initial_backoff_ms: SERVER_RECONNECT_INITIAL_BACKOFF_MS,
            max_backoff_ms: SERVER_RECONNECT_MAX_BACKOFF_MS,
            multiplier: 2,
        };
        let observed = (0..7)
            .map(|attempt| policy.backoff_ms(attempt))
            .collect::<Vec<_>>();

        assert_eq!(
            observed,
            vec![1_000, 2_000, 4_000, 8_000, 16_000, 30_000, 30_000]
        );
        assert!(
            observed.iter().take(6).sum::<u64>() >= 60_000,
            "six reconnect delays must span at least one minute"
        );
    }

    #[test]
    fn server_reconnect_backoff_restarts_after_market_data_recovery() {
        let policy = ReconnectPolicy {
            initial_backoff_ms: SERVER_RECONNECT_INITIAL_BACKOFF_MS,
            max_backoff_ms: SERVER_RECONNECT_MAX_BACKOFF_MS,
            multiplier: 2,
        };
        let mut attempt = 0;

        assert_eq!(
            next_server_reconnect_backoff(policy, &mut attempt, false),
            1_000
        );
        assert_eq!(
            next_server_reconnect_backoff(policy, &mut attempt, false),
            2_000
        );
        assert_eq!(
            next_server_reconnect_backoff(policy, &mut attempt, true),
            1_000
        );
        assert_eq!(
            next_server_reconnect_backoff(policy, &mut attempt, false),
            2_000
        );
    }

    #[test]
    fn server_connection_attempts_keep_headroom_below_the_official_rate_limit() {
        let started = tokio::time::Instant::now();
        let mut limiter =
            RollingRateLimiter::new(SERVER_CONNECTION_RATE_BUDGET, WS_OUTBOUND_RATE_WINDOW);

        for _ in 0..SERVER_CONNECTION_RATE_BUDGET {
            assert_eq!(limiter.next_available_at(started), None);
            limiter.record(started);
        }
        assert_eq!(
            limiter.next_available_at(started + Duration::from_secs(1)),
            Some(started + WS_OUTBOUND_RATE_WINDOW)
        );
        assert_eq!(
            limiter.next_available_at(started + WS_OUTBOUND_RATE_WINDOW),
            None
        );
    }

    #[test]
    fn recovered_connection_preserves_gap_evidence_without_staying_reconnecting() {
        let snapshot = ServerHealthStats {
            connection_state: ConnectionState::Connected,
            subscriptions: 10,
            last_message_age_ms: Some(5),
            reconnects: 1,
            data_gaps: 1,
            last_reconnect_backoff_ms: Some(1_000),
            rows_written: 100,
        }
        .snapshot();

        assert_eq!(snapshot.connections[0].state, ConnectionState::Connected);
        assert_eq!(snapshot.connections[0].gap_count, 1);
        assert_eq!(snapshot.gap_count, 1);
        assert_eq!(snapshot.reconnect_count, 1);
    }
}
