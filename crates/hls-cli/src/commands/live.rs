use std::{
    fs,
    io::{self, IsTerminal, Write},
    path::PathBuf,
    sync::mpsc::{self, Receiver, SyncSender, TrySendError},
    thread::{self, JoinHandle},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, bail};
use clap::Args;
use futures_util::{SinkExt, StreamExt};
use hls_core::{
    HlsError, HlsResult,
    data_gap::DataGap,
    market_state::{LiveMarketState, MarketEvent},
    time::now_millis,
};
use hls_features::engine::FeatureEngine;
use hls_hyperliquid::{
    rest::{HyperliquidRestClient, select_universe},
    ws::{
        parser::{parse_ws_message, parse_ws_ndjson},
        subscriptions::{StreamKind, SubscriptionPlan, ping_message},
    },
};
use hls_screen::ScreenRequest;
use hls_store::{
    metadata::{MetadataRegistry, RecordingRun, SymbolRegistryEntry},
    normalized::StreamingNormalizedWriter,
    raw::{RawMarketMessage, RawWriter},
    recorder::{RecordOptions, RecordSummary, record_fixture_ndjson},
};
use hls_tui::app::render_screened_table;
use tokio::time::{interval, sleep_until, timeout_at};
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::commands::record::{default_run_id, enabled_outputs, parse_symbols};

const DEFAULT_WS_URL: &str = "wss://api.hyperliquid.xyz/ws";
const DEFAULT_LIVE_DURATION_SECS: u64 = 60;
const DEFAULT_REFRESH_SECS: u64 = 30;
const DEFAULT_MAX_SUBSCRIPTIONS: usize = 980;
const LIVE_RECORDER_QUEUE_CAPACITY: usize = 65_536;
const INITIAL_RECONNECT_BACKOFF_MS: u64 = 1_000;
const MAX_RECONNECT_BACKOFF_MS: u64 = 30_000;

#[derive(Debug, Args)]
pub struct LiveArgs {
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

    #[arg(long)]
    pub tui: bool,

    #[arg(long, default_value_t = DEFAULT_MAX_SUBSCRIPTIONS)]
    pub max_subscriptions: usize,

    #[arg(long, default_value = DEFAULT_WS_URL)]
    pub ws_url: String,

    #[arg(long)]
    pub preset: Option<String>,

    #[arg(long)]
    pub r#where: Option<String>,

    #[arg(long)]
    pub sort: Option<String>,

    #[arg(long)]
    pub record: bool,

    #[arg(long)]
    pub raw: bool,

    #[arg(long)]
    pub parquet: bool,

    #[arg(long)]
    pub normalized: bool,

    #[arg(long)]
    pub run_id: Option<String>,

    #[arg(long, default_value = ".hls")]
    pub data_dir: PathBuf,

    #[arg(long, hide = true)]
    pub fixture_file: Option<PathBuf>,

    #[arg(long, hide = true)]
    pub once: bool,
}

pub async fn run(args: LiveArgs) -> anyhow::Result<()> {
    if let Some(fixture_file) = args.fixture_file.clone() {
        return run_fixture_live(args, &fixture_file).await;
    }

    run_network_live(args).await
}

