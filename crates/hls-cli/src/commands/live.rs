use std::{
    fs,
    io::{self, IsTerminal, Write},
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, SyncSender, TrySendError},
    thread::{self, JoinHandle},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, bail};
use clap::{Args, ValueEnum};
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind,
        KeyModifiers, MouseEvent, MouseEventKind,
    },
    execute,
    terminal::{
        EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
        size as terminal_size,
    },
};
use futures_util::{SinkExt, StreamExt};
use hls_core::{
    HlsError, HlsResult,
    data_gap::DataGap,
    market_state::{CandleEvent, FeatureSnapshot, LiveMarketState, MarketEvent, TradeEvent},
    metadata::MetadataEnrichment,
    time::now_millis,
};
use hls_features::engine::FeatureEngine;
use hls_hyperliquid::{
    rest::{HyperliquidRestClient, SpotMarketContext, select_universe},
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
use hls_tui::{
    app::{render_confidence_summary, render_screened_table},
    interaction::{
        WorkstationAction, WorkstationChartWindow, WorkstationCommandTarget, WorkstationDensity,
        WorkstationPane, WorkstationUiPreferences, WorkstationUiState, WorkstationView,
    },
    ratatui_app::{
        RatatuiColorMode, RatatuiFrameModel, RatatuiViewport, render_ratatui_snapshot_for_test,
    },
};
use ratatui::{Terminal, backend::CrosstermBackend};
use serde::{Deserialize, Serialize};
use tokio::time::{interval, sleep_until, timeout_at};
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::commands::metadata::{attach_metadata, load_metadata_enrichments};
use crate::commands::record::{default_run_id, enabled_outputs, parse_symbols};

const DEFAULT_WS_URL: &str = "wss://api.hyperliquid.xyz/ws";
const DEFAULT_LIVE_DURATION_SECS: u64 = 60;
const DEFAULT_REFRESH_SECS: u64 = 30;
const DEFAULT_MAX_SUBSCRIPTIONS: usize = 980;
const LIVE_RECORDER_QUEUE_CAPACITY: usize = 65_536;
const INITIAL_RECONNECT_BACKOFF_MS: u64 = 1_000;
const MAX_RECONNECT_BACKOFF_MS: u64 = 30_000;
const TUI_KEY_POLL_MS: u64 = 100;
const TUI_PREFERENCES_FILE: &str = "tui-preferences.toml";

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

    /// TUI color policy: always forces ANSI color, auto follows terminal/env detection, never disables it.
    #[arg(long, value_enum, default_value_t = LiveTuiColor::Always)]
    pub color: LiveTuiColor,

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
    pub metadata_file: Option<PathBuf>,

    #[arg(long, hide = true)]
    pub once: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum LiveTuiColor {
    Auto,
    Always,
    Never,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct PersistedTuiPreferences {
    view: String,
    density: String,
    chart_window: String,
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

    let mut snapshots = FeatureEngine::default().snapshots(&state, latest_update_ms(&state));
    attach_metadata(
        &mut snapshots,
        load_metadata_enrichments(args.metadata_file.as_ref())?,
    );
    let screen_request = ScreenRequest {
        preset: args.preset,
        where_expr: args.r#where,
        sort: args.sort,
    };
    let color_mode = live_ratatui_color_mode(args.color);
    if args.tui {
        let model = live_tui_model(
            &snapshots,
            live_table_title(args.record),
            &screen_request,
            None,
            live_tui_candles(&state),
            live_tui_trades(&state),
            LiveTuiStatus::new("fixture", "REC ready", "fixture replay"),
        );
        let table = render_live_tui_snapshot(&model, None, color_mode)?;
        print!("{table}");
    } else {
        println!("{}", render_confidence_summary(&snapshots));
        let table =
            render_screened_table(&snapshots, live_table_title(args.record), &screen_request)?;
        print!("{table}");
    }

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

    let selection = load_live_symbols(&args).await?;
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
    let mut screen_request = ScreenRequest {
        preset: args.preset.clone(),
        where_expr: args.r#where.clone(),
        sort: args.sort.clone(),
    };
    let mut metadata = selection.metadata;
    metadata.extend(load_metadata_enrichments(args.metadata_file.as_ref())?);
    let render_live_tui = args.tui || io::stderr().is_terminal();
    let color_mode = live_ratatui_color_mode(args.color);
    let keyboard_interactive =
        render_live_tui && io::stdin().is_terminal() && io::stderr().is_terminal();
    let _terminal_mode = LiveTuiGuard::enable(keyboard_interactive)?;
    let mut tui_state = render_live_tui
        .then(|| WorkstationUiState::from_preferences(load_tui_preferences(&args.data_dir)));

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
        &mut screen_request,
        &metadata,
        render_live_tui,
        color_mode,
        keyboard_interactive,
        tui_state.as_mut(),
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

    let tui_preference_save = if let Some(ui_state) = tui_state.as_ref() {
        save_tui_preferences(&args.data_dir, ui_state.preferences())
    } else {
        Ok(())
    };

    let mut summary = drive_result?;
    if let Err(err) = tui_preference_save {
        eprintln!("tui preferences save skipped: {err}");
    }
    let mut snapshots = FeatureEngine::default().snapshots(&state, now_ms_i64()?);
    attach_metadata(&mut snapshots, metadata);
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
    if !render_live_tui {
        println!("{}", render_confidence_summary(&snapshots));
    }
    if let Some(record_summary) = &record_summary {
        println!("recording run: {}", record_summary.run_id);
        println!("raw_messages={}", record_summary.raw_messages);
        println!("normalized_events={}", record_summary.normalized_events);
        println!("raw_files={}", record_summary.raw_files.len());
        println!("normalized_files={}", record_summary.normalized_files.len());
        println!("clean_shutdown={}", record_summary.clean_shutdown);
    }
    let table = if render_live_tui {
        let model = live_tui_model(
            &snapshots,
            live_table_title(record_summary.is_some()),
            &screen_request,
            tui_state.as_ref(),
            live_tui_candles(&state),
            live_tui_trades(&state),
            LiveTuiStatus::new(
                "complete",
                if record_summary.is_some() {
                    "REC done"
                } else {
                    "REC ready"
                },
                format!(
                    "ws={} events={} reconnects={} gaps={}",
                    summary.ws_messages,
                    summary.market_events,
                    summary.reconnects,
                    summary.data_gaps
                ),
            ),
        );
        render_live_tui_snapshot(&model, None, color_mode)?
    } else {
        render_screened_table(
            &snapshots,
            live_table_title(record_summary.is_some()),
            &screen_request,
        )?
    };
    print!("{table}");

    Ok(())
}

#[derive(Clone, Debug, Default)]
struct LiveSymbolSelection {
    symbols: Vec<String>,
    metadata: Vec<MetadataEnrichment>,
}

async fn load_live_symbols(args: &LiveArgs) -> anyhow::Result<LiveSymbolSelection> {
    let explicit_symbols = parse_symbols(args.symbols.as_deref());
    let markets = HyperliquidRestClient::default()
        .spot_meta_and_asset_ctxs()
        .await?;
    if !explicit_symbols.is_empty() {
        return resolve_explicit_live_symbols(&markets, &explicit_symbols);
    }

    let top_n = if args.all_symbols {
        markets.len()
    } else {
        args.top
    };
    let selected = select_universe(&markets, top_n, &[], &[])?;

    Ok(LiveSymbolSelection {
        symbols: selected
            .iter()
            .map(|market| market.symbol.hl_coin.clone())
            .collect(),
        metadata: selected.into_iter().map(|market| market.metadata).collect(),
    })
}

fn resolve_explicit_live_symbols(
    markets: &[SpotMarketContext],
    selectors: &[String],
) -> anyhow::Result<LiveSymbolSelection> {
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

    Ok(LiveSymbolSelection { symbols, metadata })
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

struct LiveProgressContext<'a> {
    state: &'a LiveMarketState,
    screen_request: &'a ScreenRequest,
    metadata: &'a [MetadataEnrichment],
    render_live_tui: bool,
    color_mode: RatatuiColorMode,
    tui_state: Option<&'a WorkstationUiState>,
    started: Instant,
    summary: &'a LiveDriveSummary,
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
    screen_request: &mut ScreenRequest,
    metadata: &[MetadataEnrichment],
    render_live_tui: bool,
    color_mode: RatatuiColorMode,
    keyboard_interactive: bool,
    mut tui_state: Option<&mut WorkstationUiState>,
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
            metadata,
            render_live_tui,
            color_mode,
            keyboard_interactive,
            tui_state.as_deref_mut(),
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
    screen_request: &mut ScreenRequest,
    metadata: &[MetadataEnrichment],
    render_live_tui: bool,
    color_mode: RatatuiColorMode,
    keyboard_interactive: bool,
    mut tui_state: Option<&mut WorkstationUiState>,
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
    let mut ui_events = interval(Duration::from_millis(TUI_KEY_POLL_MS));
    ui_events.tick().await;
    let mut last_message_recv_ns: Option<u64> = None;
    let mut received_any_message = false;

    loop {
        tokio::select! {
            _ = sleep_until(deadline) => {
                let _ = write.send(Message::Close(None)).await;
                return Ok(ConnectionOutcome::DurationElapsed);
            }
            _ = progress.tick() => {
                render_live_progress(LiveProgressContext {
                    state,
                    screen_request,
                    metadata,
                    render_live_tui,
                    color_mode,
                    tui_state: tui_state.as_deref(),
                    started,
                    summary,
                })?;
            }
            _ = ui_events.tick(), if keyboard_interactive => {
                if let Some(ui_state) = tui_state.as_deref_mut()
                    && apply_pending_tui_actions(ui_state, state, screen_request)?
                {
                    render_live_progress(LiveProgressContext {
                        state,
                        screen_request,
                        metadata,
                        render_live_tui,
                        color_mode,
                        tui_state: Some(ui_state),
                        started,
                        summary,
                    })?;
                    if ui_state.quit_requested() {
                        let _ = write.send(Message::Close(None)).await;
                        return Ok(ConnectionOutcome::DurationElapsed);
                    }
                }
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

fn render_live_progress(ctx: LiveProgressContext<'_>) -> anyhow::Result<()> {
    if ctx.render_live_tui {
        let mut snapshots = FeatureEngine::default().snapshots(ctx.state, now_ms_i64()?);
        attach_metadata(&mut snapshots, ctx.metadata.to_vec());
        let model = live_tui_model(
            &snapshots,
            "READ-ONLY Hyperliquid spot live screen",
            ctx.screen_request,
            ctx.tui_state,
            live_tui_candles(ctx.state),
            live_tui_trades(ctx.state),
            LiveTuiStatus::new(
                "LIVE",
                "REC ready",
                format!(
                    "{}s ws={} events={} reconnects={} gaps={}",
                    ctx.started.elapsed().as_secs(),
                    ctx.summary.ws_messages,
                    ctx.summary.market_events,
                    ctx.summary.reconnects,
                    ctx.summary.data_gaps
                ),
            ),
        );
        draw_live_tui_frame(&model, ctx.color_mode)?;
    } else {
        eprintln!(
            "live progress: elapsed_secs={} ws_messages={} market_events={} reconnects={} data_gaps={}",
            ctx.started.elapsed().as_secs(),
            ctx.summary.ws_messages,
            ctx.summary.market_events,
            ctx.summary.reconnects,
            ctx.summary.data_gaps
        );
    }

    Ok(())
}

fn render_live_tui_snapshot(
    model: &RatatuiFrameModel,
    viewport: Option<RatatuiViewport>,
    color_mode: RatatuiColorMode,
) -> anyhow::Result<String> {
    render_ratatui_snapshot_for_test(
        model,
        viewport.unwrap_or_else(live_ratatui_viewport),
        color_mode,
    )
    .map_err(Into::into)
}

fn live_tui_model(
    snapshots: &[hls_core::market_state::FeatureSnapshot],
    title: &str,
    screen_request: &ScreenRequest,
    tui_state: Option<&WorkstationUiState>,
    candles: Vec<CandleEvent>,
    trades: Vec<TradeEvent>,
    status: LiveTuiStatus,
) -> RatatuiFrameModel {
    RatatuiFrameModel::new(
        snapshots.to_vec(),
        title,
        screen_request.clone(),
        tui_state.cloned().unwrap_or_default(),
    )
    .with_candles(candles)
    .with_trades(trades)
    .with_status(status.stream, status.recorder, status.health)
}

struct LiveTuiStatus {
    stream: String,
    recorder: String,
    health: String,
}

impl LiveTuiStatus {
    fn new(
        stream: impl Into<String>,
        recorder: impl Into<String>,
        health: impl Into<String>,
    ) -> Self {
        Self {
            stream: stream.into(),
            recorder: recorder.into(),
            health: health.into(),
        }
    }
}

fn live_tui_candles(state: &LiveMarketState) -> Vec<CandleEvent> {
    state
        .states()
        .flat_map(|state| state.candles.iter().cloned())
        .collect()
}

fn live_tui_trades(state: &LiveMarketState) -> Vec<TradeEvent> {
    state
        .states()
        .flat_map(|state| state.trades.iter().cloned())
        .collect()
}

fn draw_live_tui_frame(
    model: &RatatuiFrameModel,
    color_mode: RatatuiColorMode,
) -> anyhow::Result<()> {
    let stderr = io::stderr();
    let backend = CrosstermBackend::new(stderr);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;
    terminal.draw(|frame| {
        hls_tui::ratatui_app::render_ratatui_frame(frame, model, color_mode);
    })?;
    terminal.backend_mut().flush()?;
    Ok(())
}

fn live_ratatui_viewport() -> RatatuiViewport {
    live_ratatui_viewport_from_size(terminal_size().ok())
}

fn live_ratatui_viewport_from_size(size: Option<(u16, u16)>) -> RatatuiViewport {
    let (width, height) = size
        .filter(|(width, height)| *width > 0 && *height > 0)
        .unwrap_or((160, 48));
    RatatuiViewport { width, height }
}

fn live_ratatui_color_mode(color: LiveTuiColor) -> RatatuiColorMode {
    resolve_live_ratatui_color_mode(color, live_terminal_color_enabled())
}

fn resolve_live_ratatui_color_mode(color: LiveTuiColor, auto_enabled: bool) -> RatatuiColorMode {
    match color {
        LiveTuiColor::Auto if auto_enabled => RatatuiColorMode::Color,
        LiveTuiColor::Auto | LiveTuiColor::Never => RatatuiColorMode::NoColor,
        LiveTuiColor::Always => RatatuiColorMode::Color,
    }
}

fn tui_preferences_path(data_dir: &Path) -> PathBuf {
    data_dir.join(TUI_PREFERENCES_FILE)
}

fn load_tui_preferences(data_dir: &Path) -> WorkstationUiPreferences {
    match try_load_tui_preferences(data_dir) {
        Ok(preferences) => preferences,
        Err(err) => {
            eprintln!("tui preferences load skipped: {err}");
            WorkstationUiPreferences::default()
        }
    }
}

fn try_load_tui_preferences(data_dir: &Path) -> anyhow::Result<WorkstationUiPreferences> {
    let path = tui_preferences_path(data_dir);
    if !path.exists() {
        return Ok(WorkstationUiPreferences::default());
    }

    let raw = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let persisted: PersistedTuiPreferences =
        toml::from_str(&raw).with_context(|| format!("parse {}", path.display()))?;
    persisted
        .to_preferences()
        .with_context(|| format!("decode {}", path.display()))
}

fn save_tui_preferences(
    data_dir: &Path,
    preferences: WorkstationUiPreferences,
) -> anyhow::Result<()> {
    fs::create_dir_all(data_dir).with_context(|| format!("create {}", data_dir.display()))?;
    let path = tui_preferences_path(data_dir);
    let raw = toml::to_string_pretty(&PersistedTuiPreferences::from_preferences(preferences))
        .context("encode TUI preferences")?;
    fs::write(&path, raw).with_context(|| format!("write {}", path.display()))
}

impl PersistedTuiPreferences {
    fn from_preferences(preferences: WorkstationUiPreferences) -> Self {
        Self {
            view: preferences.view.label().to_owned(),
            density: preferences.density.label().to_owned(),
            chart_window: preferences.chart_window.label().to_owned(),
        }
    }

    fn to_preferences(&self) -> anyhow::Result<WorkstationUiPreferences> {
        Ok(WorkstationUiPreferences {
            view: parse_workstation_view(&self.view)
                .with_context(|| format!("unknown TUI view {:?}", self.view))?,
            density: parse_workstation_density(&self.density)
                .with_context(|| format!("unknown TUI density {:?}", self.density))?,
            chart_window: parse_workstation_chart_window(&self.chart_window)
                .with_context(|| format!("unknown TUI chart window {:?}", self.chart_window))?,
        })
    }
}

fn parse_workstation_view(label: &str) -> Option<WorkstationView> {
    WorkstationView::ALL
        .into_iter()
        .find(|candidate| candidate.label() == label)
}

fn parse_workstation_density(label: &str) -> Option<WorkstationDensity> {
    WorkstationDensity::ALL
        .into_iter()
        .find(|candidate| candidate.label() == label)
}

fn parse_workstation_chart_window(label: &str) -> Option<WorkstationChartWindow> {
    WorkstationChartWindow::ALL
        .into_iter()
        .find(|candidate| candidate.label() == label)
}

fn live_terminal_color_enabled() -> bool {
    if live_terminal_color_forced() {
        true
    } else {
        live_terminal_color_auto_enabled()
    }
}

fn live_terminal_color_forced() -> bool {
    if env_flag_enabled("HLS_FORCE_COLOR")
        || env_flag_enabled("CLICOLOR_FORCE")
        || env_flag_enabled("FORCE_COLOR")
    {
        return true;
    }
    false
}

fn live_terminal_color_auto_enabled() -> bool {
    if std::env::var_os("NO_COLOR").is_some() {
        return false;
    }
    !matches!(std::env::var("TERM").as_deref(), Ok("dumb"))
}

fn env_flag_enabled(name: &str) -> bool {
    std::env::var(name)
        .map(|value| env_flag_value_enabled(Some(value.as_str())))
        .unwrap_or(false)
}

fn env_flag_value_enabled(value: Option<&str>) -> bool {
    value
        .map(|value| {
            let value = value.trim();
            !value.is_empty() && value != "0" && !value.eq_ignore_ascii_case("false")
        })
        .unwrap_or(false)
}

fn apply_pending_tui_actions(
    ui_state: &mut WorkstationUiState,
    state: &LiveMarketState,
    screen_request: &mut ScreenRequest,
) -> anyhow::Result<bool> {
    let mut actions = Vec::new();
    let mut redraw_requested = false;
    let row_count = current_screened_row_count(state, screen_request)?;
    while event::poll(Duration::from_millis(0))? {
        match live_tui_event_effect(event::read()?, ui_state, terminal_size().ok(), row_count) {
            LiveTuiEventEffect::Ignore => {}
            LiveTuiEventEffect::Redraw => redraw_requested = true,
            LiveTuiEventEffect::Action(action) => actions.push(action),
        }
    }

    if actions.is_empty() {
        return Ok(redraw_requested);
    }

    for action in actions {
        apply_live_tui_action(action, ui_state, state, screen_request)?;
    }
    Ok(true)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LiveTuiEventEffect {
    Ignore,
    Redraw,
    Action(WorkstationAction),
}

fn live_tui_event_effect(
    event: Event,
    ui_state: &WorkstationUiState,
    terminal_size: Option<(u16, u16)>,
    row_count: usize,
) -> LiveTuiEventEffect {
    match event {
        Event::Key(key) => {
            if key.kind != KeyEventKind::Press {
                return LiveTuiEventEffect::Ignore;
            }
            key_to_workstation_action(key, ui_state)
                .map_or(LiveTuiEventEffect::Ignore, LiveTuiEventEffect::Action)
        }
        Event::Mouse(mouse) => {
            mouse_to_workstation_action(mouse, ui_state, terminal_size, row_count)
                .map_or(LiveTuiEventEffect::Ignore, LiveTuiEventEffect::Action)
        }
        Event::Resize(_, _) => LiveTuiEventEffect::Redraw,
        _ => LiveTuiEventEffect::Ignore,
    }
}

fn key_to_workstation_action(
    key: KeyEvent,
    ui_state: &WorkstationUiState,
) -> Option<WorkstationAction> {
    if ui_state.command().is_some() {
        return match key.code {
            KeyCode::Enter => Some(WorkstationAction::SubmitCommand),
            KeyCode::Esc => Some(WorkstationAction::CancelCommand),
            KeyCode::Backspace => Some(WorkstationAction::CommandBackspace),
            KeyCode::Char(ch)
                if key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT =>
            {
                Some(WorkstationAction::CommandChar(ch))
            }
            _ => None,
        };
    }

    match key.code {
        KeyCode::Up | KeyCode::Char('k') => Some(WorkstationAction::Up),
        KeyCode::Down | KeyCode::Char('j') => Some(WorkstationAction::Down),
        KeyCode::PageUp => Some(WorkstationAction::PageUp),
        KeyCode::PageDown => Some(WorkstationAction::PageDown),
        KeyCode::Home => Some(WorkstationAction::Home),
        KeyCode::End => Some(WorkstationAction::End),
        KeyCode::Enter => Some(WorkstationAction::FocusPane(WorkstationPane::Detail)),
        KeyCode::Tab => Some(WorkstationAction::NextView),
        KeyCode::BackTab => Some(WorkstationAction::PreviousView),
        KeyCode::Char(']') => Some(WorkstationAction::NextPane),
        KeyCode::Char('[') => Some(WorkstationAction::PreviousPane),
        KeyCode::Char('1') => Some(WorkstationAction::FocusPane(WorkstationPane::Watchlist)),
        KeyCode::Char('2') => Some(WorkstationAction::FocusPane(WorkstationPane::Detail)),
        KeyCode::Char('3') => Some(WorkstationAction::FocusPane(WorkstationPane::Chart)),
        KeyCode::Char('4') => Some(WorkstationAction::FocusPane(WorkstationPane::Book)),
        KeyCode::Char('5') => Some(WorkstationAction::FocusPane(WorkstationPane::Tape)),
        KeyCode::Char('6') => Some(WorkstationAction::FocusPane(WorkstationPane::Status)),
        KeyCode::Char('w') | KeyCode::Char('W') => {
            Some(WorkstationAction::FocusPane(WorkstationPane::Watchlist))
        }
        KeyCode::Char('i') | KeyCode::Char('I') => {
            Some(WorkstationAction::FocusPane(WorkstationPane::Detail))
        }
        KeyCode::Char('c') | KeyCode::Char('C') => {
            Some(WorkstationAction::FocusPane(WorkstationPane::Chart))
        }
        KeyCode::Char('b') | KeyCode::Char('B') => {
            Some(WorkstationAction::FocusPane(WorkstationPane::Book))
        }
        KeyCode::Char('r') | KeyCode::Char('R') => {
            Some(WorkstationAction::FocusPane(WorkstationPane::Tape))
        }
        KeyCode::Char('o') | KeyCode::Char('O') => {
            Some(WorkstationAction::FocusPane(WorkstationPane::Status))
        }
        KeyCode::Char('/') => Some(WorkstationAction::CycleFilter),
        KeyCode::Char('g') | KeyCode::Char('G') => Some(WorkstationAction::OpenSymbolSearch),
        KeyCode::Char('p') | KeyCode::Char('P') => Some(WorkstationAction::CyclePreset),
        KeyCode::Char('s') | KeyCode::Char('S') => Some(WorkstationAction::CycleSort),
        KeyCode::Char('t') | KeyCode::Char('T') => Some(WorkstationAction::CycleChartWindow),
        KeyCode::Char('z') | KeyCode::Char('Z') => Some(WorkstationAction::TogglePaneZoom),
        KeyCode::Char('h') | KeyCode::Char('H') => {
            Some(WorkstationAction::FocusPane(WorkstationPane::Status))
        }
        KeyCode::Char('d') | KeyCode::Char('D') => Some(WorkstationAction::ToggleDensity),
        KeyCode::Char('?') | KeyCode::F(1) => Some(WorkstationAction::ToggleHelp),
        KeyCode::Char(' ') => Some(WorkstationAction::TogglePause),
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => Some(WorkstationAction::Quit),
        _ => None,
    }
}

fn mouse_to_workstation_action(
    mouse: MouseEvent,
    ui_state: &WorkstationUiState,
    terminal_size: Option<(u16, u16)>,
    row_count: usize,
) -> Option<WorkstationAction> {
    if ui_state.command().is_some() {
        return None;
    }

    match mouse.kind {
        MouseEventKind::ScrollUp => Some(WorkstationAction::Up),
        MouseEventKind::ScrollDown => Some(WorkstationAction::Down),
        MouseEventKind::Down(_) => terminal_size.map(|(width, height)| {
            if let Some(pane) =
                mouse_header_pane_for_position(mouse.column, mouse.row, width, ui_state)
            {
                WorkstationAction::FocusPane(pane)
            } else if let Some(action) =
                mouse_panel_tab_action(mouse.column, mouse.row, width, height, ui_state)
            {
                action
            } else {
                mouse_watchlist_row_for_position(
                    mouse.column,
                    mouse.row,
                    width,
                    height,
                    ui_state,
                    row_count,
                )
                .map_or_else(
                    || {
                        WorkstationAction::FocusPane(mouse_pane_for_position(
                            mouse.column,
                            mouse.row,
                            width,
                            height,
                        ))
                    },
                    WorkstationAction::SelectRow,
                )
            }
        }),
        _ => None,
    }
}

fn mouse_panel_tab_action(
    column: u16,
    row: u16,
    width: u16,
    height: u16,
    ui_state: &WorkstationUiState,
) -> Option<WorkstationAction> {
    if ui_state.pane_expanded() {
        return mouse_expanded_panel_tab_action(column, row, width, ui_state);
    }

    if width >= 132 {
        let header_height = if width >= 220 { 9 } else { 8 };
        let body_height = height.saturating_sub(header_height).saturating_sub(3);
        let detail_x = width.saturating_mul(30) / 100;
        let detail_width = width.saturating_mul(48) / 100;
        let detail_height = mouse_adaptive_detail_height(ui_state.view(), body_height, 12);
        return mouse_detail_or_chart_tab_action(
            column,
            row,
            MousePanelGeometry {
                x: detail_x,
                y: header_height,
                width: detail_width,
                height: detail_height,
            },
            MousePanelGeometry {
                x: detail_x,
                y: header_height.saturating_add(detail_height),
                width: detail_width,
                height: body_height.saturating_sub(detail_height),
            },
            ui_state,
            false,
        );
    }

    if width >= 90 {
        let body_height = height.saturating_sub(6).saturating_sub(3);
        let detail_x = width.saturating_mul(38) / 100;
        let detail_width = width.saturating_sub(detail_x);
        let detail_height = mouse_adaptive_detail_height(ui_state.view(), body_height, 19);
        return mouse_detail_or_chart_tab_action(
            column,
            row,
            MousePanelGeometry {
                x: detail_x,
                y: 6,
                width: detail_width,
                height: detail_height,
            },
            MousePanelGeometry {
                x: detail_x,
                y: 6_u16.saturating_add(detail_height),
                width: detail_width,
                height: body_height.saturating_sub(detail_height),
            },
            ui_state,
            false,
        );
    }

    let body_height = height.saturating_sub(5).saturating_sub(2);
    let watchlist_height = if ui_state.focused_pane() == WorkstationPane::Status {
        body_height.saturating_mul(36) / 100
    } else {
        body_height.saturating_mul(48) / 100
    };
    let drilldown = MousePanelGeometry {
        x: 0,
        y: 5_u16.saturating_add(watchlist_height),
        width,
        height: body_height.saturating_sub(watchlist_height),
    };
    match ui_state.focused_pane() {
        WorkstationPane::Chart => mouse_chart_tab_action(column, row, drilldown, ui_state, true),
        WorkstationPane::Watchlist | WorkstationPane::Detail => {
            mouse_view_tab_action(column, row, drilldown, ui_state, true)
        }
        WorkstationPane::Book | WorkstationPane::Tape | WorkstationPane::Status => None,
    }
}

fn mouse_expanded_panel_tab_action(
    column: u16,
    row: u16,
    width: u16,
    ui_state: &WorkstationUiState,
) -> Option<WorkstationAction> {
    let expanded_y: u16 = if width < 90 {
        5
    } else if width >= 220 {
        9
    } else if width >= 132 {
        8
    } else {
        6
    };
    let pane = MousePanelGeometry {
        x: 0,
        y: expanded_y.saturating_add(1),
        width,
        height: 1,
    };
    match ui_state.focused_pane() {
        WorkstationPane::Detail => mouse_view_tab_action(column, row, pane, ui_state, width < 90),
        WorkstationPane::Chart => mouse_chart_tab_action(column, row, pane, ui_state, width <= 72),
        _ => None,
    }
}

fn mouse_detail_or_chart_tab_action(
    column: u16,
    row: u16,
    detail: MousePanelGeometry,
    chart: MousePanelGeometry,
    ui_state: &WorkstationUiState,
    force_compact_detail: bool,
) -> Option<WorkstationAction> {
    mouse_view_tab_action(column, row, detail, ui_state, force_compact_detail)
        .or_else(|| mouse_chart_tab_action(column, row, chart, ui_state, chart.width <= 72))
}

fn mouse_view_tab_action(
    column: u16,
    row: u16,
    detail: MousePanelGeometry,
    ui_state: &WorkstationUiState,
    force_compact: bool,
) -> Option<WorkstationAction> {
    if detail.height == 0
        || row != detail.y.saturating_add(2)
        || column <= detail.x
        || column >= detail.x.saturating_add(detail.width).saturating_sub(1)
    {
        return None;
    }
    let compact = force_compact || detail.width <= 72;
    mouse_view_tab_hit(
        column,
        detail.x.saturating_add(1 + "VIEWS ".len() as u16),
        compact,
        ui_state.view(),
    )
    .map(WorkstationAction::SetView)
}

fn mouse_chart_tab_action(
    column: u16,
    row: u16,
    chart: MousePanelGeometry,
    ui_state: &WorkstationUiState,
    compact: bool,
) -> Option<WorkstationAction> {
    if chart.height == 0
        || row != chart.y.saturating_add(1)
        || column <= chart.x
        || column >= chart.x.saturating_add(chart.width).saturating_sub(1)
    {
        return None;
    }
    let prefix_len = if compact {
        "WIN ".len()
    } else {
        "TIMEFRAME RAIL ".len() + "WINDOWS ".len()
    };
    mouse_chart_tab_hit(
        column,
        chart.x.saturating_add(1 + prefix_len as u16),
        compact,
        ui_state.chart_window(),
    )
    .map(WorkstationAction::SetChartWindow)
}

fn mouse_view_tab_hit(
    column: u16,
    start_column: u16,
    compact: bool,
    active: WorkstationView,
) -> Option<WorkstationView> {
    let labels = [
        (WorkstationView::Overview, "overview", "ov"),
        (WorkstationView::Flow, "flow", "fl"),
        (WorkstationView::Quality, "quality", "ql"),
        (WorkstationView::Metadata, "metadata", "mt"),
        (WorkstationView::Explain, "explain", "ex"),
    ];
    let mut cursor = start_column;
    for (index, (view, full, short)) in labels.iter().enumerate() {
        if index > 0 {
            cursor = cursor.saturating_add(1);
        }
        let label = if compact { *short } else { *full };
        let label_width = if *view == active {
            label.len().saturating_add(2)
        } else {
            label.len()
        } as u16;
        if column >= cursor && column < cursor.saturating_add(label_width) {
            return Some(*view);
        }
        cursor = cursor.saturating_add(label_width);
    }
    None
}

fn mouse_chart_tab_hit(
    column: u16,
    start_column: u16,
    compact: bool,
    active: WorkstationChartWindow,
) -> Option<WorkstationChartWindow> {
    let mut cursor = start_column;
    for (index, window) in WorkstationChartWindow::ALL.iter().enumerate() {
        if index > 0 {
            cursor = cursor.saturating_add(1);
        }
        let full_label = window.label();
        let label = if compact {
            full_label.trim_end_matches('m')
        } else {
            full_label
        };
        let label_width = if *window == active {
            label.len().saturating_add(2)
        } else {
            label.len()
        } as u16;
        if column >= cursor && column < cursor.saturating_add(label_width) {
            return Some(*window);
        }
        cursor = cursor.saturating_add(label_width);
    }
    None
}

fn mouse_adaptive_detail_height(
    view: WorkstationView,
    available_height: u16,
    reserved_height: u16,
) -> u16 {
    let desired = match view {
        WorkstationView::Overview | WorkstationView::Flow | WorkstationView::Explain => 10,
        WorkstationView::Quality | WorkstationView::Metadata => 8,
    };
    let max_without_starving_neighbors = available_height.saturating_sub(reserved_height).max(6);
    desired.min(max_without_starving_neighbors).max(6)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct MousePanelGeometry {
    x: u16,
    y: u16,
    width: u16,
    height: u16,
}

fn mouse_header_pane_for_position(
    column: u16,
    row: u16,
    width: u16,
    ui_state: &WorkstationUiState,
) -> Option<WorkstationPane> {
    if width >= 132 {
        let desk_row = if width >= 220 { 3 } else { 2 };
        if row == desk_row {
            return mouse_desk_pane_hit(column, ui_state);
        }
    }

    let controls_row = if width < 90 {
        2
    } else if width >= 220 {
        4
    } else {
        3
    };
    if row != controls_row {
        return None;
    }

    let prefix_len = if (90..132).contains(&width) {
        "CONTROLS LAYOUT DIRECTOR resize-safe | 1-6 focus | z expand | ".len()
    } else {
        "CONTROLS ".len()
    };
    mouse_controls_pane_hit(column, 1 + prefix_len as u16, width < 90, ui_state)
}

fn mouse_desk_pane_hit(column: u16, ui_state: &WorkstationUiState) -> Option<WorkstationPane> {
    let labels = [
        (WorkstationPane::Watchlist, "WATCHLIST 1"),
        (WorkstationPane::Detail, "DETAIL 2"),
        (WorkstationPane::Chart, "CHART 3"),
        (WorkstationPane::Book, "BOOK 4"),
        (WorkstationPane::Tape, "TAPE 5"),
        (WorkstationPane::Status, "OPS 6"),
    ];
    mouse_pane_label_hit(column, 1 + "DESK ".len() as u16, ui_state, &labels)
}

fn mouse_controls_pane_hit(
    column: u16,
    start_column: u16,
    narrow: bool,
    ui_state: &WorkstationUiState,
) -> Option<WorkstationPane> {
    let labels = if narrow {
        [
            (WorkstationPane::Watchlist, "1W"),
            (WorkstationPane::Detail, "2D"),
            (WorkstationPane::Chart, "3C"),
            (WorkstationPane::Book, "4B"),
            (WorkstationPane::Tape, "5T"),
            (WorkstationPane::Status, "6S"),
        ]
    } else {
        [
            (WorkstationPane::Watchlist, "1 WATCH"),
            (WorkstationPane::Detail, "2 DETAIL"),
            (WorkstationPane::Chart, "3 CHART"),
            (WorkstationPane::Book, "4 BOOK"),
            (WorkstationPane::Tape, "5 TAPE"),
            (WorkstationPane::Status, "6 STATUS"),
        ]
    };
    mouse_pane_label_hit(column, start_column, ui_state, &labels)
}

fn mouse_pane_label_hit(
    column: u16,
    start_column: u16,
    ui_state: &WorkstationUiState,
    labels: &[(WorkstationPane, &'static str)],
) -> Option<WorkstationPane> {
    let mut cursor = start_column;
    for (index, (pane, label)) in labels.iter().enumerate() {
        if index > 0 {
            cursor = cursor.saturating_add(1);
        }
        let label_width = if ui_state.focused_pane() == *pane {
            label.len().saturating_add(2)
        } else {
            label.len()
        } as u16;
        if column >= cursor && column < cursor.saturating_add(label_width) {
            return Some(*pane);
        }
        cursor = cursor.saturating_add(label_width);
    }
    None
}

fn mouse_watchlist_row_for_position(
    column: u16,
    row: u16,
    width: u16,
    height: u16,
    ui_state: &WorkstationUiState,
    row_count: usize,
) -> Option<usize> {
    if row_count == 0 {
        return None;
    }
    let table = mouse_watchlist_table_geometry(column, width, height, ui_state)?;
    if row < table.y.saturating_add(2)
        || row >= table.y.saturating_add(table.height).saturating_sub(1)
    {
        return None;
    }
    let clicked_offset = usize::from(row.saturating_sub(table.y).saturating_sub(2));
    let visible_start = mouse_watchlist_visible_start(
        ui_state.selected_index(row_count).unwrap_or_default(),
        row_count,
        ui_state.visible_row_limit(),
        table.height,
    );
    let index = visible_start.saturating_add(clicked_offset);
    (index < row_count).then_some(index)
}

fn mouse_watchlist_table_geometry(
    column: u16,
    width: u16,
    height: u16,
    ui_state: &WorkstationUiState,
) -> Option<MouseTableGeometry> {
    if width >= 132 {
        let header_height = if width >= 220 { 9 } else { 8 };
        let body_height = height.saturating_sub(header_height).saturating_sub(3);
        let watchlist_width = width.saturating_mul(30) / 100;
        if column >= watchlist_width {
            return None;
        }
        let router_height = watchlist_router_height(watchlist_width, body_height, ui_state);
        return Some(MouseTableGeometry {
            y: header_height,
            height: body_height.saturating_sub(router_height),
        });
    }

    if width >= 90 {
        let body_height = height.saturating_sub(6).saturating_sub(3);
        let watchlist_width = width.saturating_mul(38) / 100;
        if column >= watchlist_width {
            return None;
        }
        let router_height = watchlist_router_height(watchlist_width, body_height, ui_state);
        return Some(MouseTableGeometry {
            y: 6,
            height: body_height.saturating_sub(router_height),
        });
    }

    let body_height = height.saturating_sub(5).saturating_sub(2);
    let watchlist_height = if ui_state.focused_pane() == WorkstationPane::Status {
        body_height.saturating_mul(36) / 100
    } else {
        body_height.saturating_mul(48) / 100
    };
    Some(MouseTableGeometry {
        y: 5,
        height: watchlist_height,
    })
}

fn watchlist_router_height(width: u16, height: u16, ui_state: &WorkstationUiState) -> u16 {
    if width < 72 || height < 18 {
        return 0;
    }
    if ui_state.pane_expanded() && ui_state.focused_pane() == WorkstationPane::Watchlist {
        12
    } else if height >= 20 {
        7
    } else {
        4
    }
}

fn mouse_watchlist_visible_start(
    selected: usize,
    row_count: usize,
    density_limit: usize,
    table_height: u16,
) -> usize {
    if row_count == 0 || density_limit == 0 {
        return 0;
    }
    let table_row_capacity = usize::from(table_height.saturating_sub(3)).max(1);
    let capacity = density_limit.min(table_row_capacity).min(row_count);
    let selected = selected.min(row_count - 1);
    let mut start = selected.saturating_sub(capacity / 2);
    if start + capacity > row_count {
        start = row_count - capacity;
    }
    start
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct MouseTableGeometry {
    y: u16,
    height: u16,
}

fn mouse_pane_for_position(column: u16, row: u16, width: u16, height: u16) -> WorkstationPane {
    if row <= 2 || row.saturating_add(2) >= height {
        return WorkstationPane::Status;
    }

    if width >= 128 {
        let watchlist_end = width.saturating_mul(30) / 100;
        let right_start = width.saturating_mul(78) / 100;
        if column < watchlist_end {
            WorkstationPane::Watchlist
        } else if column >= right_start {
            if row < height / 2 {
                WorkstationPane::Book
            } else {
                WorkstationPane::Tape
            }
        } else if row < height / 2 {
            WorkstationPane::Detail
        } else {
            WorkstationPane::Chart
        }
    } else if width >= 88 {
        if row < height / 3 {
            WorkstationPane::Watchlist
        } else if row < (height.saturating_mul(2) / 3) {
            WorkstationPane::Detail
        } else {
            WorkstationPane::Chart
        }
    } else if row < height / 2 {
        WorkstationPane::Watchlist
    } else {
        WorkstationPane::Detail
    }
}

fn apply_live_tui_action(
    action: WorkstationAction,
    ui_state: &mut WorkstationUiState,
    state: &LiveMarketState,
    screen_request: &mut ScreenRequest,
) -> anyhow::Result<()> {
    match action {
        WorkstationAction::SubmitCommand => {
            submit_live_command(ui_state, state, screen_request)?;
        }
        _ => {
            let row_count = current_screened_row_count(state, screen_request)?;
            ui_state.apply(action, row_count);
        }
    }
    Ok(())
}

fn submit_live_command(
    ui_state: &mut WorkstationUiState,
    state: &LiveMarketState,
    screen_request: &mut ScreenRequest,
) -> anyhow::Result<bool> {
    let Some(command) = ui_state.command().cloned() else {
        return Ok(false);
    };
    let input = command.input().trim();
    let mut candidate = screen_request.clone();
    let snapshots = FeatureEngine::default().snapshots(state, now_ms_i64()?);

    match command.target() {
        WorkstationCommandTarget::Filter => {
            candidate.preset = None;
            candidate.where_expr = non_empty_command_value(input);
        }
        WorkstationCommandTarget::Preset => {
            candidate.preset = match input {
                "" | "none" | "clear" => None,
                value => Some(value.to_owned()),
            };
            candidate.where_expr = None;
            candidate.sort = None;
        }
        WorkstationCommandTarget::Sort => {
            candidate.sort = non_empty_command_value(input);
        }
        WorkstationCommandTarget::Symbol => {
            let rows = hls_screen::ScreenEngine.apply(&snapshots, screen_request)?;
            match find_symbol_row_index(&rows, input) {
                Some(index) => {
                    ui_state.select_symbol(rows[index].symbol.clone(), index, rows.len());
                    ui_state.close_command();
                }
                None => {
                    ui_state.set_command_error(format!(
                        "no visible symbol matches '{}'",
                        if input.is_empty() { "<empty>" } else { input }
                    ));
                }
            }
            return Ok(true);
        }
    }

    match hls_screen::ScreenEngine.apply(&snapshots, &candidate) {
        Ok(_) => {
            *screen_request = candidate;
            ui_state.close_command();
        }
        Err(err) => {
            ui_state.set_command_error(live_command_error_message(&err));
        }
    }
    Ok(true)
}

fn non_empty_command_value(input: &str) -> Option<String> {
    (!input.trim().is_empty()).then(|| input.trim().to_owned())
}

fn find_symbol_row_index(rows: &[FeatureSnapshot], input: &str) -> Option<usize> {
    let needle = input.trim().to_ascii_lowercase();
    if needle.is_empty() {
        return None;
    }
    rows.iter()
        .position(|row| row_matches_symbol_query(row, &needle))
}

fn row_matches_symbol_query(row: &FeatureSnapshot, needle: &str) -> bool {
    row.symbol.to_ascii_lowercase().contains(needle)
        || row.metadata.as_ref().is_some_and(|metadata| {
            metadata.display_name.to_ascii_lowercase().contains(needle)
                || metadata
                    .feed_identifier
                    .to_ascii_lowercase()
                    .contains(needle)
                || metadata.symbol.to_ascii_lowercase().contains(needle)
        })
}

fn live_command_error_message(err: &HlsError) -> String {
    let message = err.to_string();
    if message.contains("type-incompatible comparison") {
        "type-incompatible comparison between string and number".to_owned()
    } else {
        message
    }
}

fn current_screened_row_count(
    state: &LiveMarketState,
    screen_request: &ScreenRequest,
) -> anyhow::Result<usize> {
    let snapshots = FeatureEngine::default().snapshots(state, now_ms_i64()?);
    Ok(hls_screen::ScreenEngine
        .apply(&snapshots, screen_request)?
        .len())
}

struct LiveTuiGuard {
    raw_enabled: bool,
    alternate_screen_enabled: bool,
    mouse_capture_enabled: bool,
}

impl LiveTuiGuard {
    fn enable(enabled: bool) -> anyhow::Result<Self> {
        if enabled {
            enable_raw_mode()?;
            if let Err(err) = execute!(io::stderr(), EnterAlternateScreen) {
                let _ = disable_raw_mode();
                return Err(err.into());
            }
            if let Err(err) = execute!(io::stderr(), EnableMouseCapture) {
                let _ = execute!(io::stderr(), LeaveAlternateScreen);
                let _ = disable_raw_mode();
                return Err(err.into());
            }
        }
        Ok(Self {
            raw_enabled: enabled,
            alternate_screen_enabled: enabled,
            mouse_capture_enabled: enabled,
        })
    }
}

impl Drop for LiveTuiGuard {
    fn drop(&mut self) {
        if self.mouse_capture_enabled {
            let _ = execute!(io::stderr(), DisableMouseCapture);
        }
        if self.alternate_screen_enabled {
            let _ = execute!(io::stderr(), LeaveAlternateScreen);
        }
        if self.raw_enabled {
            let _ = disable_raw_mode();
        }
    }
}

fn live_table_title(recording_active: bool) -> &'static str {
    if recording_active {
        "RECORDING Hyperliquid spot live screen"
    } else {
        "READ-ONLY Hyperliquid spot live screen"
    }
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
    use clap::Parser;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use hls_core::{
        metadata::{COHORT_UNKNOWN_METADATA, MetadataEnrichmentInput},
        symbol::MarketSymbol,
    };
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
    fn live_ratatui_viewport_uses_terminal_size_with_workstation_fallback() {
        assert_eq!(
            live_ratatui_viewport_from_size(Some((240, 64))),
            RatatuiViewport {
                width: 240,
                height: 64
            }
        );
        assert_eq!(
            live_ratatui_viewport_from_size(None),
            RatatuiViewport {
                width: 160,
                height: 48
            }
        );
        assert_eq!(
            live_ratatui_viewport_from_size(Some((0, 0))),
            RatatuiViewport {
                width: 160,
                height: 48
            }
        );
    }

    #[test]
    fn live_tui_control_keys_map_to_screen_actions() {
        let state = WorkstationUiState::default();
        assert_eq!(
            key_to_workstation_action(
                KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE),
                &state
            ),
            Some(WorkstationAction::CycleFilter)
        );
        assert_eq!(
            key_to_workstation_action(
                KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE),
                &state
            ),
            Some(WorkstationAction::CyclePreset)
        );
        assert_eq!(
            key_to_workstation_action(
                KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE),
                &state
            ),
            Some(WorkstationAction::CycleSort)
        );
        assert_eq!(
            key_to_workstation_action(
                KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE),
                &state
            ),
            Some(WorkstationAction::CycleChartWindow)
        );
        assert_eq!(
            key_to_workstation_action(
                KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE),
                &state
            ),
            Some(WorkstationAction::OpenSymbolSearch)
        );
        assert_eq!(
            key_to_workstation_action(
                KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE),
                &state
            ),
            Some(WorkstationAction::TogglePaneZoom)
        );
        assert_eq!(
            key_to_workstation_action(
                KeyEvent::new(KeyCode::Char(']'), KeyModifiers::NONE),
                &state
            ),
            Some(WorkstationAction::NextPane)
        );
        assert_eq!(
            key_to_workstation_action(
                KeyEvent::new(KeyCode::Char('['), KeyModifiers::NONE),
                &state
            ),
            Some(WorkstationAction::PreviousPane)
        );
        assert_eq!(
            key_to_workstation_action(
                KeyEvent::new(KeyCode::Char('1'), KeyModifiers::NONE),
                &state
            ),
            Some(WorkstationAction::FocusPane(WorkstationPane::Watchlist))
        );
        assert_eq!(
            key_to_workstation_action(
                KeyEvent::new(KeyCode::Char('4'), KeyModifiers::NONE),
                &state
            ),
            Some(WorkstationAction::FocusPane(WorkstationPane::Book))
        );
        assert_eq!(
            key_to_workstation_action(
                KeyEvent::new(KeyCode::Char('6'), KeyModifiers::NONE),
                &state
            ),
            Some(WorkstationAction::FocusPane(WorkstationPane::Status))
        );
        assert_eq!(
            key_to_workstation_action(
                KeyEvent::new(KeyCode::Char('w'), KeyModifiers::NONE),
                &state
            ),
            Some(WorkstationAction::FocusPane(WorkstationPane::Watchlist))
        );
        assert_eq!(
            key_to_workstation_action(
                KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE),
                &state
            ),
            Some(WorkstationAction::FocusPane(WorkstationPane::Detail))
        );
        assert_eq!(
            key_to_workstation_action(
                KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE),
                &state
            ),
            Some(WorkstationAction::FocusPane(WorkstationPane::Chart))
        );
        assert_eq!(
            key_to_workstation_action(
                KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE),
                &state
            ),
            Some(WorkstationAction::FocusPane(WorkstationPane::Book))
        );
        assert_eq!(
            key_to_workstation_action(
                KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE),
                &state
            ),
            Some(WorkstationAction::FocusPane(WorkstationPane::Tape))
        );
        assert_eq!(
            key_to_workstation_action(
                KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE),
                &state
            ),
            Some(WorkstationAction::FocusPane(WorkstationPane::Status))
        );
        assert_eq!(
            key_to_workstation_action(
                KeyEvent::new(KeyCode::Char('W'), KeyModifiers::SHIFT),
                &state
            ),
            Some(WorkstationAction::FocusPane(WorkstationPane::Watchlist))
        );
        assert_eq!(
            key_to_workstation_action(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), &state),
            Some(WorkstationAction::FocusPane(WorkstationPane::Detail))
        );
        assert_eq!(
            key_to_workstation_action(
                KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE),
                &state
            ),
            Some(WorkstationAction::FocusPane(WorkstationPane::Status))
        );
        assert_eq!(
            key_to_workstation_action(
                KeyEvent::new(KeyCode::Char('H'), KeyModifiers::SHIFT),
                &state
            ),
            Some(WorkstationAction::FocusPane(WorkstationPane::Status))
        );

        let mut command_state = WorkstationUiState::default();
        command_state.apply(WorkstationAction::CycleFilter, 1);
        assert_eq!(
            key_to_workstation_action(
                KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE),
                &command_state
            ),
            Some(WorkstationAction::CommandChar('q'))
        );
        assert_eq!(
            key_to_workstation_action(
                KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
                &command_state
            ),
            Some(WorkstationAction::SubmitCommand)
        );
        assert_eq!(
            key_to_workstation_action(
                KeyEvent::new(KeyCode::Char(']'), KeyModifiers::NONE),
                &command_state
            ),
            Some(WorkstationAction::CommandChar(']'))
        );
        assert_eq!(
            key_to_workstation_action(
                KeyEvent::new(KeyCode::Char('4'), KeyModifiers::NONE),
                &command_state
            ),
            Some(WorkstationAction::CommandChar('4'))
        );
        assert_eq!(
            key_to_workstation_action(
                KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE),
                &command_state
            ),
            Some(WorkstationAction::CommandChar('z'))
        );
        assert_eq!(
            key_to_workstation_action(
                KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE),
                &command_state
            ),
            Some(WorkstationAction::CommandChar('g'))
        );
        assert_eq!(
            key_to_workstation_action(
                KeyEvent::new(KeyCode::Char('w'), KeyModifiers::NONE),
                &command_state
            ),
            Some(WorkstationAction::CommandChar('w'))
        );
        assert_eq!(
            key_to_workstation_action(
                KeyEvent::new(KeyCode::Char('O'), KeyModifiers::SHIFT),
                &command_state
            ),
            Some(WorkstationAction::CommandChar('O'))
        );
    }

    #[test]
    fn live_tui_mouse_events_map_to_keyboard_parity_actions() {
        let state = WorkstationUiState::default();
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::ScrollUp,
                    column: 0,
                    row: 0,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((160, 48)),
                20,
            ),
            Some(WorkstationAction::Up)
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::ScrollDown,
                    column: 0,
                    row: 0,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((160, 48)),
                20,
            ),
            Some(WorkstationAction::Down)
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 10,
                    row: 11,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((160, 48)),
                20,
            ),
            Some(WorkstationAction::SelectRow(1))
        );
        let mut scrolled_state = WorkstationUiState::default();
        scrolled_state.apply(WorkstationAction::PageDown, 20);
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 10,
                    row: 11,
                    modifiers: KeyModifiers::NONE,
                },
                &scrolled_state,
                Some((160, 48)),
                20,
            ),
            Some(WorkstationAction::SelectRow(1))
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 126,
                    row: 30,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((160, 48)),
                20,
            ),
            Some(WorkstationAction::FocusPane(WorkstationPane::Tape))
        );

        let mut command_state = WorkstationUiState::default();
        command_state.apply(WorkstationAction::CycleFilter, 1);
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::ScrollDown,
                    column: 0,
                    row: 0,
                    modifiers: KeyModifiers::NONE,
                },
                &command_state,
                Some((160, 48)),
                20,
            ),
            None
        );
    }

    #[test]
    fn live_tui_mouse_clicks_visible_pane_rails() {
        let state = WorkstationUiState::default();

        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 22,
                    row: 2,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((160, 48)),
                20,
            ),
            Some(WorkstationAction::FocusPane(WorkstationPane::Detail))
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 75,
                    row: 3,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((100, 32)),
                20,
            ),
            Some(WorkstationAction::FocusPane(WorkstationPane::Detail))
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 15,
                    row: 2,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((72, 24)),
                20,
            ),
            Some(WorkstationAction::FocusPane(WorkstationPane::Detail))
        );

        let mut command_state = WorkstationUiState::default();
        command_state.apply(WorkstationAction::CycleFilter, 1);
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 22,
                    row: 2,
                    modifiers: KeyModifiers::NONE,
                },
                &command_state,
                Some((160, 48)),
                20,
            ),
            None
        );
    }

    #[test]
    fn live_tui_mouse_clicks_visible_view_and_chart_tabs() {
        let state = WorkstationUiState::default();

        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 73,
                    row: 10,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((160, 48)),
                20,
            ),
            Some(WorkstationAction::SetView(WorkstationView::Quality))
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 85,
                    row: 19,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((160, 48)),
                20,
            ),
            Some(WorkstationAction::SetChartWindow(
                WorkstationChartWindow::ThirtyMinutes
            ))
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 15,
                    row: 15,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((72, 24)),
                20,
            ),
            Some(WorkstationAction::SetView(WorkstationView::Quality))
        );

        let mut chart_state = WorkstationUiState::default();
        chart_state.apply(WorkstationAction::FocusPane(WorkstationPane::Chart), 20);
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 14,
                    row: 14,
                    modifiers: KeyModifiers::NONE,
                },
                &chart_state,
                Some((72, 24)),
                20,
            ),
            Some(WorkstationAction::SetChartWindow(
                WorkstationChartWindow::ThirtyMinutes
            ))
        );

        let mut command_state = WorkstationUiState::default();
        command_state.apply(WorkstationAction::CycleFilter, 1);
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 73,
                    row: 10,
                    modifiers: KeyModifiers::NONE,
                },
                &command_state,
                Some((160, 48)),
                20,
            ),
            None
        );
    }

    #[test]
    fn live_tui_resize_event_requests_redraw_without_mutating_state() {
        let mut state = WorkstationUiState::default();
        state.apply(WorkstationAction::Down, 3);
        assert_eq!(state.selected_index(3), Some(1));

        assert_eq!(
            live_tui_event_effect(Event::Resize(96, 30), &state, Some((160, 48)), 3),
            LiveTuiEventEffect::Redraw
        );
        assert_eq!(state.selected_index(3), Some(1));
        assert_eq!(state.view(), WorkstationView::Overview);
        assert_eq!(state.focused_pane(), WorkstationPane::Watchlist);
    }

    #[test]
    fn live_tui_event_effect_preserves_key_and_mouse_action_mapping() {
        let state = WorkstationUiState::default();
        assert_eq!(
            live_tui_event_effect(
                Event::Key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE)),
                &state,
                Some((160, 48)),
                20,
            ),
            LiveTuiEventEffect::Action(WorkstationAction::Down)
        );
        assert_eq!(
            live_tui_event_effect(
                Event::Key(KeyEvent {
                    code: KeyCode::Char('j'),
                    modifiers: KeyModifiers::NONE,
                    kind: KeyEventKind::Release,
                    state: crossterm::event::KeyEventState::NONE,
                }),
                &state,
                Some((160, 48)),
                20,
            ),
            LiveTuiEventEffect::Ignore
        );
        assert_eq!(
            live_tui_event_effect(
                Event::Mouse(MouseEvent {
                    kind: MouseEventKind::ScrollDown,
                    column: 0,
                    row: 0,
                    modifiers: KeyModifiers::NONE,
                }),
                &state,
                Some((160, 48)),
                20,
            ),
            LiveTuiEventEffect::Action(WorkstationAction::Down)
        );

        let mut command_state = WorkstationUiState::default();
        command_state.apply(WorkstationAction::CycleFilter, 1);
        assert_eq!(
            live_tui_event_effect(
                Event::Mouse(MouseEvent {
                    kind: MouseEventKind::ScrollDown,
                    column: 0,
                    row: 0,
                    modifiers: KeyModifiers::NONE,
                }),
                &command_state,
                Some((160, 48)),
                20,
            ),
            LiveTuiEventEffect::Ignore
        );
    }

    #[test]
    fn live_tui_force_color_env_flags_are_explicit() {
        assert!(env_flag_value_enabled(Some("1")));
        assert!(env_flag_value_enabled(Some("true")));
        assert!(!env_flag_value_enabled(Some("0")));
        assert!(!env_flag_value_enabled(Some("false")));
        assert!(!env_flag_value_enabled(Some("")));
        assert!(!env_flag_value_enabled(None));
    }

    #[derive(Debug, Parser)]
    struct LiveArgsParseHarness {
        #[command(flatten)]
        args: LiveArgs,
    }

    #[test]
    fn live_tui_defaults_to_colored_workstation_theme() {
        let parsed = LiveArgsParseHarness::try_parse_from(["hls-live", "--tui"])
            .expect("default live tui args parse");

        assert_eq!(parsed.args.color, LiveTuiColor::Always);
        assert_eq!(
            resolve_live_ratatui_color_mode(parsed.args.color, false),
            RatatuiColorMode::Color
        );
    }

    #[test]
    fn live_tui_color_flag_resolves_auto_always_and_never() {
        assert_eq!(
            resolve_live_ratatui_color_mode(LiveTuiColor::Auto, true),
            RatatuiColorMode::Color
        );
        assert_eq!(
            resolve_live_ratatui_color_mode(LiveTuiColor::Auto, false),
            RatatuiColorMode::NoColor
        );
        assert_eq!(
            resolve_live_ratatui_color_mode(LiveTuiColor::Always, false),
            RatatuiColorMode::Color
        );
        assert_eq!(
            resolve_live_ratatui_color_mode(LiveTuiColor::Never, true),
            RatatuiColorMode::NoColor
        );
    }

    #[test]
    fn live_tui_preferences_round_trip_display_state() {
        let temp = tempfile::tempdir().expect("tempdir");
        let preferences = WorkstationUiPreferences {
            view: WorkstationView::Flow,
            density: WorkstationDensity::Dense,
            chart_window: WorkstationChartWindow::ThirtyMinutes,
        };
        let state = WorkstationUiState::from_preferences(preferences);

        assert_eq!(state.view(), WorkstationView::Flow);
        assert_eq!(state.density(), WorkstationDensity::Dense);
        assert_eq!(state.chart_window(), WorkstationChartWindow::ThirtyMinutes);
        assert_eq!(state.preferences(), preferences);

        save_tui_preferences(temp.path(), state.preferences()).expect("preferences save");
        let raw =
            fs::read_to_string(tui_preferences_path(temp.path())).expect("preferences file reads");

        assert!(raw.contains("view = \"flow\""));
        assert!(raw.contains("density = \"dense\""));
        assert!(raw.contains("chart_window = \"30m\""));
        assert_eq!(load_tui_preferences(temp.path()), preferences);
    }

    #[test]
    fn live_tui_preferences_fall_back_for_bad_local_files() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::create_dir_all(temp.path()).expect("create temp data dir");
        let path = tui_preferences_path(temp.path());

        fs::write(
            &path,
            r#"
view = "orders"
density = "dense"
chart_window = "15m"
"#,
        )
        .expect("write unknown preference");
        assert_eq!(
            load_tui_preferences(temp.path()),
            WorkstationUiPreferences::default()
        );

        fs::write(&path, "not valid toml = [").expect("write malformed preference");
        assert_eq!(
            load_tui_preferences(temp.path()),
            WorkstationUiPreferences::default()
        );
    }

    #[test]
    fn live_tui_command_submission_preserves_active_request_on_invalid_filter() {
        let mut state = LiveMarketState::new(["@107".to_owned()]);
        for event in parse_ws_ndjson(include_str!(
            "../../../../tests/fixtures/hyperliquid/ws_mock_live.ndjson"
        ))
        .expect("fixture parses")
        {
            state.apply(event).expect("event applies");
        }
        let mut request = ScreenRequest::default();
        let mut ui_state = WorkstationUiState::default();

        ui_state.apply(WorkstationAction::CycleFilter, 1);
        for ch in "spread_bps < 20".chars() {
            ui_state.apply(WorkstationAction::CommandChar(ch), 1);
        }
        assert!(submit_live_command(&mut ui_state, &state, &mut request).expect("valid applies"));
        assert_eq!(request.where_expr.as_deref(), Some("spread_bps < 20"));
        assert!(ui_state.command().is_none());

        ui_state.apply(WorkstationAction::CycleFilter, 1);
        for ch in "symbol > 10".chars() {
            ui_state.apply(WorkstationAction::CommandChar(ch), 1);
        }
        assert!(submit_live_command(&mut ui_state, &state, &mut request).expect("invalid handled"));

        assert_eq!(request.where_expr.as_deref(), Some("spread_bps < 20"));
        assert_eq!(
            ui_state.command_error(),
            Some("type-incompatible comparison between string and number")
        );
        assert!(ui_state.command().is_some());
    }

    #[test]
    fn live_tui_command_submission_applies_preset_and_sort() {
        let mut state = LiveMarketState::new(["@107".to_owned()]);
        for event in parse_ws_ndjson(include_str!(
            "../../../../tests/fixtures/hyperliquid/ws_mock_live.ndjson"
        ))
        .expect("fixture parses")
        {
            state.apply(event).expect("event applies");
        }
        let mut request = ScreenRequest::default();
        let mut ui_state = WorkstationUiState::default();

        ui_state.apply(WorkstationAction::CyclePreset, 1);
        for ch in "thin_books".chars() {
            ui_state.apply(WorkstationAction::CommandChar(ch), 1);
        }
        assert!(submit_live_command(&mut ui_state, &state, &mut request).expect("preset applies"));
        assert_eq!(request.preset.as_deref(), Some("thin_books"));
        assert!(request.where_expr.is_none());
        assert!(request.sort.is_none());

        ui_state.apply(WorkstationAction::CycleSort, 1);
        for ch in "spread_bps:asc".chars() {
            ui_state.apply(WorkstationAction::CommandChar(ch), 1);
        }
        assert!(submit_live_command(&mut ui_state, &state, &mut request).expect("sort applies"));
        assert_eq!(request.preset.as_deref(), Some("thin_books"));
        assert_eq!(request.sort.as_deref(), Some("spread_bps:asc"));
        assert!(ui_state.command().is_none());
    }

    #[test]
    fn live_tui_command_submission_jumps_to_visible_symbol() {
        let mut state = LiveMarketState::new(["@107".to_owned()]);
        for event in parse_ws_ndjson(include_str!(
            "../../../../tests/fixtures/hyperliquid/ws_mock_live.ndjson"
        ))
        .expect("fixture parses")
        {
            state.apply(event).expect("event applies");
        }
        let mut request = ScreenRequest::default();
        let mut ui_state = WorkstationUiState::default();

        ui_state.apply(WorkstationAction::OpenSymbolSearch, 1);
        for ch in "@107".chars() {
            ui_state.apply(WorkstationAction::CommandChar(ch), 1);
        }
        assert!(submit_live_command(&mut ui_state, &state, &mut request).expect("symbol applies"));
        assert_eq!(ui_state.selected_index(1), Some(0));
        assert_eq!(ui_state.selected_symbol(), Some("@107"));
        assert!(ui_state.command().is_none());
        assert_eq!(request, ScreenRequest::default());

        ui_state.apply(WorkstationAction::OpenSymbolSearch, 1);
        for ch in "NOPE".chars() {
            ui_state.apply(WorkstationAction::CommandChar(ch), 1);
        }
        assert!(submit_live_command(&mut ui_state, &state, &mut request).expect("miss handled"));
        assert_eq!(
            ui_state.command_error(),
            Some("no visible symbol matches 'NOPE'")
        );
        assert!(ui_state.command().is_some());
        assert_eq!(request, ScreenRequest::default());
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

    #[test]
    fn explicit_live_symbol_selectors_resolve_display_names_to_feed_ids() {
        let markets = vec![spot_market("HYPE/USDC", "@107", 107, 150, 0)];

        let selection =
            resolve_explicit_live_symbols(&markets, &["HYPE/USDC".to_owned(), "@107".to_owned()])
                .expect("selectors resolve");

        assert_eq!(selection.symbols, vec!["@107"]);
        assert_eq!(selection.metadata[0].display_name, "HYPE/USDC");
        assert_eq!(selection.metadata[0].feed_identifier, "@107");
    }

    #[test]
    fn explicit_live_symbol_selector_errors_on_unknown_pair() {
        let markets = vec![spot_market("HYPE/USDC", "@107", 107, 150, 0)];

        let err = resolve_explicit_live_symbols(&markets, &["ETH/USDC".to_owned()])
            .expect_err("unknown selector fails");

        assert!(err.to_string().contains("unknown Hyperliquid spot symbol"));
    }

    fn spot_market(
        display_name: &str,
        feed_identifier: &str,
        spot_index: u32,
        base_token_index: u32,
        quote_token_index: u32,
    ) -> SpotMarketContext {
        let symbol = MarketSymbol::new(
            display_name,
            spot_index,
            base_token_index,
            quote_token_index,
            2,
            8,
            true,
        )
        .expect("valid symbol");
        assert_eq!(symbol.hl_coin, feed_identifier);
        let metadata = MetadataEnrichment::from_public_input(MetadataEnrichmentInput {
            symbol: feed_identifier.to_owned(),
            display_name: display_name.to_owned(),
            feed_identifier: feed_identifier.to_owned(),
            spot_index,
            base_token_index,
            quote_token_index,
            metadata_source: "test".to_owned(),
            metadata_fetched_at_ms: 0,
            deploy_time_ms: None,
            deployer: None,
            seeded_usdc: None,
            max_supply: None,
            circulating_supply: None,
            now_ms: 0,
        });
        assert!(metadata.has_tag(COHORT_UNKNOWN_METADATA));

        SpotMarketContext {
            symbol,
            metadata,
            day_ntl_vlm: Some(1.0),
            prev_day_px: None,
            mark_px: None,
            mid_px: None,
            circulating_supply: None,
        }
    }
}
