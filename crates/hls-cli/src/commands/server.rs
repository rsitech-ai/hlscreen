use std::{fs, net::SocketAddr, path::PathBuf, time::Duration};

use anyhow::{Context, bail};
use clap::Args;
use futures_util::{SinkExt, StreamExt};
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
        parser::{parse_ws_message, parse_ws_ndjson},
        subscriptions::{StreamKind, SubscriptionPlan, ping_message},
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
};

const DEFAULT_WS_URL: &str = "wss://api.hyperliquid.xyz/ws";
const DEFAULT_LIVE_DURATION_SECS: u64 = 60;
const DEFAULT_REFRESH_SECS: u64 = 5;
const DEFAULT_MAX_SUBSCRIPTIONS: usize = 980;
const LIVE_API_PUBLISH_INTERVAL_MS: u64 = 250;

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

    #[arg(long)]
    pub symbols: Option<String>,

    #[arg(long, default_value_t = 50)]
    pub top: usize,

    #[arg(long)]
    pub all_symbols: bool,

    #[arg(long, default_value_t = DEFAULT_LIVE_DURATION_SECS)]
    pub duration_secs: u64,

    #[arg(long, default_value_t = DEFAULT_REFRESH_SECS)]
    pub refresh_secs: u64,

    #[arg(long, default_value_t = DEFAULT_MAX_SUBSCRIPTIONS)]
    pub max_subscriptions: usize,

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

    let listener = TcpListener::bind(bind).await?;
    let address = listener.local_addr()?;
    eprintln!("hls server listening on http://{address} (read-only, Ctrl-C to stop)");
    serve_shared_until_shutdown(
        listener,
        SharedApiState::new(ApiState::new(health, Vec::new())),
        async {
            if let Err(error) = tokio::signal::ctrl_c().await {
                eprintln!("failed to install Ctrl-C handler: {error}");
            }
        },
    )
    .await?;
    eprintln!("hls server stopped cleanly");
    Ok(())
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
    if args.duration_secs == 0 {
        bail!("--duration-secs must be greater than zero");
    }
    let fee_profile = load_fee_profile(args.fee_profile_file.as_ref())?;
    let listener = TcpListener::bind(bind).await?;
    let address = listener.local_addr()?;
    let shared = SharedApiState::new(ApiState::new(connecting_health().snapshot(), Vec::new()));
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let server = tokio::spawn(serve_shared_until_shutdown(
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

    let summary = if let Some(fixture_file) = &args.fixture_file {
        publish_fixture_live_api(&args, fixture_file, &shared, fee_profile.as_ref())?
    } else {
        publish_network_live_api(&args, &shared, fee_profile.as_ref()).await?
    };

    let _ = shutdown_tx.send(());
    server
        .await
        .map_err(|err| HlsError::External(format!("live API server task failed: {err}")))??;

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
            subscriptions: summary.subscriptions,
            last_message_age_ms: Some(0),
            reconnects: 0,
            data_gaps: 0,
            rows_written: summary.market_events,
        },
        &mut summary,
    )?;
    Ok(summary)
}

async fn publish_network_live_api(
    args: &ServerArgs,
    shared: &SharedApiState,
    fee_profile: Option<&hls_core::fees::FeeProfile>,
) -> anyhow::Result<ServerLiveSummary> {
    let selection = load_server_live_symbols(args).await?;
    let symbols = selection.symbols;
    let mut plan =
        SubscriptionPlan::new(symbols.clone()).with_max_subscriptions(args.max_subscriptions);
    if args.all_symbols && plan.subscription_count() > args.max_subscriptions {
        plan = plan.with_streams([
            StreamKind::Trades,
            StreamKind::Bbo,
            StreamKind::ActiveAssetCtx,
        ]);
    }
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

    while tokio::time::Instant::now() < deadline {
        let connected_at_ms = now_ms_i64()?;
        match drive_server_live_connection(
            args,
            &subscription_messages,
            shared,
            &mut state,
            &metadata,
            fee_profile,
            deadline,
            connected_at_ms,
            &mut summary,
        )
        .await
        {
            Ok(()) => break,
            Err(error) => {
                summary.reconnects = summary.reconnects.saturating_add(1);
                summary.data_gaps = summary.data_gaps.saturating_add(1);
                eprintln!(
                    "live API reconnect: reason={} reconnects={} data_gaps={}",
                    error, summary.reconnects, summary.data_gaps
                );
                publish_api_snapshot(
                    shared,
                    &state,
                    &metadata,
                    fee_profile,
                    ServerHealthStats {
                        subscriptions: summary.subscriptions,
                        last_message_age_ms: None,
                        reconnects: summary.reconnects,
                        data_gaps: summary.data_gaps,
                        rows_written: summary.market_events,
                    },
                    &mut summary,
                )?;
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }

    Ok(summary)
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
    connected_at_ms: i64,
    summary: &mut ServerLiveSummary,
) -> anyhow::Result<()> {
    let (ws, _) = connect_async(&args.ws_url)
        .await
        .with_context(|| format!("connect {}", args.ws_url))?;
    let (mut write, mut read) = ws.split();
    for message in subscription_messages {
        write
            .send(Message::Text(message.clone().into()))
            .await
            .context("send live API subscription")?;
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
                        subscriptions: summary.subscriptions,
                        last_message_age_ms: age,
                        reconnects: summary.reconnects,
                        data_gaps: summary.data_gaps,
                        rows_written: summary.market_events,
                    },
                    summary,
                )?;
                return Ok(());
            },
            _ = heartbeat.tick() => {
                write
                    .send(Message::Text(ping_message().to_owned().into()))
                    .await
                    .context("send live API heartbeat")?;
            }
            _ = refresh.tick() => {
                let age = last_message_at_ms.and_then(|last| message_age_ms(last).ok());
                publish_api_snapshot(
                    shared,
                    state,
                    metadata,
                    fee_profile,
                    ServerHealthStats {
                        subscriptions: summary.subscriptions,
                        last_message_age_ms: age,
                        reconnects: summary.reconnects,
                        data_gaps: summary.data_gaps,
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
                        subscriptions: summary.subscriptions,
                        last_message_age_ms: age,
                        reconnects: summary.reconnects,
                        data_gaps: summary.data_gaps,
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
                        write.send(Message::Pong(payload)).await.context("send live API pong")?;
                    }
                    Message::Close(frame) => bail!("Hyperliquid WebSocket closed: {frame:?}"),
                    Message::Pong(_) | Message::Frame(_) => {}
                }
            }
        }
    }
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
}

#[derive(Clone, Copy, Debug)]
struct ServerHealthStats {
    subscriptions: usize,
    last_message_age_ms: Option<u64>,
    reconnects: u64,
    data_gaps: u64,
    rows_written: u64,
}

impl ServerHealthStats {
    fn snapshot(self) -> hls_core::health::HealthSnapshot {
        HealthInputs {
            safety: ReadOnlySafety::read_only(),
            connection: ConnectionHealth {
                state: if self.data_gaps > 0 {
                    ConnectionState::Reconnecting
                } else {
                    ConnectionState::Connected
                },
                connected_at_ms: None,
                last_message_at_ms: None,
                reconnect_count: self.reconnects,
                last_reconnect_backoff_ms: None,
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
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio_tungstenite::accept_async;

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
            now_ms_i64().expect("connected time"),
            &mut summary,
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
}