async fn run_fixture_live(args: LiveArgs, fixture_file: &PathBuf) -> anyhow::Result<()> {
    if !args.once {
        bail!("fixture-backed live mode currently requires --once");
    }

    let raw = fs::read_to_string(fixture_file)
        .with_context(|| format!("read {}", fixture_file.display()))?;

    if args.record {
        if args.parquet {
            bail!(
                "Parquet output is not implemented in this slice; use --normalized for replayable JSONL"
            );
        }
        let run_id = args.run_id.clone().unwrap_or_else(default_run_id);
        let (raw_enabled, normalized_enabled) = enabled_outputs(args.raw, args.normalized);
        let summary = record_fixture_ndjson(
            &raw,
            RecordOptions::new(
                &args.data_dir,
                &run_id,
                parse_symbols(args.symbols.as_deref()),
                raw_enabled,
                normalized_enabled,
            ),
        )?;
        println!("recording run: {}", summary.run_id);
        println!("clean_shutdown={}", summary.clean_shutdown);
    }

    let events = parse_ws_ndjson(&raw)?;
    let symbols = selected_symbols(&args, &events);
    let mut state = LiveMarketState::new(symbols);

    for event in events {
        state.apply(event)?;
    }

    let snapshots = FeatureEngine::default().snapshots(&state, latest_update_ms(&state));
    print!(
        "{}",
        render_screened_table(
            &snapshots,
            "READ-ONLY Hyperliquid spot live screen",
            &ScreenRequest {
                preset: args.preset,
                where_expr: args.r#where,
                sort: args.sort,
            }
        )?
    );

    Ok(())
}

async fn run_network_live(args: LiveArgs) -> anyhow::Result<()> {
    if args.once {
        bail!("--once is only supported with --fixture-file");
    }
    if args.duration_secs == 0 {
        bail!("--duration-secs must be greater than zero");
    }
    if args.parquet {
        bail!(
            "Parquet output is not implemented in this slice; use --normalized for replayable JSONL"
        );
    }

    let symbols = load_live_symbols(&args).await?;
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
    let mut state = LiveMarketState::new(symbols.clone());
    let run_id = args.run_id.clone().unwrap_or_else(default_run_id);
    let (raw_enabled, normalized_enabled) = enabled_outputs(args.raw, args.normalized);
    let recorder = if args.record {
        Some(LiveRecorder::new(
            &args.data_dir,
            &run_id,
            symbols.clone(),
            raw_enabled,
            normalized_enabled,
        )?)
    } else {
        None
    };
    let screen_request = ScreenRequest {
        preset: args.preset.clone(),
        where_expr: args.r#where.clone(),
        sort: args.sort.clone(),
    };
    let render_live_tui = args.tui || io::stderr().is_terminal();

    eprintln!(
        "read-only live run: symbols={} subscriptions={} streams_per_symbol={} duration_secs={} ws_url={}",
        symbols.len(),
        subscription_messages.len(),
        plan.streams().len(),
        args.duration_secs,
        args.ws_url
    );

    let drive_result = drive_live_ws(
        &args.ws_url,
        &subscription_messages,
        &symbols,
        Duration::from_secs(args.duration_secs),
        Duration::from_secs(args.refresh_secs.max(1)),
        &mut state,
        &screen_request,
        render_live_tui,
        recorder.as_ref(),
    )
    .await;

    let record_summary = if let Some(recorder) = recorder {
        match recorder.finish(drive_result.is_ok()) {
            Ok(summary) => Some(summary),
            Err(err) if drive_result.is_err() => {
                eprintln!("recording closeout failed after live error: {err}");
                None
            }
            Err(err) => return Err(err.into()),
        }
    } else {
        None
    };

    let mut summary = drive_result?;
    let snapshots = FeatureEngine::default().snapshots(&state, now_ms_i64()?);
    summary.row_count = snapshots.len();

    println!("live_run=complete");
    println!("symbols={}", symbols.len());
    println!("subscriptions={}", subscription_messages.len());
    println!("streams_per_symbol={}", plan.streams().len());
    println!("ws_messages={}", summary.ws_messages);
    println!("market_events={}", summary.market_events);
    println!("reconnects={}", summary.reconnects);
    println!("data_gaps={}", summary.data_gaps);
    println!("elapsed_secs={}", summary.elapsed_secs);
    if let Some(record_summary) = &record_summary {
        println!("recording run: {}", record_summary.run_id);
        println!("raw_messages={}", record_summary.raw_messages);
        println!("normalized_events={}", record_summary.normalized_events);
        println!("raw_files={}", record_summary.raw_files.len());
        println!("normalized_files={}", record_summary.normalized_files.len());
        println!("clean_shutdown={}", record_summary.clean_shutdown);
    }
    print!(
        "{}",
        render_screened_table(
            &snapshots,
            "READ-ONLY Hyperliquid spot live screen",
            &screen_request
        )?
    );

    Ok(())
}

