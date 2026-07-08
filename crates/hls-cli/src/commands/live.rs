use std::{
    fs,
    path::PathBuf,
    time::{Duration, Instant},
};

use anyhow::{Context, bail};
use clap::Args;
use futures_util::{SinkExt, StreamExt};
use hls_core::{
    HlsError, HlsResult,
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
use tokio::time::{interval, sleep_until};
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::commands::record::{default_run_id, enabled_outputs, parse_symbols};

const DEFAULT_WS_URL: &str = "wss://api.hyperliquid.xyz/ws";
const DEFAULT_LIVE_DURATION_SECS: u64 = 60;
const DEFAULT_REFRESH_SECS: u64 = 30;
const DEFAULT_MAX_SUBSCRIPTIONS: usize = 980;

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
    let mut recorder = if args.record {
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
        Duration::from_secs(args.duration_secs),
        Duration::from_secs(args.refresh_secs.max(1)),
        &mut state,
        recorder.as_mut(),
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
    let snapshots = FeatureEngine::default().snapshots(&state, latest_update_ms(&state));
    summary.row_count = snapshots.len();

    println!("live_run=complete");
    println!("symbols={}", symbols.len());
    println!("subscriptions={}", subscription_messages.len());
    println!("streams_per_symbol={}", plan.streams().len());
    println!("ws_messages={}", summary.ws_messages);
    println!("market_events={}", summary.market_events);
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
            &ScreenRequest {
                preset: args.preset,
                where_expr: args.r#where,
                sort: args.sort,
            }
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
    elapsed_secs: u64,
    row_count: usize,
}

async fn drive_live_ws(
    ws_url: &str,
    subscription_messages: &[String],
    duration: Duration,
    refresh_interval: Duration,
    state: &mut LiveMarketState,
    mut recorder: Option<&mut LiveRecorder>,
) -> anyhow::Result<LiveDriveSummary> {
    let (ws, _) = connect_async(ws_url)
        .await
        .map_err(|err| HlsError::External(format!("connect Hyperliquid WebSocket: {err}")))?;
    let (mut write, mut read) = ws.split();

    for message in subscription_messages {
        write
            .send(Message::Text(message.clone().into()))
            .await
            .map_err(|err| HlsError::External(format!("send subscription: {err}")))?;
    }

    let started = Instant::now();
    let deadline = tokio::time::Instant::now() + duration;
    let mut heartbeat = interval(Duration::from_secs(20));
    heartbeat.tick().await;
    let mut progress = interval(refresh_interval);
    progress.tick().await;
    let mut summary = LiveDriveSummary::default();

    loop {
        tokio::select! {
            _ = sleep_until(deadline) => {
                break;
            }
            _ = heartbeat.tick() => {
                write
                    .send(Message::Text(ping_message().to_owned().into()))
                    .await
                    .map_err(|err| HlsError::External(format!("send heartbeat ping: {err}")))?;
            }
            _ = progress.tick() => {
                eprintln!(
                    "live progress: elapsed_secs={} ws_messages={} market_events={}",
                    started.elapsed().as_secs(),
                    summary.ws_messages,
                    summary.market_events
                );
            }
            next = read.next() => {
                let Some(next) = next else {
                    return Err(HlsError::External("Hyperliquid WebSocket closed before duration elapsed".to_owned()).into());
                };
                let message = next
                    .map_err(|err| HlsError::External(format!("read WebSocket message: {err}")))?;
                if let Some(line) = ws_message_text(message, &mut write).await? {
                    summary.ws_messages += 1;
                    if let Some(recorder) = recorder.as_deref_mut() {
                        recorder.record_raw_line(&line)?;
                    }
                    let events = parse_ws_message(&line)?;
                    summary.market_events += events.len() as u64;
                    if let Some(recorder) = recorder.as_deref_mut() {
                        recorder.record_events(&events)?;
                    }
                    for event in events {
                        state.apply(event)?;
                    }
                }
            }
        }
    }

    let _ = write.send(Message::Close(None)).await;
    summary.elapsed_secs = started.elapsed().as_secs();
    Ok(summary)
}

async fn ws_message_text<S>(message: Message, write: &mut S) -> HlsResult<Option<String>>
where
    S: futures_util::Sink<Message> + Unpin,
    <S as futures_util::Sink<Message>>::Error: std::fmt::Display,
{
    match message {
        Message::Text(text) => Ok(Some(text.to_string())),
        Message::Binary(bytes) => String::from_utf8(bytes.to_vec()).map(Some).map_err(|err| {
            HlsError::Parse(format!("binary WebSocket message was not UTF-8: {err}"))
        }),
        Message::Ping(payload) => {
            write
                .send(Message::Pong(payload))
                .await
                .map_err(|err| HlsError::External(format!("send WebSocket pong: {err}")))?;
            Ok(None)
        }
        Message::Pong(_) | Message::Frame(_) => Ok(None),
        Message::Close(frame) => Err(HlsError::External(format!(
            "Hyperliquid WebSocket closed: {frame:?}"
        ))),
    }
}

struct LiveRecorder {
    registry: MetadataRegistry,
    run_id: String,
    raw_writer: Option<RawWriter>,
    normalized_writer: Option<StreamingNormalizedWriter>,
    seq: u64,
    raw_messages: u64,
    normalized_events: u64,
}

impl LiveRecorder {
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

    fn record_raw_line(&mut self, line: &str) -> HlsResult<()> {
        let Some(raw_writer) = &mut self.raw_writer else {
            return Ok(());
        };
        self.seq = self.seq.saturating_add(1);
        let recv_ts_ns = u64::try_from(now_millis()?.saturating_mul(1_000_000))
            .map_err(|_| HlsError::Time("receive timestamp overflowed u64 ns".to_owned()))?;
        let message = RawMarketMessage::from_ws_line(recv_ts_ns, 0, self.seq, line)?;
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