async fn load_live_symbols(args: &LiveArgs) -> anyhow::Result<Vec<String>> {
    let explicit_symbols = parse_symbols(args.symbols.as_deref());
    if !explicit_symbols.is_empty() {
        return Ok(explicit_symbols);
    }

    let markets = HyperliquidRestClient::default()
        .spot_meta_and_asset_ctxs()
        .await?;
    let top_n = if args.all_symbols {
        markets.len()
    } else {
        args.top
    };
    let selected = select_universe(&markets, top_n, &[], &[])?;

    Ok(selected
        .into_iter()
        .map(|market| market.symbol.hl_coin)
        .collect())
}

#[derive(Clone, Debug, Default)]
struct LiveDriveSummary {
    ws_messages: u64,
    market_events: u64,
    reconnects: u64,
    data_gaps: u64,
    elapsed_secs: u64,
    row_count: usize,
}

#[derive(Debug)]
enum ConnectionOutcome {
    DurationElapsed,
    Reconnect {
        conn_id: u64,
        gap_started_at_ns: u64,
        gap_ended_at_ns: u64,
        reason: String,
        received_any_message: bool,
    },
}

#[derive(Debug)]
enum WsReadEvent {
    Text(String),
    Control,
    Reconnect(String),
}

#[allow(clippy::too_many_arguments)]
async fn drive_live_ws(
    ws_url: &str,
    subscription_messages: &[String],
    symbols: &[String],
    duration: Duration,
    refresh_interval: Duration,
    state: &mut LiveMarketState,
    screen_request: &ScreenRequest,
    render_live_tui: bool,
    recorder: Option<&LiveRecorder>,
) -> anyhow::Result<LiveDriveSummary> {
    let started = Instant::now();
    let deadline = tokio::time::Instant::now() + duration;
    let mut summary = LiveDriveSummary::default();
    let mut conn_id = 0;
    let mut reconnect_attempt = 0;

    while tokio::time::Instant::now() < deadline {
        let outcome = drive_live_connection(
            ws_url,
            subscription_messages,
            conn_id,
            deadline,
            started,
            refresh_interval,
            state,
            screen_request,
            render_live_tui,
            recorder,
            &mut summary,
        )
        .await?;

        match outcome {
            ConnectionOutcome::DurationElapsed => break,
            ConnectionOutcome::Reconnect {
                conn_id: closed_conn_id,
                gap_started_at_ns,
                gap_ended_at_ns,
                reason,
                received_any_message,
            } => {
                summary.reconnects = summary.reconnects.saturating_add(1);
                summary.data_gaps = summary.data_gaps.saturating_add(1);
                if let Some(recorder) = recorder {
                    recorder.record_gap(
                        closed_conn_id,
                        gap_started_at_ns,
                        gap_ended_at_ns,
                        &reason,
                        symbols,
                    )?;
                }

                let backoff = reconnect_backoff(reconnect_attempt);
                eprintln!(
                    "live reconnect: conn_id={} reason={} backoff_ms={} reconnects={} data_gaps={}",
                    closed_conn_id,
                    reason,
                    backoff.as_millis(),
                    summary.reconnects,
                    summary.data_gaps
                );
                conn_id = conn_id.saturating_add(1);
                reconnect_attempt = if received_any_message {
                    0
                } else {
                    reconnect_attempt.saturating_add(1)
                };
                sleep_for_backoff_until_deadline(backoff, deadline).await;
            }
        }
    }

    if summary.ws_messages == 0 && summary.reconnects > 0 {
        return Err(HlsError::External(format!(
            "live run ended without receiving any WebSocket messages after {} reconnect attempt(s)",
            summary.reconnects
        ))
        .into());
    }

    summary.elapsed_secs = started.elapsed().as_secs();
    Ok(summary)
}

#[allow(clippy::too_many_arguments)]
async fn drive_live_connection(
    ws_url: &str,
    subscription_messages: &[String],
    conn_id: u64,
    deadline: tokio::time::Instant,
    started: Instant,
    refresh_interval: Duration,
    state: &mut LiveMarketState,
    screen_request: &ScreenRequest,
    render_live_tui: bool,
    recorder: Option<&LiveRecorder>,
    summary: &mut LiveDriveSummary,
) -> anyhow::Result<ConnectionOutcome> {
    let connect_started_ns = now_ns_u64()?;
    let (ws, _) = match timeout_at(deadline, connect_async(ws_url)).await {
        Ok(Ok(value)) => value,
        Ok(Err(err)) => {
            return Ok(ConnectionOutcome::Reconnect {
                conn_id,
                gap_started_at_ns: connect_started_ns,
                gap_ended_at_ns: now_ns_u64()?,
                reason: format!("connect Hyperliquid WebSocket: {err}"),
                received_any_message: false,
            });
        }
        Err(_) => {
            return Ok(ConnectionOutcome::Reconnect {
                conn_id,
                gap_started_at_ns: connect_started_ns,
                gap_ended_at_ns: now_ns_u64()?,
                reason: "connect Hyperliquid WebSocket timed out before run deadline".to_owned(),
                received_any_message: false,
            });
        }
    };
    let (mut write, mut read) = ws.split();

    for message in subscription_messages {
        if let Err(err) = write.send(Message::Text(message.clone().into())).await {
            return Ok(ConnectionOutcome::Reconnect {
                conn_id,
                gap_started_at_ns: connect_started_ns,
                gap_ended_at_ns: now_ns_u64()?,
                reason: format!("send subscription: {err}"),
                received_any_message: false,
            });
        }
    }

    let mut heartbeat = interval(Duration::from_secs(20));
    heartbeat.tick().await;
    let mut progress = interval(refresh_interval);
    progress.tick().await;
    let mut last_message_recv_ns: Option<u64> = None;
    let mut received_any_message = false;

    loop {
        tokio::select! {
            _ = sleep_until(deadline) => {
                let _ = write.send(Message::Close(None)).await;
                return Ok(ConnectionOutcome::DurationElapsed);
            }
            _ = progress.tick() => {
                render_live_progress(
                    state,
                    screen_request,
                    render_live_tui,
                    started,
                    summary,
                )?;
            }
            _ = heartbeat.tick() => {
                if let Err(err) = write.send(Message::Text(ping_message().to_owned().into())).await {
                    return Ok(ConnectionOutcome::Reconnect {
                        conn_id,
                        gap_started_at_ns: last_message_recv_ns.unwrap_or(connect_started_ns),
                        gap_ended_at_ns: now_ns_u64()?,
                        reason: format!("send heartbeat ping: {err}"),
                        received_any_message,
                    });
                }
            }
            next = read.next() => {
                let recv_ts_ns = now_ns_u64()?;
                let Some(next) = next else {
                    return Ok(ConnectionOutcome::Reconnect {
                        conn_id,
                        gap_started_at_ns: last_message_recv_ns.unwrap_or(connect_started_ns),
                        gap_ended_at_ns: recv_ts_ns,
                        reason: "Hyperliquid WebSocket stream ended".to_owned(),
                        received_any_message,
                    });
                };
                let message = match next {
                    Ok(message) => message,
                    Err(err) => {
                        return Ok(ConnectionOutcome::Reconnect {
                            conn_id,
                            gap_started_at_ns: last_message_recv_ns.unwrap_or(connect_started_ns),
                            gap_ended_at_ns: recv_ts_ns,
                            reason: format!("read WebSocket message: {err}"),
                            received_any_message,
                        });
                    }
                };
                received_any_message = true;
                last_message_recv_ns = Some(recv_ts_ns);
                match ws_message_text(message, &mut write).await? {
                    WsReadEvent::Text(line) => {
                    summary.ws_messages += 1;
                    if let Some(recorder) = recorder {
                        recorder.record_raw_line(recv_ts_ns, conn_id, line.clone())?;
                    }
                    let events: Vec<_> = parse_ws_message(&line)?
                        .into_iter()
                        .map(|event| event.with_recv_ts_ns(recv_ts_ns))
                        .collect();
                    summary.market_events += events.len() as u64;
                    if let Some(recorder) = recorder {
                        recorder.record_events(events.clone())?;
                    }
                    for event in events {
                        state.apply(event)?;
                    }
                    }
                    WsReadEvent::Control => {}
                    WsReadEvent::Reconnect(reason) => {
                        return Ok(ConnectionOutcome::Reconnect {
                            conn_id,
                            gap_started_at_ns: last_message_recv_ns.unwrap_or(connect_started_ns),
                            gap_ended_at_ns: recv_ts_ns,
                            reason,
                            received_any_message,
                        });
                    }
                }
            }
        }
    }
}

async fn ws_message_text<S>(message: Message, write: &mut S) -> HlsResult<WsReadEvent>
where
    S: futures_util::Sink<Message> + Unpin,
    <S as futures_util::Sink<Message>>::Error: std::fmt::Display,
{
    match message {
        Message::Text(text) => Ok(WsReadEvent::Text(text.to_string())),
        Message::Binary(bytes) => String::from_utf8(bytes.to_vec())
            .map(WsReadEvent::Text)
            .map_err(|err| {
                HlsError::Parse(format!("binary WebSocket message was not UTF-8: {err}"))
            }),
        Message::Ping(payload) => {
            write
                .send(Message::Pong(payload))
                .await
                .map_err(|err| HlsError::External(format!("send WebSocket pong: {err}")))?;
            Ok(WsReadEvent::Control)
        }
        Message::Pong(_) | Message::Frame(_) => Ok(WsReadEvent::Control),
        Message::Close(frame) => Ok(WsReadEvent::Reconnect(format!(
            "Hyperliquid WebSocket closed: {frame:?}"
        ))),
    }
}

enum LiveRecordCommand {
    Raw {
        recv_ts_ns: u64,
        conn_id: u64,
        line: String,
    },
    Events(Vec<MarketEvent>),
    Gap(DataGap),
    Finish {
        clean_shutdown: bool,
    },
}

struct LiveRecorder {
    run_id: String,
    sender: SyncSender<LiveRecordCommand>,
    handle: JoinHandle<HlsResult<RecordSummary>>,
}

impl LiveRecorder {
    fn new(
        data_dir: &PathBuf,
        run_id: &str,
        symbols: Vec<String>,
        raw_enabled: bool,
        normalized_enabled: bool,
    ) -> HlsResult<Self> {
        let worker =
            LiveRecorderWorker::new(data_dir, run_id, symbols, raw_enabled, normalized_enabled)?;
        let (sender, receiver) = mpsc::sync_channel(LIVE_RECORDER_QUEUE_CAPACITY);
        let handle = thread::Builder::new()
            .name("hls-live-recorder".to_owned())
            .spawn(move || worker.run(receiver))?;

        Ok(Self {
            run_id: run_id.to_owned(),
            sender,
            handle,
        })
    }

    fn record_raw_line(&self, recv_ts_ns: u64, conn_id: u64, line: String) -> HlsResult<()> {
        self.send(LiveRecordCommand::Raw {
            recv_ts_ns,
            conn_id,
            line,
        })
    }

    fn record_events(&self, events: Vec<MarketEvent>) -> HlsResult<()> {
        if events.is_empty() {
            return Ok(());
        }
        self.send(LiveRecordCommand::Events(events))
    }

    fn record_gap(
        &self,
        conn_id: u64,
        started_at_ns: u64,
        ended_at_ns: u64,
        reason: &str,
        symbols: &[String],
    ) -> HlsResult<()> {
        self.send(LiveRecordCommand::Gap(DataGap::new(
            self.run_id.clone(),
            conn_id,
            started_at_ns,
            ended_at_ns,
            reason.to_owned(),
            symbols.to_vec(),
            true,
        )))
    }

    fn finish(self, clean_shutdown: bool) -> HlsResult<RecordSummary> {
        let _ = self
            .sender
            .send(LiveRecordCommand::Finish { clean_shutdown });
        drop(self.sender);
        self.handle
            .join()
            .map_err(|_| HlsError::External("live recorder worker panicked".to_owned()))?
    }

    fn send(&self, command: LiveRecordCommand) -> HlsResult<()> {
        match self.sender.try_send(command) {
            Ok(()) => Ok(()),
            Err(TrySendError::Full(_)) => Err(HlsError::External(format!(
                "live recorder queue is full at capacity {LIVE_RECORDER_QUEUE_CAPACITY}; failing closed to avoid silent data loss"
            ))),
            Err(TrySendError::Disconnected(_)) => Err(HlsError::External(
                "live recorder worker disconnected".to_owned(),
            )),
        }
    }
}

struct LiveRecorderWorker {
    registry: MetadataRegistry,
    run_id: String,
    raw_writer: Option<RawWriter>,
    normalized_writer: Option<StreamingNormalizedWriter>,
    seq: u64,
    raw_messages: u64,
    normalized_events: u64,
}

impl LiveRecorderWorker {
    fn new(
        data_dir: &PathBuf,
        run_id: &str,
        symbols: Vec<String>,
        raw_enabled: bool,
        normalized_enabled: bool,
    ) -> HlsResult<Self> {
        if !raw_enabled && !normalized_enabled {
            return Err(HlsError::Config(
                "recording requires --raw, --normalized, or both".to_owned(),
            ));
        }

        let registry = MetadataRegistry::open(data_dir.join("hls.sqlite"))?;
        let started_at_ms = now_ms_i64()?;
        registry.insert_run(&RecordingRun::new(
            run_id,
            started_at_ms,
            raw_enabled,
            normalized_enabled,
        ))?;
        for symbol in &symbols {
            registry.insert_symbol(&SymbolRegistryEntry::new(
                symbol,
                started_at_ms,
                started_at_ms,
            ))?;
        }

        Ok(Self {
            registry,
            run_id: run_id.to_owned(),
            raw_writer: raw_enabled
                .then(|| RawWriter::new(data_dir, run_id, 8 * 1024 * 1024))
                .transpose()?,
            normalized_writer: normalized_enabled
                .then(|| StreamingNormalizedWriter::new(data_dir, run_id))
                .transpose()?,
            seq: 0,
            raw_messages: 0,
            normalized_events: 0,
        })
    }

    fn run(mut self, receiver: Receiver<LiveRecordCommand>) -> HlsResult<RecordSummary> {
        let mut clean_shutdown = false;
        for command in receiver {
            match command {
                LiveRecordCommand::Raw {
                    recv_ts_ns,
                    conn_id,
                    line,
                } => self.record_raw_line(recv_ts_ns, conn_id, &line)?,
                LiveRecordCommand::Events(events) => self.record_events(&events)?,
                LiveRecordCommand::Gap(gap) => self.registry.insert_gap(&gap)?,
                LiveRecordCommand::Finish {
                    clean_shutdown: requested_clean_shutdown,
                } => {
                    clean_shutdown = requested_clean_shutdown;
                    break;
                }
            }
        }

        self.finish(clean_shutdown)
    }

    fn record_raw_line(&mut self, recv_ts_ns: u64, conn_id: u64, line: &str) -> HlsResult<()> {
        let Some(raw_writer) = &mut self.raw_writer else {
            return Ok(());
        };
        self.seq = self.seq.saturating_add(1);
        let message = RawMarketMessage::from_ws_line(recv_ts_ns, conn_id, self.seq, line)?;
        raw_writer.write(&message)?;
        self.raw_messages += 1;
        Ok(())
    }

    fn record_events(&mut self, events: &[MarketEvent]) -> HlsResult<()> {
        let Some(normalized_writer) = &mut self.normalized_writer else {
            return Ok(());
        };
        for event in events {
            normalized_writer.write_event(event)?;
            self.normalized_events += 1;
        }
        Ok(())
    }

    fn finish(mut self, clean_shutdown: bool) -> HlsResult<RecordSummary> {
        let mut raw_files = Vec::new();
        if let Some(raw_writer) = self.raw_writer.take() {
            raw_files = raw_writer.finish()?;
            for file in &raw_files {
                self.registry.insert_file(file)?;
            }
        }

        let mut normalized_files = Vec::new();
        if let Some(normalized_writer) = self.normalized_writer.take()
            && let Some(file) = normalized_writer.finish()?
        {
            self.registry.insert_file(&file)?;
            normalized_files.push(file);
        }

        self.registry
            .finish_run(&self.run_id, now_ms_i64()?, clean_shutdown)?;
        Ok(RecordSummary {
            run_id: self.run_id,
            raw_files,
            normalized_files,
            raw_messages: self.raw_messages,
            normalized_events: self.normalized_events,
            clean_shutdown,
        })
    }
}

fn render_live_progress(
    state: &LiveMarketState,
    screen_request: &ScreenRequest,
    render_live_tui: bool,
    started: Instant,
    summary: &LiveDriveSummary,
) -> anyhow::Result<()> {
    if render_live_tui {
        let snapshots = FeatureEngine::default().snapshots(state, now_ms_i64()?);
        let table = render_screened_table(
            &snapshots,
            "READ-ONLY Hyperliquid spot live screen",
            screen_request,
        )?;
        let mut stderr = io::stderr().lock();
        write!(stderr, "\x1b[2J\x1b[H{table}")?;
        writeln!(
            stderr,
            "live progress: elapsed_secs={} ws_messages={} market_events={} reconnects={} data_gaps={}",
            started.elapsed().as_secs(),
            summary.ws_messages,
            summary.market_events,
            summary.reconnects,
            summary.data_gaps
        )?;
        stderr.flush()?;
    } else {
        eprintln!(
            "live progress: elapsed_secs={} ws_messages={} market_events={} reconnects={} data_gaps={}",
            started.elapsed().as_secs(),
            summary.ws_messages,
            summary.market_events,
            summary.reconnects,
            summary.data_gaps
        );
    }

    Ok(())
}

async fn sleep_for_backoff_until_deadline(backoff: Duration, deadline: tokio::time::Instant) {
    let now = tokio::time::Instant::now();
    if now >= deadline {
        return;
    }

    let wake_at = (now + backoff).min(deadline);
    sleep_until(wake_at).await;
}

fn reconnect_backoff(attempt: u64) -> Duration {
    let shift = u32::try_from(attempt.min(16)).unwrap_or(16);
    let multiplier = 1_u64.checked_shl(shift).unwrap_or(u64::MAX);
    Duration::from_millis(
        INITIAL_RECONNECT_BACKOFF_MS
            .saturating_mul(multiplier)
            .min(MAX_RECONNECT_BACKOFF_MS),
    )
}

fn now_ns_u64() -> HlsResult<u64> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| HlsError::Time(format!("system clock is before UNIX epoch: {err}")))?
        .as_nanos();
    u64::try_from(nanos)
        .map_err(|_| HlsError::Time("current time overflowed u64 nanoseconds".to_owned()))
}

fn selected_symbols(
    args: &LiveArgs,
    events: &[hls_core::market_state::MarketEvent],
) -> Vec<String> {
    if let Some(symbols) = &args.symbols {
        return symbols
            .split(',')
            .map(str::trim)
            .filter(|symbol| !symbol.is_empty())
            .map(ToOwned::to_owned)
            .collect();
    }

    let mut symbols: Vec<String> = events
        .iter()
        .filter_map(hls_core::market_state::MarketEvent::hl_coin)
        .map(ToOwned::to_owned)
        .collect();
    symbols.sort();
    symbols.dedup();
    symbols.truncate(args.top);
    symbols
}

fn latest_update_ms(state: &LiveMarketState) -> i64 {
    state
        .states()
        .filter_map(|symbol_state| symbol_state.last_update_ms)
        .max()
        .unwrap_or_default()
}

fn now_ms_i64() -> HlsResult<i64> {
    i64::try_from(now_millis()?)
        .map_err(|_| HlsError::Time("current time overflowed i64 milliseconds".to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use hls_store::{
        metadata::MetadataRegistry, normalized::read_normalized_events, raw::read_raw_file,
    };

    #[test]
    fn reconnect_backoff_is_bounded_for_live_runtime() {
        assert_eq!(reconnect_backoff(0), Duration::from_millis(1_000));
        assert_eq!(reconnect_backoff(1), Duration::from_millis(2_000));
        assert_eq!(reconnect_backoff(5), Duration::from_millis(30_000));
        assert_eq!(reconnect_backoff(100), Duration::from_millis(30_000));
    }

    #[test]
    fn live_recorder_worker_preserves_receive_timestamps_and_gaps() {
        let temp = tempfile::tempdir().expect("tempdir");
        let data_dir = temp.path().to_path_buf();
        let line = r#"{"channel":"trades","data":[{"coin":"@107","side":"B","px":"35.00","sz":"2.0","time":1710000000000,"hash":"0xabc","tid":11}]}"#;
        let recv_ts_ns = 1_710_000_000_123_456_789;
        let recorder = LiveRecorder::new(
            &data_dir,
            "live-worker-test",
            vec!["@107".to_owned()],
            true,
            true,
        )
        .expect("live recorder starts");

        recorder
            .record_raw_line(recv_ts_ns, 3, line.to_owned())
            .expect("raw enqueue succeeds");
        let events: Vec<_> = parse_ws_message(line)
            .expect("line parses")
            .into_iter()
            .map(|event| event.with_recv_ts_ns(recv_ts_ns))
            .collect();
        recorder
            .record_events(events)
            .expect("normalized enqueue succeeds");
        recorder
            .record_gap(
                3,
                recv_ts_ns,
                recv_ts_ns + 1_000_000,
                "test reconnect",
                &["@107".to_owned()],
            )
            .expect("gap enqueue succeeds");

        let summary = recorder.finish(true).expect("clean recorder finish");
        assert_eq!(summary.raw_messages, 1);
        assert_eq!(summary.normalized_events, 1);
        assert!(summary.clean_shutdown);

        let raw = read_raw_file(data_dir.join(&summary.raw_files[0].path)).expect("raw reads");
        assert_eq!(raw[0].recv_ts_ns, recv_ts_ns);
        assert_eq!(raw[0].conn_id, 3);

        let normalized = read_normalized_events(data_dir.join(&summary.normalized_files[0].path))
            .expect("normalized reads");
        assert_eq!(normalized[0].recv_ts_ns(), recv_ts_ns);

        let registry = MetadataRegistry::open(data_dir.join("hls.sqlite")).expect("registry opens");
        let run = registry
            .get_run("live-worker-test")
            .expect("get run")
            .expect("run exists");
        assert_eq!(run.gap_count, 1);
        let gaps = registry.list_gaps("live-worker-test").expect("gaps list");
        assert_eq!(gaps[0].reason, "test reconnect");
        assert_eq!(gaps[0].affected_symbols, vec!["@107".to_owned()]);
    }
}
