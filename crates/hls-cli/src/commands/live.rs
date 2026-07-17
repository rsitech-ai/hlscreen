use std::{
    cell::Cell,
    collections::{HashSet, VecDeque},
    fs,
    future::{Future, pending},
    io::{self, IsTerminal, Write},
    path::{Component, Path, PathBuf},
    pin::Pin,
    sync::{
        Mutex,
        atomic::{AtomicU8, Ordering},
        mpsc::{self, Receiver, SyncSender, TrySendError},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, bail};
use clap::{Args, ValueEnum};
use crossterm::{
    cursor::{Hide, Show},
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind,
        KeyModifiers, MouseEvent, MouseEventKind,
    },
    execute,
    style::Colored,
    terminal::{
        EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
        size as terminal_size,
    },
};
use futures_util::{SinkExt, StreamExt};
use hls_core::{
    HlsError, HlsResult,
    alerts::AlertPlaybook,
    data_gap::DataGap,
    market_state::{CandleEvent, FeatureSnapshot, LiveMarketState, MarketEvent, TradeEvent},
    metadata::MetadataEnrichment,
    time::now_millis,
};
use hls_features::{
    alerts::AlertEvaluator,
    engine::{ConfidenceInputs, FeatureEngine},
};
use hls_hyperliquid::{
    rest::{HyperliquidRestClient, SpotMarketContext, select_universe},
    ws::{
        parser::{parse_ws_message, parse_ws_ndjson},
        subscriptions::{
            OFFICIAL_WS_SUBSCRIPTION_LIMIT, StreamKind, SubscriptionPlan, ping_message,
        },
    },
};
use hls_screen::ScreenRequest;
use hls_store::{
    backfill::is_supported_candle_interval,
    metadata::{MetadataRegistry, RecordingRun, SymbolRegistryEntry},
    normalized::StreamingNormalizedWriter,
    parquet::export_normalized_events_to_parquet,
    paths::{prepare_data_file_path, validate_run_id},
    raw::{RawMarketMessage, RawWriter},
    recorder::{RecordOptions, RecordSummary, record_fixture_ndjson},
};
use hls_tui::{
    alerts::{BoundedAlertHistory, TuiAlertRecord},
    app::{render_confidence_summary, render_screened_table},
    interaction::{
        WorkstationAction, WorkstationChartWindow, WorkstationCommandTarget, WorkstationDensity,
        WorkstationPane, WorkstationScrollDirection, WorkstationUiPreferences, WorkstationUiState,
        WorkstationView,
    },
    ratatui_app::{
        RatatuiColorMode, RatatuiFrameModel, RatatuiViewport, render_ratatui_snapshot_for_test,
    },
};
use ratatui::{Terminal, backend::CrosstermBackend};
#[cfg(test)]
use ratatui::{TerminalOptions, Viewport, layout::Rect};
use serde::{Deserialize, Serialize};
use tokio::time::{Interval, MissedTickBehavior, interval, sleep, sleep_until};
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::commands::alerts::load_playbook_file;
use crate::commands::backfill::{self, BackfillArgs, DEFAULT_REST_URL};
use crate::commands::metadata::{attach_metadata, load_metadata_enrichments};
use crate::commands::record::{default_run_id, enabled_outputs, parse_symbols};
use crate::commands::ws_rate_limit::{RollingMessageRateLimiter, RollingRateLimiter};

const DEFAULT_WS_URL: &str = "wss://api.hyperliquid.xyz/ws";
const DEFAULT_LIVE_DURATION_SECS: u64 = 60;
const DEFAULT_TUI_DURATION_SECS: u64 = 0;
const DEFAULT_REFRESH_SECS: u64 = 30;
const DEFAULT_LIVE_TOP: usize = 50;
const DEFAULT_TUI_TOP: usize = 10;
const DEFAULT_TUI_REFRESH_SECS: u64 = 1;
const DEFAULT_MAX_SUBSCRIPTIONS: usize = 980;
const LIVE_RECORDER_QUEUE_CAPACITY: usize = 65_536;
const INITIAL_RECONNECT_BACKOFF_MS: u64 = 1_000;
const MAX_RECONNECT_BACKOFF_MS: u64 = 30_000;
const TUI_KEY_POLL_MS: u64 = 100;
const MAX_TUI_EVENTS_PER_SYMBOL: usize = 64;
const WS_MARKET_DATA_TIMEOUT: Duration = Duration::from_secs(60);
const WS_MARKET_DATA_CHECK_INTERVAL: Duration = Duration::from_secs(5);
const TUI_PREFERENCES_FILE: &str = "tui-preferences.toml";
const MAX_DEFERRED_LIVE_DIAGNOSTICS: usize = 8;
const LIVE_CONNECTION_RATE_BUDGET: usize = 29;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
enum LiveTuiSessionState {
    Inactive = 0,
    Activating = 1,
    Active = 2,
    Interrupted = 3,
}

impl LiveTuiSessionState {
    fn from_raw(value: u8) -> Self {
        match value {
            0 => Self::Inactive,
            1 => Self::Activating,
            2 => Self::Active,
            3 => Self::Interrupted,
            _ => unreachable!("invalid live TUI session state {value}"),
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Inactive => "inactive",
            Self::Activating => "activating",
            Self::Active => "active",
            Self::Interrupted => "interrupted by panic restoration",
        }
    }
}

#[derive(Clone, Copy)]
enum LiveTuiSessionEnforcement {
    Interactive,
    Unmanaged,
}

std::thread_local! {
    static TERMINAL_OPERATION_CONTEXT: Cell<(usize, u32)> = const { Cell::new((0, 0)) };
}

struct TerminalOperationContextGuard {
    previous: (usize, u32),
}

impl TerminalOperationContextGuard {
    fn enter(owner: usize, depth: u32) -> Self {
        let previous = TERMINAL_OPERATION_CONTEXT.with(|context| {
            let previous = context.get();
            context.set((owner, depth));
            previous
        });
        Self { previous }
    }
}

impl Drop for TerminalOperationContextGuard {
    fn drop(&mut self) {
        TERMINAL_OPERATION_CONTEXT.with(|context| context.set(self.previous));
    }
}

struct TerminalOperationCoordinator {
    operation_lock: Mutex<()>,
    state: AtomicU8,
}

impl TerminalOperationCoordinator {
    const fn new() -> Self {
        Self {
            operation_lock: Mutex::new(()),
            state: AtomicU8::new(LiveTuiSessionState::Inactive as u8),
        }
    }

    fn state(&self) -> LiveTuiSessionState {
        LiveTuiSessionState::from_raw(self.state.load(Ordering::Acquire))
    }

    fn set_state(&self, state: LiveTuiSessionState) {
        self.state.store(state as u8, Ordering::Release);
    }

    fn with_operation<R>(&self, operation: impl FnOnce(bool) -> R) -> R {
        let owner = self as *const Self as usize;
        let context = TERMINAL_OPERATION_CONTEXT.with(Cell::get);
        if context.0 == owner && context.1 > 0 {
            let _context = TerminalOperationContextGuard::enter(owner, context.1.saturating_add(1));
            return operation(true);
        }

        let _operation_lock = self
            .operation_lock
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let _context = TerminalOperationContextGuard::enter(owner, 1);
        operation(false)
    }

    fn begin_activation(&self) -> Result<(), LiveTuiSessionState> {
        self.state
            .compare_exchange(
                LiveTuiSessionState::Inactive as u8,
                LiveTuiSessionState::Activating as u8,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .map(|_| ())
            .map_err(LiveTuiSessionState::from_raw)
    }

    fn publish_active(&self) -> Result<(), LiveTuiSessionState> {
        self.state
            .compare_exchange(
                LiveTuiSessionState::Activating as u8,
                LiveTuiSessionState::Active as u8,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .map(|_| ())
            .map_err(LiveTuiSessionState::from_raw)
    }

    fn with_session_operation<R>(
        &self,
        enforcement: LiveTuiSessionEnforcement,
        operation: impl FnOnce() -> R,
    ) -> Result<R, LiveTuiSessionState> {
        self.with_operation(|_| {
            let state = self.state();
            let allowed = matches!(
                (enforcement, state),
                (
                    LiveTuiSessionEnforcement::Interactive,
                    LiveTuiSessionState::Active
                ) | (
                    LiveTuiSessionEnforcement::Unmanaged,
                    LiveTuiSessionState::Inactive
                )
            );
            if !allowed {
                return Err(state);
            }
            Ok(operation())
        })
    }

    fn handle_panic(&self, restore: impl FnOnce(), delegate: impl FnOnce()) {
        self.with_operation(|reentrant| {
            let state = self.state();
            let session_owned = matches!(
                state,
                LiveTuiSessionState::Activating | LiveTuiSessionState::Active
            );
            if session_owned {
                self.set_state(LiveTuiSessionState::Interrupted);
                restore();
            }
            delegate();
            if reentrant && session_owned {
                self.set_state(LiveTuiSessionState::Inactive);
            }
        });
    }

    fn finish_session(&self, restore: impl FnOnce()) -> bool {
        self.with_operation(|_| match self.state() {
            LiveTuiSessionState::Activating | LiveTuiSessionState::Active => {
                restore();
                self.set_state(LiveTuiSessionState::Inactive);
                true
            }
            LiveTuiSessionState::Interrupted => {
                self.set_state(LiveTuiSessionState::Inactive);
                false
            }
            LiveTuiSessionState::Inactive => false,
        })
    }
}

static TERMINAL_OPERATION_COORDINATOR: TerminalOperationCoordinator =
    TerminalOperationCoordinator::new();

#[derive(Default)]
struct LiveDiagnostics {
    deferred: VecDeque<String>,
}

impl LiveDiagnostics {
    fn route(&mut self, tui_owns_stderr: bool, message: impl Into<String>) -> Option<String> {
        let message = message.into();
        if !tui_owns_stderr {
            return Some(message);
        }
        if self.deferred.len() == MAX_DEFERRED_LIVE_DIAGNOSTICS {
            self.deferred.pop_front();
        }
        self.deferred.push_back(message);
        None
    }

    fn emit(&mut self, tui_owns_stderr: bool, message: impl Into<String>) {
        if let Some(message) = self.route(tui_owns_stderr, message) {
            eprintln!("{message}");
        }
    }

    fn take_deferred(&mut self) -> Vec<String> {
        self.deferred.drain(..).collect()
    }

    fn flush_deferred(&mut self) {
        for message in self.take_deferred() {
            eprintln!("{message}");
        }
    }
}

fn after_live_tui_teardown<Recording, Preferences>(
    teardown: impl FnOnce(),
    finish_recording: impl FnOnce() -> Recording,
    save_preferences: impl FnOnce() -> Preferences,
) -> (Recording, Preferences) {
    teardown();
    let recording = finish_recording();
    let preferences = save_preferences();
    (recording, preferences)
}

#[derive(Debug, Args)]
pub struct LiveArgs {
    #[arg(long)]
    pub symbols: Option<String>,

    #[arg(long, default_value_t = DEFAULT_LIVE_TOP)]
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

    /// Evaluate a validated local-only alert playbook in the explicit TUI.
    #[arg(long)]
    pub alert_playbook_file: Option<PathBuf>,

    #[arg(long)]
    pub record: bool,

    #[arg(long)]
    pub raw: bool,

    #[arg(long)]
    pub parquet: bool,

    #[arg(long)]
    pub normalized: bool,

    /// After a clean recorded run, append public candle coverage for reconnect gaps.
    #[arg(long)]
    pub backfill_gaps: bool,

    #[arg(long, default_value = "1m")]
    pub backfill_interval: String,

    #[arg(long, default_value = DEFAULT_REST_URL)]
    pub rest_url: String,

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

#[derive(Debug, Args)]
pub struct TuiArgs {
    #[arg(long)]
    pub symbols: Option<String>,

    #[arg(long, default_value_t = DEFAULT_TUI_TOP)]
    pub top: usize,

    #[arg(long)]
    pub all_symbols: bool,

    #[arg(long, default_value_t = DEFAULT_TUI_DURATION_SECS)]
    pub duration_secs: u64,

    #[arg(long, default_value_t = DEFAULT_TUI_REFRESH_SECS)]
    pub refresh_secs: u64,

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

    /// Evaluate a validated local-only alert playbook in the TUI.
    #[arg(long)]
    pub alert_playbook_file: Option<PathBuf>,

    #[arg(long)]
    pub record: bool,

    #[arg(long)]
    pub raw: bool,

    #[arg(long)]
    pub parquet: bool,

    #[arg(long)]
    pub normalized: bool,

    /// After a clean recorded run, append public candle coverage for reconnect gaps.
    #[arg(long)]
    pub backfill_gaps: bool,

    #[arg(long, default_value = "1m")]
    pub backfill_interval: String,

    #[arg(long, default_value = DEFAULT_REST_URL)]
    pub rest_url: String,

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

impl TuiArgs {
    pub(crate) fn into_live_args(self) -> LiveArgs {
        LiveArgs {
            symbols: self.symbols,
            top: self.top,
            all_symbols: self.all_symbols,
            duration_secs: self.duration_secs,
            refresh_secs: self.refresh_secs,
            tui: true,
            color: self.color,
            max_subscriptions: self.max_subscriptions,
            ws_url: self.ws_url,
            preset: self.preset,
            r#where: self.r#where,
            sort: self.sort,
            alert_playbook_file: self.alert_playbook_file,
            record: self.record,
            raw: self.raw,
            parquet: self.parquet,
            normalized: self.normalized,
            backfill_gaps: self.backfill_gaps,
            backfill_interval: self.backfill_interval,
            rest_url: self.rest_url,
            run_id: self.run_id,
            data_dir: self.data_dir,
            fixture_file: self.fixture_file,
            metadata_file: self.metadata_file,
            once: self.once,
        }
    }
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
    validate_live_args(&args, LiveTerminalCapabilities::detect())?;
    let alert_runtime = load_live_alert_runtime(args.alert_playbook_file.as_ref())?;

    if let Some(fixture_file) = args.fixture_file.clone() {
        return run_fixture_live(args, &fixture_file, alert_runtime).await;
    }

    run_network_live(args, alert_runtime).await
}

struct LiveAlertRuntime {
    playbook: AlertPlaybook,
    evaluator: AlertEvaluator,
    history: BoundedAlertHistory,
    last_observation: Option<Vec<LiveAlertObservation>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct LiveAlertObservation {
    symbol: String,
    confidence_score: u8,
    spread_bps: Option<u64>,
    spread_shock_bps: Option<u64>,
    tob_depth_usd: Option<u64>,
    tob_imbalance: Option<u64>,
    signed_notional_flow_30s: Option<u64>,
    bbo_ofi_proxy_30s: Option<u64>,
    rv_1m: Option<u64>,
    rv_5m: Option<u64>,
    day_ntl_vlm: Option<u64>,
}

impl LiveAlertObservation {
    fn capture(snapshots: &[FeatureSnapshot]) -> Vec<Self> {
        let mut observations = snapshots
            .iter()
            .map(|snapshot| Self {
                symbol: snapshot.symbol.clone(),
                confidence_score: snapshot.confidence.score,
                spread_bps: snapshot.spread_bps.map(f64::to_bits),
                spread_shock_bps: snapshot.spread_shock_bps.map(f64::to_bits),
                tob_depth_usd: snapshot.tob_depth_usd.map(f64::to_bits),
                tob_imbalance: snapshot.tob_imbalance.map(f64::to_bits),
                signed_notional_flow_30s: snapshot.signed_notional_flow_30s.map(f64::to_bits),
                bbo_ofi_proxy_30s: snapshot.bbo_ofi_proxy_30s.map(f64::to_bits),
                rv_1m: snapshot.rv_1m.map(f64::to_bits),
                rv_5m: snapshot.rv_5m.map(f64::to_bits),
                day_ntl_vlm: snapshot.day_ntl_vlm.map(f64::to_bits),
            })
            .collect::<Vec<_>>();
        observations.sort_unstable_by(|left, right| left.symbol.cmp(&right.symbol));
        observations
    }
}

impl LiveAlertRuntime {
    fn new(playbook: AlertPlaybook) -> anyhow::Result<Self> {
        playbook.validate()?;
        Ok(Self {
            playbook,
            evaluator: AlertEvaluator::default(),
            history: BoundedAlertHistory::default(),
            last_observation: None,
        })
    }

    fn evaluate(&mut self, snapshots: &[FeatureSnapshot], now_ms: i64) -> anyhow::Result<()> {
        let observation = LiveAlertObservation::capture(snapshots);
        if self.last_observation.as_ref() == Some(&observation) {
            return Ok(());
        }
        let evaluation = self.evaluator.evaluate(&self.playbook, snapshots, now_ms)?;
        self.last_observation = Some(observation);
        self.history.extend(evaluation.events);
        Ok(())
    }

    fn records(&self) -> Vec<TuiAlertRecord> {
        self.history.records().iter().cloned().collect()
    }
}

fn load_live_alert_runtime(path: Option<&PathBuf>) -> anyhow::Result<Option<LiveAlertRuntime>> {
    path.map(|path| LiveAlertRuntime::new(load_playbook_file(path)?))
        .transpose()
}

fn evaluate_live_alerts(
    runtime: &mut Option<LiveAlertRuntime>,
    snapshots: &[FeatureSnapshot],
    now_ms: i64,
) -> anyhow::Result<Vec<TuiAlertRecord>> {
    let Some(runtime) = runtime.as_mut() else {
        return Ok(Vec::new());
    };
    runtime.evaluate(snapshots, now_ms)?;
    Ok(runtime.records())
}

fn validate_live_args(args: &LiveArgs, terminals: LiveTerminalCapabilities) -> anyhow::Result<()> {
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
    if !args.record && (args.raw || args.normalized || args.parquet || args.run_id.is_some()) {
        bail!("--raw, --normalized, --parquet, and --run-id require --record");
    }
    if args.backfill_gaps && !args.record {
        bail!("--backfill-gaps requires --record");
    }
    let (_, normalized_enabled) = enabled_outputs(args.raw, args.normalized || args.parquet);
    if args.backfill_gaps && !normalized_enabled {
        bail!("--backfill-gaps requires normalized recording output");
    }
    if args.backfill_gaps && !is_supported_candle_interval(&args.backfill_interval) {
        bail!("--backfill-interval must be a supported public candle interval");
    }
    if args.backfill_gaps {
        backfill::validate_rest_url(&args.rest_url)?;
    }
    if let Some(run_id) = args.run_id.as_deref() {
        validate_run_id(run_id)?;
    }
    if args.data_dir.as_os_str().is_empty() {
        bail!("--data-dir must not be empty");
    }
    if args.once && args.fixture_file.is_none() {
        bail!("--once is only supported with --fixture-file");
    }
    if args.fixture_file.is_some() && !args.once && !args.tui {
        bail!("fixture-backed interactive mode requires --tui when --once is absent");
    }
    if args.alert_playbook_file.is_some() && !args.tui {
        bail!(
            "--alert-playbook-file requires the explicit TUI surface (`hls tui` or `hls live --tui`)"
        );
    }
    if args.fixture_file.is_none()
        && !(args.ws_url.starts_with("ws://") || args.ws_url.starts_with("wss://"))
    {
        bail!("--ws-url must use the ws:// or wss:// scheme");
    }

    validate_live_duration(args.duration_secs, args.tui, args.once, terminals)?;
    if !args.once {
        LiveRunLifetime::from_duration_secs(args.duration_secs, tokio::time::Instant::now())?;
    }

    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct LiveTerminalCapabilities {
    stdin_is_terminal: bool,
    stderr_is_terminal: bool,
}

impl LiveTerminalCapabilities {
    fn new(stdin_is_terminal: bool, stderr_is_terminal: bool) -> Self {
        Self {
            stdin_is_terminal,
            stderr_is_terminal,
        }
    }

    fn detect() -> Self {
        Self::new(io::stdin().is_terminal(), io::stderr().is_terminal())
    }

    fn supports_unbounded_tui(self) -> bool {
        self.stdin_is_terminal && self.stderr_is_terminal
    }
}

fn validate_live_duration(
    duration_secs: u64,
    tui: bool,
    once: bool,
    terminals: LiveTerminalCapabilities,
) -> anyhow::Result<()> {
    if once {
        return Ok(());
    }

    if duration_secs == 0 && !tui {
        bail!(
            "--duration-secs 0 is only supported with --tui; use a positive duration for `hls live`"
        );
    }

    if duration_secs == 0 && !terminals.supports_unbounded_tui() {
        bail!(
            "--duration-secs 0 requires an interactive TUI with both stdin and stderr attached to a terminal; pass --duration-secs <positive> for non-interactive runs"
        );
    }

    Ok(())
}

pub async fn run_tui(args: TuiArgs) -> anyhow::Result<()> {
    run(args.into_live_args()).await
}

async fn run_fixture_live(
    args: LiveArgs,
    fixture_file: &PathBuf,
    mut alert_runtime: Option<LiveAlertRuntime>,
) -> anyhow::Result<()> {
    let raw = fs::read_to_string(fixture_file)
        .with_context(|| format!("read {}", fixture_file.display()))?;

    if args.record {
        let run_id = args.run_id.clone().unwrap_or_else(default_run_id);
        let (raw_enabled, normalized_enabled) =
            enabled_outputs(args.raw, args.normalized || args.parquet);
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
        if args.backfill_gaps {
            let outcome = backfill::run_with_outcome(BackfillArgs {
                run_id: summary.run_id.clone(),
                interval: args.backfill_interval.clone(),
                rest_url: args.rest_url.clone(),
                retry: false,
                data_dir: args.data_dir.clone(),
            })
            .await?;
            if !fixture_continues_after_backfill(outcome) {
                return Ok(());
            }
        }
        if args.parquet {
            let parquet = export_normalized_events_to_parquet(&args.data_dir, &summary.run_id)
                .with_context(|| format!("export '{}' to parquet", summary.run_id))?;
            println!("parquet_file={}", parquet.path);
            println!("parquet_rows={}", parquet.rows);
            println!("parquet_bytes={}", parquet.bytes);
        }
    }

    let events = parse_ws_ndjson(&raw)?;
    let symbols = selected_symbols(&args, &events);
    let mut state = LiveMarketState::new(symbols);

    for event in events {
        state.apply(event)?;
    }

    if args.once {
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
        let color_mode = live_ratatui_color_mode(args.color, io::stdout().is_terminal());
        if args.tui {
            let alerts =
                evaluate_live_alerts(&mut alert_runtime, &snapshots, latest_update_ms(&state))?;
            let mut snapshot_ui_state = WorkstationUiState::default();
            if !alerts.is_empty() {
                snapshot_ui_state.apply(
                    WorkstationAction::FocusPane(WorkstationPane::Status),
                    snapshots.len(),
                );
            }
            let model = live_tui_model(
                &snapshots,
                live_table_title(args.record),
                &screen_request,
                Some(&snapshot_ui_state),
                live_tui_candles(&state),
                live_tui_trades(&state),
                LiveTuiStatus::new("fixture", "REC ready", "fixture replay"),
            )
            .with_alerts(alerts);
            let table = render_live_tui_snapshot(&model, None, color_mode)?;
            print!("{table}");
        } else {
            println!("{}", render_confidence_summary(&snapshots));
            let table =
                render_screened_table(&snapshots, live_table_title(args.record), &screen_request)?;
            print!("{table}");
        }
        return Ok(());
    }

    let metadata = load_metadata_enrichments(args.metadata_file.as_ref())?;
    let screen_request = ScreenRequest {
        preset: args.preset.clone(),
        where_expr: args.r#where.clone(),
        sort: args.sort.clone(),
    };
    run_interactive_fixture_tui(&args, state, screen_request, metadata, alert_runtime).await
}

async fn run_interactive_fixture_tui(
    args: &LiveArgs,
    state: LiveMarketState,
    mut screen_request: ScreenRequest,
    metadata: Vec<MetadataEnrichment>,
    alert_runtime: Option<LiveAlertRuntime>,
) -> anyhow::Result<()> {
    let color_mode = live_ratatui_color_mode(args.color, io::stderr().is_terminal());
    let keyboard_interactive = io::stdin().is_terminal() && io::stderr().is_terminal();
    let lifetime =
        LiveRunLifetime::from_duration_secs(args.duration_secs, tokio::time::Instant::now())?;
    let mut diagnostics = LiveDiagnostics::default();
    let preflight = preflight_tui_preferences(&args.data_dir);
    if let Some(warning) = preflight.warning {
        diagnostics.emit(true, warning);
    }
    let mut ui_state = WorkstationUiState::from_preferences(preflight.preferences);
    let mut shutdown_signal = match install_shutdown_signal() {
        Ok(signal) => signal,
        Err(err) => {
            diagnostics.flush_deferred();
            return Err(err);
        }
    };

    let terminal_mode = match LiveTuiGuard::enable(keyboard_interactive) {
        Ok(guard) => guard,
        Err(err) => {
            diagnostics.flush_deferred();
            return Err(err);
        }
    };
    let renderer_enforcement = if keyboard_interactive {
        LiveTuiSessionEnforcement::Interactive
    } else {
        LiveTuiSessionEnforcement::Unmanaged
    };
    let mut tui_renderer = match LiveTuiRenderer::new(renderer_enforcement)
        .map(|renderer| renderer.with_alert_runtime(alert_runtime))
    {
        Ok(renderer) => renderer,
        Err(err) => {
            drop(terminal_mode);
            diagnostics.flush_deferred();
            return Err(err);
        }
    };

    let started = Instant::now();
    let summary = LiveDriveSummary::default();
    let session_result = async {
        render_live_progress(
            LiveProgressContext {
                state: &state,
                screen_request: &screen_request,
                metadata: &metadata,
                color_mode,
                tui_state: Some(&ui_state),
                started,
                summary: &summary,
                mode: LiveProgressMode::Fixture,
            },
            Some(&mut tui_renderer),
        )?;
        wait_for_live_stop(
            lifetime,
            &mut shutdown_signal,
            keyboard_interactive,
            Some(&mut ui_state),
            &state,
            &mut screen_request,
            &metadata,
            color_mode,
            LiveProgressMode::Fixture,
            started,
            &summary,
            Some(&mut tui_renderer),
        )
        .await
    }
    .await;

    let (_, preference_save) = after_live_tui_teardown(
        || {
            drop(tui_renderer);
            drop(terminal_mode);
            diagnostics.flush_deferred();
        },
        || (),
        || save_tui_preferences(&args.data_dir, ui_state.preferences()),
    );
    if let Err(err) = preference_save {
        diagnostics.emit(false, format!("tui preferences save skipped: {err}"));
    }

    session_result.map(|_| ())
}

async fn run_network_live(
    args: LiveArgs,
    alert_runtime: Option<LiveAlertRuntime>,
) -> anyhow::Result<()> {
    let selection = load_live_symbols(&args).await?;
    let symbols = selection.symbols;
    let plan = live_subscription_plan(symbols.clone(), args.all_symbols, args.max_subscriptions);
    let subscription_messages = plan.subscribe_messages()?;
    let mut state = LiveMarketState::new(symbols.clone());
    let run_id = args.run_id.clone().unwrap_or_else(default_run_id);
    let (raw_enabled, normalized_enabled) =
        enabled_outputs(args.raw, args.normalized || args.parquet);
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
    let stderr_tty = io::stderr().is_terminal();
    let render_live_tui = args.tui || stderr_tty;
    let tui_color_mode = live_ratatui_color_mode(args.color, stderr_tty);
    let output_color_mode = live_ratatui_color_mode(args.color, io::stdout().is_terminal());
    let keyboard_interactive = render_live_tui && io::stdin().is_terminal() && stderr_tty;
    let lifetime =
        LiveRunLifetime::from_duration_secs(args.duration_secs, tokio::time::Instant::now())?;
    let mut diagnostics = LiveDiagnostics::default();
    let mut tui_state = if render_live_tui {
        let preflight = preflight_tui_preferences(&args.data_dir);
        if let Some(warning) = preflight.warning {
            diagnostics.emit(true, warning);
        }
        Some(WorkstationUiState::from_preferences(preflight.preferences))
    } else {
        None
    };

    if !render_live_tui {
        eprintln!(
            "read-only live run: symbols={} subscriptions={} streams_per_symbol={} global_streams={} duration_secs={} ws_url={}",
            symbols.len(),
            subscription_messages.len(),
            plan.per_symbol_stream_count(),
            plan.global_stream_count(),
            args.duration_secs,
            args.ws_url
        );
    }
    let mut shutdown_signal = match install_shutdown_signal() {
        Ok(signal) => signal,
        Err(err) => {
            diagnostics.flush_deferred();
            return Err(err);
        }
    };

    let terminal_mode = match LiveTuiGuard::enable(keyboard_interactive) {
        Ok(guard) => guard,
        Err(err) => {
            diagnostics.flush_deferred();
            return Err(err);
        }
    };
    let renderer_enforcement = if keyboard_interactive {
        LiveTuiSessionEnforcement::Interactive
    } else {
        LiveTuiSessionEnforcement::Unmanaged
    };
    let mut tui_renderer = match render_live_tui
        .then(|| {
            LiveTuiRenderer::new(renderer_enforcement)
                .map(|renderer| renderer.with_alert_runtime(alert_runtime))
        })
        .transpose()
    {
        Ok(renderer) => renderer,
        Err(err) => {
            drop(terminal_mode);
            diagnostics.flush_deferred();
            return Err(err);
        }
    };
    let drive_result = drive_live_ws(
        &args.ws_url,
        &subscription_messages,
        &symbols,
        lifetime,
        &mut shutdown_signal,
        Duration::from_secs(args.refresh_secs),
        &mut state,
        &mut screen_request,
        &metadata,
        tui_renderer.as_mut(),
        tui_color_mode,
        keyboard_interactive,
        tui_state.as_mut(),
        recorder.as_ref(),
        &mut diagnostics,
    )
    .await;

    let clean_shutdown = drive_result
        .as_ref()
        .map(LiveDriveSummary::is_clean_shutdown)
        .unwrap_or(false);
    let final_alerts = tui_renderer
        .as_ref()
        .map_or_else(Vec::new, LiveTuiRenderer::alert_records);
    let (record_summary_result, tui_preference_save) = after_live_tui_teardown(
        || {
            drop(tui_renderer);
            drop(terminal_mode);
            diagnostics.flush_deferred();
        },
        || -> anyhow::Result<Option<RecordSummary>> {
            if let Some(recorder) = recorder {
                recorder
                    .finish(clean_shutdown)
                    .map(Some)
                    .map_err(anyhow::Error::from)
            } else {
                Ok(None)
            }
        },
        || {
            if let Some(ui_state) = tui_state.as_ref() {
                save_tui_preferences(&args.data_dir, ui_state.preferences())
            } else {
                Ok(())
            }
        },
    );
    if drive_result.is_err()
        && let Err(err) = &record_summary_result
    {
        diagnostics.emit(
            false,
            format!("recording closeout failed after live error: {err}"),
        );
    }
    if let Err(err) = &tui_preference_save {
        diagnostics.emit(false, format!("tui preferences save skipped: {err}"));
    }

    let mut summary = drive_result?;
    let record_summary = record_summary_result?;
    if let Some(record_summary) = &record_summary {
        run_requested_backfill(
            &args,
            &record_summary.run_id,
            summary.stop_reason,
            &mut shutdown_signal,
        )
        .await?;
    }
    let mut snapshots = live_feature_snapshots(&state, &summary, now_ms_i64()?);
    attach_metadata(&mut snapshots, metadata);
    summary.row_count = snapshots.len();

    println!("live_run=complete");
    println!("symbols={}", symbols.len());
    println!("subscriptions={}", subscription_messages.len());
    println!("streams_per_symbol={}", plan.per_symbol_stream_count());
    println!("global_streams={}", plan.global_stream_count());
    println!("ws_messages={}", summary.ws_messages);
    println!("market_events={}", summary.market_events);
    println!("parser_drops=0");
    println!("reconnects={}", summary.reconnects);
    println!("data_gaps={}", summary.data_gaps);
    println!("alerts_emitted={}", final_alerts.len());
    println!("elapsed_secs={}", summary.elapsed_secs);
    println!(
        "stop_reason={}",
        summary.stop_reason_label().unwrap_or("unknown")
    );
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
        if args.parquet {
            let parquet =
                export_normalized_events_to_parquet(&args.data_dir, &record_summary.run_id)
                    .with_context(|| format!("export '{}' to parquet", record_summary.run_id))?;
            println!("parquet_file={}", parquet.path);
            println!("parquet_rows={}", parquet.rows);
            println!("parquet_bytes={}", parquet.bytes);
        }
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
        )
        .with_alerts(final_alerts);
        render_live_tui_snapshot(&model, None, output_color_mode)?
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

async fn run_requested_backfill(
    args: &LiveArgs,
    run_id: &str,
    live_stop_reason: Option<LiveStopReason>,
    shutdown_signal: &mut ShutdownSignal,
) -> anyhow::Result<()> {
    if !args.backfill_gaps {
        return Ok(());
    }
    if live_stop_reason == Some(LiveStopReason::Signal) {
        eprintln!("backfill_run=stopped stop_reason=signal_before_backfill");
        return Ok(());
    }
    let cancellation = async { shutdown_signal.as_mut().await.map(|_| ()) };
    let Some(summary) = backfill::execute_with_cancellation(
        BackfillArgs {
            run_id: run_id.to_owned(),
            interval: args.backfill_interval.clone(),
            rest_url: args.rest_url.clone(),
            retry: false,
            data_dir: args.data_dir.clone(),
        },
        cancellation,
    )
    .await?
    else {
        eprintln!("backfill_run=stopped stop_reason=signal");
        return Ok(());
    };
    backfill::print_summary(&summary);
    if summary.requests_failed > 0 {
        bail!(
            "{} public candle request(s) failed; unrepaired attempts were recorded",
            summary.requests_failed
        );
    }
    Ok(())
}

fn live_subscription_plan(
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
    stop_reason: Option<LiveStopReason>,
    gap_symbols: HashSet<String>,
    recording_active: bool,
}

impl LiveDriveSummary {
    fn mark_gap(&mut self, symbols: &[String]) {
        self.reconnects = self.reconnects.saturating_add(1);
        self.data_gaps = self.data_gaps.saturating_add(1);
        self.gap_symbols.extend(symbols.iter().cloned());
    }

    fn confidence_inputs(&self) -> ConfidenceInputs {
        self.gap_symbols
            .iter()
            .fold(ConfidenceInputs::default(), |inputs, symbol| {
                inputs.with_gap_symbol(symbol.clone())
            })
    }

    fn recorder_status(&self) -> &'static str {
        if self.recording_active {
            "REC active"
        } else {
            "REC ready"
        }
    }

    fn mark_stopped(&mut self, stop_reason: LiveStopReason) {
        self.stop_reason = Some(stop_reason);
    }

    fn stop_reason_label(&self) -> Option<&'static str> {
        self.stop_reason.map(LiveStopReason::label)
    }

    fn is_clean_shutdown(&self) -> bool {
        self.stop_reason
            .is_some_and(LiveStopReason::is_clean_shutdown)
    }

    fn is_no_market_data_failure(&self) -> bool {
        self.market_events == 0
            && !matches!(
                self.stop_reason,
                Some(LiveStopReason::OperatorQuit | LiveStopReason::Signal)
            )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LiveRunLifetime {
    Bounded(tokio::time::Instant),
    Unbounded,
}

impl LiveRunLifetime {
    fn from_duration_secs(duration_secs: u64, now: tokio::time::Instant) -> HlsResult<Self> {
        if duration_secs == 0 {
            Ok(Self::Unbounded)
        } else {
            now.checked_add(Duration::from_secs(duration_secs))
                .map(Self::Bounded)
                .ok_or_else(|| {
                    HlsError::Config(format!(
                        "--duration-secs value {duration_secs} is too large for this runtime"
                    ))
                })
        }
    }

    fn has_expired_by(self, now: tokio::time::Instant) -> bool {
        matches!(self, Self::Bounded(deadline) if now >= deadline)
    }

    async fn wait_for_expiry(self) {
        match self {
            Self::Bounded(deadline) => sleep_until(deadline).await,
            Self::Unbounded => pending::<()>().await,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LiveStopReason {
    DurationElapsed,
    OperatorQuit,
    Signal,
}

impl LiveStopReason {
    fn label(self) -> &'static str {
        match self {
            Self::DurationElapsed => "duration_elapsed",
            Self::OperatorQuit => "operator_quit",
            Self::Signal => "signal",
        }
    }

    fn is_clean_shutdown(self) -> bool {
        matches!(
            self,
            Self::DurationElapsed | Self::OperatorQuit | Self::Signal
        )
    }
}

type ShutdownSignal = Pin<Box<dyn Future<Output = anyhow::Result<LiveStopReason>> + Send>>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LiveProgressMode {
    Live,
    Fixture,
}

impl LiveProgressMode {
    fn title(self) -> &'static str {
        match self {
            Self::Live => "READ-ONLY Hyperliquid spot live screen",
            Self::Fixture => "READ-ONLY Hyperliquid spot fixture replay",
        }
    }

    fn status(self) -> &'static str {
        match self {
            Self::Live => "LIVE",
            Self::Fixture => "fixture",
        }
    }
}

struct LiveProgressContext<'a> {
    state: &'a LiveMarketState,
    screen_request: &'a ScreenRequest,
    metadata: &'a [MetadataEnrichment],
    color_mode: RatatuiColorMode,
    tui_state: Option<&'a WorkstationUiState>,
    started: Instant,
    summary: &'a LiveDriveSummary,
    mode: LiveProgressMode,
}

#[derive(Debug)]
enum ConnectionOutcome {
    Stopped(LiveStopReason),
    Reconnect {
        conn_id: u64,
        gap_started_at_ns: u64,
        gap_ended_at_ns: u64,
        reason: String,
        received_market_data: bool,
    },
}

#[allow(clippy::too_many_arguments)]
fn reconnect_after_ws_send_failure(
    conn_id: u64,
    connect_started_ns: u64,
    last_market_event_recv_ns: Option<u64>,
    gap_ended_at_ns: u64,
    operation: &str,
    error: impl std::fmt::Display,
    received_market_data: bool,
) -> ConnectionOutcome {
    ConnectionOutcome::Reconnect {
        conn_id,
        gap_started_at_ns: last_market_event_recv_ns.unwrap_or(connect_started_ns),
        gap_ended_at_ns,
        reason: format!("{operation}: {error}"),
        received_market_data,
    }
}

#[derive(Debug)]
enum WsReadEvent {
    Text(String),
    Control,
    Reply(Message),
    Reconnect(String),
}

#[derive(Debug, Eq, PartialEq)]
enum CancellableSendOutcome<T> {
    Sent(T),
    Stopped(LiveStopReason),
}

async fn cancellable_send<SendFuture, StopFuture>(
    send_future: SendFuture,
    stop_future: StopFuture,
) -> anyhow::Result<CancellableSendOutcome<SendFuture::Output>>
where
    SendFuture: Future,
    StopFuture: Future<Output = anyhow::Result<LiveStopReason>>,
{
    tokio::pin!(send_future);
    tokio::pin!(stop_future);

    tokio::select! {
        output = &mut send_future => Ok(CancellableSendOutcome::Sent(output)),
        stop_reason = &mut stop_future => stop_reason.map(CancellableSendOutcome::Stopped),
    }
}

#[allow(clippy::too_many_arguments)]
async fn wait_for_live_stop(
    lifetime: LiveRunLifetime,
    shutdown_signal: &mut ShutdownSignal,
    keyboard_interactive: bool,
    mut tui_state: Option<&mut WorkstationUiState>,
    state: &LiveMarketState,
    screen_request: &mut ScreenRequest,
    metadata: &[MetadataEnrichment],
    color_mode: RatatuiColorMode,
    progress_mode: LiveProgressMode,
    started: Instant,
    summary: &LiveDriveSummary,
    mut tui_frame_sink: Option<&mut LiveTuiRenderer>,
) -> anyhow::Result<LiveStopReason> {
    let mut ui_events = live_interval(Duration::from_millis(TUI_KEY_POLL_MS));

    loop {
        tokio::select! {
            _ = lifetime.wait_for_expiry() => return Ok(LiveStopReason::DurationElapsed),
            result = shutdown_signal.as_mut() => return result,
            _ = ui_events.tick(), if keyboard_interactive => {
                if let Some(stop_reason) = poll_live_tui_actions(
                    tui_state.as_deref_mut(),
                    state,
                    screen_request,
                    metadata,
                    color_mode,
                    progress_mode,
                    started,
                    summary,
                    tui_frame_sink.as_deref_mut(),
                )? {
                    return Ok(stop_reason);
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn send_live_message<S>(
    write: &mut S,
    message: Message,
    outbound_rate_limiter: &mut RollingMessageRateLimiter,
    lifetime: LiveRunLifetime,
    shutdown_signal: &mut ShutdownSignal,
    keyboard_interactive: bool,
    mut tui_state: Option<&mut WorkstationUiState>,
    state: &LiveMarketState,
    screen_request: &mut ScreenRequest,
    metadata: &[MetadataEnrichment],
    color_mode: RatatuiColorMode,
    started: Instant,
    summary: &LiveDriveSummary,
    mut tui_frame_sink: Option<&mut LiveTuiRenderer>,
) -> anyhow::Result<CancellableSendOutcome<Result<(), S::Error>>>
where
    S: futures_util::Sink<Message> + Unpin,
{
    let stop_future = wait_for_live_stop(
        lifetime,
        shutdown_signal,
        keyboard_interactive,
        tui_state.as_deref_mut(),
        state,
        screen_request,
        metadata,
        color_mode,
        LiveProgressMode::Live,
        started,
        summary,
        tui_frame_sink.as_deref_mut(),
    );
    match wait_for_live_outbound_message_slot(outbound_rate_limiter, stop_future).await? {
        CancellableSendOutcome::Sent(()) => {}
        CancellableSendOutcome::Stopped(stop_reason) => {
            return Ok(CancellableSendOutcome::Stopped(stop_reason));
        }
    }

    // Reserve before the write so failed or partially completed sends remain
    // conservatively counted against the exchange's outbound-message limit.
    outbound_rate_limiter.record(tokio::time::Instant::now());
    let stop_future = wait_for_live_stop(
        lifetime,
        shutdown_signal,
        keyboard_interactive,
        tui_state,
        state,
        screen_request,
        metadata,
        color_mode,
        LiveProgressMode::Live,
        started,
        summary,
        tui_frame_sink,
    );
    cancellable_send(write.send(message), stop_future).await
}

#[allow(clippy::too_many_arguments)]
async fn drive_live_ws(
    ws_url: &str,
    subscription_messages: &[String],
    symbols: &[String],
    lifetime: LiveRunLifetime,
    shutdown_signal: &mut ShutdownSignal,
    refresh_interval: Duration,
    state: &mut LiveMarketState,
    screen_request: &mut ScreenRequest,
    metadata: &[MetadataEnrichment],
    mut tui_frame_sink: Option<&mut LiveTuiRenderer>,
    color_mode: RatatuiColorMode,
    keyboard_interactive: bool,
    mut tui_state: Option<&mut WorkstationUiState>,
    recorder: Option<&LiveRecorder>,
    diagnostics: &mut LiveDiagnostics,
) -> anyhow::Result<LiveDriveSummary> {
    let started = Instant::now();
    let mut summary = LiveDriveSummary {
        recording_active: recorder.is_some(),
        ..LiveDriveSummary::default()
    };
    let mut conn_id = 0;
    let mut reconnect_attempt = 0;
    let mut outbound_rate_limiter = RollingMessageRateLimiter::default();
    let mut connection_rate_limiter = RollingRateLimiter::new(
        LIVE_CONNECTION_RATE_BUDGET,
        crate::commands::ws_rate_limit::WS_OUTBOUND_RATE_WINDOW,
    );
    loop {
        if lifetime.has_expired_by(tokio::time::Instant::now()) {
            summary.mark_stopped(LiveStopReason::DurationElapsed);
            break;
        }
        let outcome = drive_live_connection(
            ws_url,
            subscription_messages,
            &mut outbound_rate_limiter,
            &mut connection_rate_limiter,
            conn_id,
            lifetime,
            started,
            refresh_interval,
            state,
            screen_request,
            metadata,
            tui_frame_sink.as_deref_mut(),
            color_mode,
            keyboard_interactive,
            tui_state.as_deref_mut(),
            recorder,
            &mut summary,
            shutdown_signal,
        )
        .await?;

        match outcome {
            ConnectionOutcome::Stopped(stop_reason) => {
                summary.mark_stopped(stop_reason);
                break;
            }
            ConnectionOutcome::Reconnect {
                conn_id: closed_conn_id,
                gap_started_at_ns,
                gap_ended_at_ns,
                reason,
                received_market_data,
            } => {
                summary.mark_gap(symbols);
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
                diagnostics.emit(
                    tui_frame_sink.is_some(),
                    format!(
                        "live reconnect: conn_id={} reason={} backoff_ms={} reconnects={} data_gaps={}",
                        closed_conn_id,
                        reason,
                        backoff.as_millis(),
                        summary.reconnects,
                        summary.data_gaps
                    ),
                );
                conn_id = conn_id.saturating_add(1);
                reconnect_attempt = if received_market_data {
                    0
                } else {
                    reconnect_attempt.saturating_add(1)
                };
                let backoff_wait = sleep(backoff);
                tokio::pin!(backoff_wait);
                let mut ui_events = live_interval(Duration::from_millis(TUI_KEY_POLL_MS));
                ui_events.tick().await;
                let stop_reason = loop {
                    tokio::select! {
                        _ = lifetime.wait_for_expiry() => break Some(LiveStopReason::DurationElapsed),
                        result = shutdown_signal.as_mut() => break Some(result?),
                        _ = ui_events.tick(), if keyboard_interactive => {
                            if let Some(stop_reason) = poll_live_tui_actions(
                                tui_state.as_deref_mut(),
                                state,
                                screen_request,
                                metadata,
                                color_mode,
                                LiveProgressMode::Live,
                                started,
                                &summary,
                                tui_frame_sink.as_deref_mut(),
                            )? {
                                break Some(stop_reason);
                            }
                        }
                        _ = &mut backoff_wait => break None,
                    }
                };
                if let Some(stop_reason) = stop_reason {
                    summary.mark_stopped(stop_reason);
                    break;
                }
            }
        }
    }

    if summary.is_no_market_data_failure() {
        return Err(HlsError::External(format!(
            "live run ended without receiving any market-data events after {} reconnect attempt(s)",
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
    outbound_rate_limiter: &mut RollingMessageRateLimiter,
    connection_rate_limiter: &mut RollingRateLimiter,
    conn_id: u64,
    lifetime: LiveRunLifetime,
    started: Instant,
    refresh_interval: Duration,
    state: &mut LiveMarketState,
    screen_request: &mut ScreenRequest,
    metadata: &[MetadataEnrichment],
    mut tui_frame_sink: Option<&mut LiveTuiRenderer>,
    color_mode: RatatuiColorMode,
    keyboard_interactive: bool,
    mut tui_state: Option<&mut WorkstationUiState>,
    recorder: Option<&LiveRecorder>,
    summary: &mut LiveDriveSummary,
    shutdown_signal: &mut ShutdownSignal,
) -> anyhow::Result<ConnectionOutcome> {
    let stop_future = wait_for_live_stop(
        lifetime,
        shutdown_signal,
        keyboard_interactive,
        tui_state.as_deref_mut(),
        state,
        screen_request,
        metadata,
        color_mode,
        LiveProgressMode::Live,
        started,
        summary,
        tui_frame_sink.as_deref_mut(),
    );
    match wait_for_live_connection_slot(connection_rate_limiter, stop_future).await? {
        CancellableSendOutcome::Sent(()) => {}
        CancellableSendOutcome::Stopped(stop_reason) => {
            return Ok(ConnectionOutcome::Stopped(stop_reason));
        }
    }
    let connect_started_ns = now_ns_u64()?;
    let mut ui_events = live_interval(Duration::from_millis(TUI_KEY_POLL_MS));
    ui_events.tick().await;
    let connect = connect_async(ws_url);
    tokio::pin!(connect);
    let connect_result = loop {
        tokio::select! {
            _ = lifetime.wait_for_expiry() => return Ok(ConnectionOutcome::Stopped(LiveStopReason::DurationElapsed)),
            result = shutdown_signal.as_mut() => return result.map(ConnectionOutcome::Stopped),
            _ = ui_events.tick(), if keyboard_interactive => {
                if let Some(stop_reason) = poll_live_tui_actions(
                    tui_state.as_deref_mut(),
                    state,
                    screen_request,
                    metadata,
                    color_mode,
                    LiveProgressMode::Live,
                    started,
                    summary,
                    tui_frame_sink.as_deref_mut(),
                )? {
                    return Ok(ConnectionOutcome::Stopped(stop_reason));
                }
            }
            result = &mut connect => break result,
        }
    };
    let (ws, _) = match connect_result {
        Ok(value) => value,
        Err(err) => {
            return Ok(ConnectionOutcome::Reconnect {
                conn_id,
                gap_started_at_ns: connect_started_ns,
                gap_ended_at_ns: now_ns_u64()?,
                reason: format!("connect Hyperliquid WebSocket: {err}"),
                received_market_data: false,
            });
        }
    };
    let (mut write, mut read) = ws.split();

    for message in subscription_messages {
        match send_live_message(
            &mut write,
            Message::Text(message.clone().into()),
            outbound_rate_limiter,
            lifetime,
            shutdown_signal,
            keyboard_interactive,
            tui_state.as_deref_mut(),
            state,
            screen_request,
            metadata,
            color_mode,
            started,
            summary,
            tui_frame_sink.as_deref_mut(),
        )
        .await?
        {
            CancellableSendOutcome::Sent(Ok(())) => {}
            CancellableSendOutcome::Sent(Err(err)) => {
                return Ok(ConnectionOutcome::Reconnect {
                    conn_id,
                    gap_started_at_ns: connect_started_ns,
                    gap_ended_at_ns: now_ns_u64()?,
                    reason: format!("send subscription: {err}"),
                    received_market_data: false,
                });
            }
            CancellableSendOutcome::Stopped(stop_reason) => {
                return Ok(ConnectionOutcome::Stopped(stop_reason));
            }
        }
    }

    let mut heartbeat = live_interval(Duration::from_secs(20));
    heartbeat.tick().await;
    let mut progress = live_interval(refresh_interval);
    progress.tick().await;
    let mut ui_events = live_interval(Duration::from_millis(TUI_KEY_POLL_MS));
    ui_events.tick().await;
    let mut inactivity = live_interval(WS_MARKET_DATA_CHECK_INTERVAL);
    inactivity.tick().await;
    let mut last_market_event_recv_ns: Option<u64> = None;
    let mut last_market_data_at = Instant::now();
    let mut received_market_data = false;

    loop {
        tokio::select! {
            _ = lifetime.wait_for_expiry() => {
                return Ok(ConnectionOutcome::Stopped(LiveStopReason::DurationElapsed));
            }
            result = shutdown_signal.as_mut() => {
                return result.map(ConnectionOutcome::Stopped);
            }
            _ = progress.tick() => {
                render_live_progress(LiveProgressContext {
                    state,
                    screen_request,
                    metadata,
                    color_mode,
                    tui_state: tui_state.as_deref(),
                    started,
                    summary,
                    mode: LiveProgressMode::Live,
                }, tui_frame_sink.as_deref_mut())?;
            }
            _ = ui_events.tick(), if keyboard_interactive => {
                if let Some(stop_reason) = poll_live_tui_actions(
                    tui_state.as_deref_mut(),
                    state,
                    screen_request,
                    metadata,
                    color_mode,
                    LiveProgressMode::Live,
                    started,
                    summary,
                    tui_frame_sink.as_deref_mut(),
                )? {
                    return Ok(ConnectionOutcome::Stopped(stop_reason));
                }
            }
            _ = heartbeat.tick() => {
                match send_live_message(
                    &mut write,
                    Message::Text(ping_message().to_owned().into()),
                    outbound_rate_limiter,
                    lifetime,
                    shutdown_signal,
                    keyboard_interactive,
                    tui_state.as_deref_mut(),
                    state,
                    screen_request,
                    metadata,
                    color_mode,
                    started,
                    summary,
                    tui_frame_sink.as_deref_mut(),
                ).await? {
                    CancellableSendOutcome::Sent(Ok(())) => {}
                    CancellableSendOutcome::Sent(Err(err)) => {
                        return Ok(reconnect_after_ws_send_failure(
                            conn_id,
                            connect_started_ns,
                            last_market_event_recv_ns,
                            now_ns_u64()?,
                            "send heartbeat ping",
                            err,
                            received_market_data,
                        ));
                    }
                    CancellableSendOutcome::Stopped(stop_reason) => {
                        return Ok(ConnectionOutcome::Stopped(stop_reason));
                    }
                }
            }
            _ = inactivity.tick() => {
                let now = Instant::now();
                if websocket_inactive(last_market_data_at, now) {
                    return Ok(ConnectionOutcome::Reconnect {
                        conn_id,
                        gap_started_at_ns: last_market_event_recv_ns.unwrap_or(connect_started_ns),
                        gap_ended_at_ns: now_ns_u64()?,
                        reason: format!(
                            "no market-data event for {} seconds",
                            WS_MARKET_DATA_TIMEOUT.as_secs()
                        ),
                        received_market_data,
                    });
                }
            }
            next = read.next() => {
                let recv_ts_ns = now_ns_u64()?;
                let Some(next) = next else {
                    return Ok(ConnectionOutcome::Reconnect {
                        conn_id,
                        gap_started_at_ns: last_market_event_recv_ns.unwrap_or(connect_started_ns),
                        gap_ended_at_ns: recv_ts_ns,
                        reason: "Hyperliquid WebSocket stream ended".to_owned(),
                        received_market_data,
                    });
                };
                let message = match next {
                    Ok(message) => message,
                    Err(err) => {
                        return Ok(ConnectionOutcome::Reconnect {
                            conn_id,
                            gap_started_at_ns: last_market_event_recv_ns.unwrap_or(connect_started_ns),
                            gap_ended_at_ns: recv_ts_ns,
                            reason: format!("read WebSocket message: {err}"),
                            received_market_data,
                        });
                    }
                };
                match ws_message_text(message)? {
                    WsReadEvent::Text(line) => {
                    summary.ws_messages += 1;
                    if let Some(recorder) = recorder {
                        recorder.record_raw_line(recv_ts_ns, conn_id, line.clone())?;
                    }
                    let events: Vec<_> = parse_ws_message(&line)?
                        .into_iter()
                        .map(|event| event.with_recv_ts_ns(recv_ts_ns))
                        .collect();
                    if !events.is_empty() {
                        received_market_data = true;
                        last_market_event_recv_ns = Some(recv_ts_ns);
                        last_market_data_at = Instant::now();
                    }
                    summary.market_events += events.len() as u64;
                    if let Some(recorder) = recorder {
                        recorder.record_events(events.clone())?;
                    }
                    for event in events {
                        state.apply(event)?;
                    }
                    }
                    WsReadEvent::Control => {}
                    WsReadEvent::Reply(reply) => {
                        match send_live_message(
                            &mut write,
                            reply,
                            outbound_rate_limiter,
                            lifetime,
                            shutdown_signal,
                            keyboard_interactive,
                            tui_state.as_deref_mut(),
                            state,
                            screen_request,
                            metadata,
                            color_mode,
                            started,
                            summary,
                            tui_frame_sink.as_deref_mut(),
                        ).await? {
                            CancellableSendOutcome::Sent(Ok(())) => {}
                            CancellableSendOutcome::Sent(Err(err)) => {
                                return Ok(reconnect_after_ws_send_failure(
                                    conn_id,
                                    connect_started_ns,
                                    last_market_event_recv_ns,
                                    recv_ts_ns,
                                    "send WebSocket pong",
                                    err,
                                    received_market_data,
                                ));
                            }
                            CancellableSendOutcome::Stopped(stop_reason) => {
                                return Ok(ConnectionOutcome::Stopped(stop_reason));
                            }
                        }
                    }
                    WsReadEvent::Reconnect(reason) => {
                        return Ok(ConnectionOutcome::Reconnect {
                            conn_id,
                            gap_started_at_ns: last_market_event_recv_ns.unwrap_or(connect_started_ns),
                            gap_ended_at_ns: recv_ts_ns,
                            reason,
                            received_market_data,
                        });
                    }
                }
            }
        }
    }
}

async fn wait_for_live_connection_slot<StopFuture>(
    limiter: &mut RollingRateLimiter,
    stop_future: StopFuture,
) -> anyhow::Result<CancellableSendOutcome<()>>
where
    StopFuture: Future<Output = anyhow::Result<LiveStopReason>>,
{
    let now = tokio::time::Instant::now();
    let available_at = limiter.next_available_at(now).unwrap_or(now);
    tokio::pin!(stop_future);
    tokio::select! {
        biased;
        stop_reason = &mut stop_future => {
            return stop_reason.map(CancellableSendOutcome::Stopped);
        }
        _ = sleep_until(available_at) => {}
    }
    limiter.record(tokio::time::Instant::now());
    Ok(CancellableSendOutcome::Sent(()))
}

async fn wait_for_live_outbound_message_slot<StopFuture>(
    limiter: &mut RollingMessageRateLimiter,
    stop_future: StopFuture,
) -> anyhow::Result<CancellableSendOutcome<()>>
where
    StopFuture: Future<Output = anyhow::Result<LiveStopReason>>,
{
    let now = tokio::time::Instant::now();
    let available_at = limiter.next_available_at(now).unwrap_or(now);
    cancellable_send(sleep_until(available_at), stop_future).await
}

fn fixture_continues_after_backfill(outcome: backfill::BackfillRunOutcome) -> bool {
    outcome == backfill::BackfillRunOutcome::Completed
}

fn ws_message_text(message: Message) -> HlsResult<WsReadEvent> {
    match message {
        Message::Text(text) => Ok(WsReadEvent::Text(text.to_string())),
        Message::Binary(bytes) => String::from_utf8(bytes.to_vec())
            .map(WsReadEvent::Text)
            .map_err(|err| {
                HlsError::Parse(format!("binary WebSocket message was not UTF-8: {err}"))
            }),
        Message::Ping(payload) => Ok(WsReadEvent::Reply(Message::Pong(payload))),
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
    #[cfg(test)]
    WaitForRelease {
        entered: mpsc::Sender<()>,
        release: Receiver<()>,
    },
}

struct LiveRecorder {
    run_id: String,
    sender: Option<SyncSender<LiveRecordCommand>>,
    handle: Option<JoinHandle<HlsResult<RecordSummary>>>,
}

impl LiveRecorder {
    fn new(
        data_dir: &Path,
        run_id: &str,
        symbols: Vec<String>,
        raw_enabled: bool,
        normalized_enabled: bool,
    ) -> HlsResult<Self> {
        Self::new_with_spawner(
            data_dir,
            run_id,
            symbols,
            raw_enabled,
            normalized_enabled,
            |worker| {
                thread::Builder::new()
                    .name("hls-live-recorder".to_owned())
                    .spawn(worker)
            },
        )
    }

    fn new_with_spawner<F>(
        data_dir: &Path,
        run_id: &str,
        symbols: Vec<String>,
        raw_enabled: bool,
        normalized_enabled: bool,
        spawn_worker: F,
    ) -> HlsResult<Self>
    where
        F: FnOnce(
            Box<dyn FnOnce() -> HlsResult<RecordSummary> + Send>,
        ) -> io::Result<JoinHandle<HlsResult<RecordSummary>>>,
    {
        let (sender, receiver) = mpsc::sync_channel(LIVE_RECORDER_QUEUE_CAPACITY);
        let (startup_sender, startup_receiver) = mpsc::sync_channel(1);
        let worker_data_dir = data_dir.to_path_buf();
        let worker_run_id = run_id.to_owned();
        let worker = Box::new(move || {
            let worker = match LiveRecorderWorker::new(
                &worker_data_dir,
                &worker_run_id,
                symbols,
                raw_enabled,
                normalized_enabled,
            ) {
                Ok(worker) => worker,
                Err(error) => {
                    return match startup_sender.send(Err(error)) {
                        Ok(()) => Err(HlsError::External(
                            "live recorder initialization failed and was reported synchronously"
                                .to_owned(),
                        )),
                        Err(send_error) => {
                            match send_error.0 {
                                Err(reported_error) => Err(HlsError::External(format!(
                                    "live recorder initialization failed: {reported_error}; startup handshake also disconnected"
                                ))),
                                Ok(()) => Err(HlsError::External(
                                    "live recorder startup handshake disconnected while reporting an invalid success state"
                                        .to_owned(),
                                )),
                            }
                        }
                    };
                }
            };
            if startup_sender.send(Ok(())).is_err() {
                return match worker.finish(false) {
                    Ok(_) => Err(HlsError::External(
                        "live recorder startup handshake disconnected; run terminalized as unclean"
                            .to_owned(),
                    )),
                    Err(closeout_error) => Err(HlsError::External(format!(
                        "live recorder startup handshake disconnected; unclean terminalization also failed: {closeout_error}"
                    ))),
                };
            }
            worker.run(receiver)
        });
        let handle = spawn_worker(worker)?;

        match startup_receiver.recv() {
            Ok(Ok(())) => {}
            Ok(Err(initialization_error)) => {
                return match handle.join() {
                    Ok(_) => Err(initialization_error),
                    Err(_) => Err(HlsError::External(format!(
                        "live recorder initialization failed: {initialization_error}; worker also panicked while reporting startup failure"
                    ))),
                };
            }
            Err(error) => {
                return match handle.join() {
                    Ok(Ok(_)) => Err(HlsError::External(format!(
                        "live recorder startup handshake disconnected unexpectedly: {error}"
                    ))),
                    Ok(Err(worker_error)) => Err(HlsError::External(format!(
                        "live recorder startup handshake disconnected unexpectedly: {error}; worker failed: {worker_error}"
                    ))),
                    Err(_) => Err(HlsError::External(format!(
                        "live recorder startup handshake disconnected unexpectedly: {error}; worker panicked"
                    ))),
                };
            }
        }

        Ok(Self {
            run_id: run_id.to_owned(),
            sender: Some(sender),
            handle: Some(handle),
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
            false,
        )))
    }

    fn finish(mut self, clean_shutdown: bool) -> HlsResult<RecordSummary> {
        self.shutdown(clean_shutdown)
    }

    fn shutdown(&mut self, clean_shutdown: bool) -> HlsResult<RecordSummary> {
        let send_error = self.sender.take().and_then(|sender| {
            sender
                .send(LiveRecordCommand::Finish { clean_shutdown })
                .err()
                .map(|err| {
                    HlsError::External(format!(
                        "live recorder worker disconnected during shutdown: {err}"
                    ))
                })
        });
        let Some(handle) = self.handle.take() else {
            return Err(HlsError::External(
                "live recorder worker was already shut down".to_owned(),
            ));
        };
        let summary = handle
            .join()
            .map_err(|_| HlsError::External("live recorder worker panicked".to_owned()))??;
        if let Some(err) = send_error {
            return Err(err);
        }
        Ok(summary)
    }

    fn send(&self, command: LiveRecordCommand) -> HlsResult<()> {
        let sender = self.sender.as_ref().ok_or_else(|| {
            HlsError::External("live recorder worker is shutting down".to_owned())
        })?;
        match sender.try_send(command) {
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

impl Drop for LiveRecorder {
    fn drop(&mut self) {
        let _ = self.shutdown(false);
    }
}

struct LiveRecorderWorker {
    registry: MetadataRegistry,
    run_id: String,
    started_at_ms: i64,
    raw_writer: Option<RawWriter>,
    normalized_writer: Option<StreamingNormalizedWriter>,
    seq: u64,
    raw_messages: u64,
    normalized_events: u64,
}

impl LiveRecorderWorker {
    fn new(
        data_dir: &Path,
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
        let initialization = (|| {
            for symbol in &symbols {
                registry.insert_symbol(&SymbolRegistryEntry::new(
                    symbol,
                    started_at_ms,
                    started_at_ms,
                ))?;
            }

            let raw_writer = raw_enabled
                .then(|| RawWriter::new(data_dir, run_id, 8 * 1024 * 1024))
                .transpose()?;
            let normalized_writer = normalized_enabled
                .then(|| StreamingNormalizedWriter::new(data_dir, run_id))
                .transpose()?;
            Ok((raw_writer, normalized_writer))
        })();
        let (raw_writer, normalized_writer) = match initialization {
            Ok(writers) => writers,
            Err(primary_error) => {
                return Err(terminalize_failed_recorder_construction(
                    &registry,
                    run_id,
                    started_at_ms,
                    primary_error,
                ));
            }
        };

        Ok(Self {
            registry,
            run_id: run_id.to_owned(),
            started_at_ms,
            raw_writer,
            normalized_writer,
            seq: 0,
            raw_messages: 0,
            normalized_events: 0,
        })
    }

    fn run(mut self, receiver: Receiver<LiveRecordCommand>) -> HlsResult<RecordSummary> {
        let clean_shutdown = match self.process_commands(receiver) {
            Ok(clean_shutdown) => clean_shutdown,
            Err(worker_err) => {
                return match self.finish(false) {
                    Ok(_) => Err(worker_err),
                    Err(closeout_err) => Err(HlsError::External(format!(
                        "live recorder worker failed: {worker_err}; unclean closeout also failed: {closeout_err}"
                    ))),
                };
            }
        };
        self.finish(clean_shutdown)
    }

    fn process_commands(&mut self, receiver: Receiver<LiveRecordCommand>) -> HlsResult<bool> {
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
                #[cfg(test)]
                LiveRecordCommand::WaitForRelease { entered, release } => {
                    let _ = entered.send(());
                    let _ = release.recv();
                }
            }
        }
        Ok(clean_shutdown)
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
        let mut failures = Vec::new();
        let mut raw_files = Vec::new();
        if let Some(raw_writer) = self.raw_writer.take() {
            match raw_writer.finish() {
                Ok(files) => {
                    raw_files = files;
                    for file in &raw_files {
                        if let Err(error) = self.registry.insert_file(file) {
                            record_recorder_closeout_failure(
                                &mut failures,
                                format!("register raw file '{}': {error}", file.path),
                            );
                        }
                    }
                }
                Err(error) => record_recorder_closeout_failure(
                    &mut failures,
                    format!("finish raw writer: {error}"),
                ),
            }
        }

        let mut normalized_files = Vec::new();
        if let Some(normalized_writer) = self.normalized_writer.take() {
            match normalized_writer.finish() {
                Ok(Some(file)) => {
                    if let Err(error) = self.registry.insert_file(&file) {
                        record_recorder_closeout_failure(
                            &mut failures,
                            format!("register normalized file '{}': {error}", file.path),
                        );
                    }
                    normalized_files.push(file);
                }
                Ok(None) => {}
                Err(error) => record_recorder_closeout_failure(
                    &mut failures,
                    format!("finish normalized writer: {error}"),
                ),
            }
        }

        let ended_at_ms = match now_ms_i64() {
            Ok(ended_at_ms) => ended_at_ms,
            Err(error) => {
                record_recorder_closeout_failure(
                    &mut failures,
                    format!(
                        "resolve recorder end time: {error}; using run start time as explicit fallback"
                    ),
                );
                self.started_at_ms
            }
        };
        let effective_clean_shutdown = clean_shutdown && failures.is_empty();
        if let Err(error) =
            self.registry
                .finish_run(&self.run_id, ended_at_ms, effective_clean_shutdown)
        {
            record_recorder_closeout_failure(
                &mut failures,
                format!("terminalize recording run '{}': {error}", self.run_id),
            );
        }

        if !failures.is_empty() {
            return Err(HlsError::External(format!(
                "live recorder closeout failed: {}",
                failures.join("; ")
            )));
        }
        Ok(RecordSummary {
            run_id: self.run_id,
            raw_files,
            normalized_files,
            raw_messages: self.raw_messages,
            normalized_events: self.normalized_events,
            clean_shutdown: effective_clean_shutdown,
        })
    }
}

fn terminalize_failed_recorder_construction(
    registry: &MetadataRegistry,
    run_id: &str,
    started_at_ms: i64,
    primary_error: HlsError,
) -> HlsError {
    let mut contexts = Vec::new();
    let ended_at_ms = match now_ms_i64() {
        Ok(ended_at_ms) => ended_at_ms,
        Err(error) => {
            contexts.push(format!(
                "resolve recorder end time failed: {error}; used run start time as explicit fallback"
            ));
            started_at_ms
        }
    };
    if let Err(error) = registry.finish_run(run_id, ended_at_ms, false) {
        contexts.push(format!(
            "terminalize failed recording run '{run_id}' failed: {error}"
        ));
    }

    if contexts.is_empty() {
        HlsError::External(format!(
            "live recorder construction failed after run insertion: {primary_error}; run terminalized as unclean"
        ))
    } else {
        HlsError::External(format!(
            "live recorder construction failed after run insertion: {primary_error}; {}",
            contexts.join("; ")
        ))
    }
}

fn record_recorder_closeout_failure(failures: &mut Vec<String>, failure: String) {
    failures.push(failure);
}

fn render_live_progress<S>(
    ctx: LiveProgressContext<'_>,
    tui_frame_sink: Option<&mut S>,
) -> anyhow::Result<()>
where
    S: LiveTuiFrameSink + ?Sized,
{
    if let Some(tui_frame_sink) = tui_frame_sink {
        let mut snapshots = live_feature_snapshots(ctx.state, ctx.summary, now_ms_i64()?);
        attach_metadata(&mut snapshots, ctx.metadata.to_vec());
        let model = live_tui_model(
            &snapshots,
            ctx.mode.title(),
            ctx.screen_request,
            ctx.tui_state,
            live_tui_candles(ctx.state),
            live_tui_trades(ctx.state),
            LiveTuiStatus::new(
                ctx.mode.status(),
                ctx.summary.recorder_status(),
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
        tui_frame_sink.draw(&model, ctx.color_mode)?;
    } else {
        eprintln!(
            "{} progress: elapsed_secs={} ws_messages={} market_events={} reconnects={} data_gaps={}",
            ctx.mode.status().to_ascii_lowercase(),
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

fn live_feature_snapshots(
    state: &LiveMarketState,
    summary: &LiveDriveSummary,
    now_ms: i64,
) -> Vec<FeatureSnapshot> {
    FeatureEngine::default().snapshots_with_confidence_inputs(
        state,
        now_ms,
        &summary.confidence_inputs(),
    )
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
        .flat_map(|state| {
            state
                .candles
                .iter()
                .skip(
                    state
                        .candles
                        .len()
                        .saturating_sub(MAX_TUI_EVENTS_PER_SYMBOL),
                )
                .cloned()
        })
        .collect()
}

fn live_tui_trades(state: &LiveMarketState) -> Vec<TradeEvent> {
    state
        .states()
        .flat_map(|state| {
            state
                .trades
                .iter()
                .skip(state.trades.len().saturating_sub(MAX_TUI_EVENTS_PER_SYMBOL))
                .cloned()
        })
        .collect()
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

fn live_ratatui_color_mode(color: LiveTuiColor, output_is_terminal: bool) -> RatatuiColorMode {
    resolve_live_ratatui_color_mode(color, live_terminal_color_enabled(output_is_terminal))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct LiveTerminalColorDiagnostics {
    pub force_color: bool,
    pub auto_color: bool,
    pub effective_auto_color: bool,
}

pub(crate) fn live_terminal_color_diagnostics() -> LiveTerminalColorDiagnostics {
    live_terminal_color_diagnostics_for(io::stderr().is_terminal())
}

fn live_terminal_color_diagnostics_for(output_is_terminal: bool) -> LiveTerminalColorDiagnostics {
    let force_color = live_terminal_color_forced();
    let auto_color = live_terminal_color_auto_enabled(output_is_terminal);
    LiveTerminalColorDiagnostics {
        force_color,
        auto_color,
        effective_auto_color: force_color || auto_color,
    }
}

fn resolve_live_ratatui_color_mode(color: LiveTuiColor, auto_enabled: bool) -> RatatuiColorMode {
    match color {
        LiveTuiColor::Auto if auto_enabled => RatatuiColorMode::Color,
        LiveTuiColor::Auto | LiveTuiColor::Never => RatatuiColorMode::NoColor,
        LiveTuiColor::Always => RatatuiColorMode::Color,
    }
}

#[cfg(test)]
fn tui_preferences_path(data_dir: &Path) -> PathBuf {
    data_dir.join(TUI_PREFERENCES_FILE)
}

struct TuiPreferencePreflight {
    preferences: WorkstationUiPreferences,
    warning: Option<String>,
}

fn preflight_tui_preferences(data_dir: &Path) -> TuiPreferencePreflight {
    match try_load_tui_preferences(data_dir) {
        Ok(preferences) => TuiPreferencePreflight {
            preferences,
            warning: None,
        },
        Err(err) => TuiPreferencePreflight {
            preferences: WorkstationUiPreferences::default(),
            warning: Some(format!("tui preferences load skipped: {err}")),
        },
    }
}

fn try_load_tui_preferences(data_dir: &Path) -> anyhow::Result<WorkstationUiPreferences> {
    let Some(path) = checked_tui_preferences_path(data_dir, false)? else {
        return Ok(WorkstationUiPreferences::default());
    };

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
    let path = checked_tui_preferences_path(data_dir, true)?.ok_or_else(|| {
        anyhow::anyhow!(
            "TUI preferences destination was unavailable after creating '{}'",
            data_dir.display()
        )
    })?;
    let raw = toml::to_string_pretty(&PersistedTuiPreferences::from_preferences(preferences))
        .context("encode TUI preferences")?;
    let parent = path.parent().ok_or_else(|| {
        anyhow::anyhow!("TUI preferences path '{}' has no parent", path.display())
    })?;
    let mut temporary = tempfile::Builder::new()
        .prefix(".tui-preferences.toml.tmp-")
        .tempfile_in(parent)
        .with_context(|| format!("create temporary TUI preferences in {}", parent.display()))?;
    temporary
        .write_all(raw.as_bytes())
        .with_context(|| format!("write temporary TUI preferences for {}", path.display()))?;
    temporary
        .as_file_mut()
        .flush()
        .with_context(|| format!("flush temporary TUI preferences for {}", path.display()))?;
    temporary
        .as_file()
        .sync_all()
        .with_context(|| format!("sync temporary TUI preferences for {}", path.display()))?;
    reject_symlink_at_path(&path, "TUI preferences destination")?;
    temporary.persist(&path).map_err(|error| {
        anyhow::anyhow!(
            "atomically replace TUI preferences '{}': {}",
            path.display(),
            error.error
        )
    })?;
    Ok(())
}

fn checked_tui_preferences_path(
    data_dir: &Path,
    create_data_dir: bool,
) -> anyhow::Result<Option<PathBuf>> {
    let data_dir = validate_tui_preferences_data_dir_components(data_dir)?;
    match fs::symlink_metadata(&data_dir) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            bail!(
                "refusing symbolic link at TUI preferences data directory '{}'",
                data_dir.display()
            );
        }
        Ok(metadata) if !metadata.is_dir() => {
            bail!(
                "TUI preferences data path '{}' is not a directory",
                data_dir.display()
            );
        }
        Ok(_) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound && !create_data_dir => {
            return Ok(None);
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(error).with_context(|| {
                format!("inspect TUI preferences data path {}", data_dir.display())
            });
        }
    }

    let path = if create_data_dir {
        prepare_data_file_path(&data_dir, TUI_PREFERENCES_FILE).map_err(|error| {
            anyhow::anyhow!(
                "prepare TUI preferences path in '{}': {error}",
                data_dir.display()
            )
        })?
    } else {
        let canonical_data_dir = fs::canonicalize(&data_dir)
            .with_context(|| format!("resolve TUI preferences data path {}", data_dir.display()))?;
        canonical_data_dir.join(TUI_PREFERENCES_FILE)
    };
    match fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            bail!(
                "refusing symbolic link at TUI preferences destination '{}'",
                path.display()
            );
        }
        Ok(metadata) if !metadata.is_file() => {
            bail!(
                "TUI preferences destination '{}' is not a regular file",
                path.display()
            );
        }
        Ok(_) => Ok(Some(path)),
        Err(error) if error.kind() == io::ErrorKind::NotFound && create_data_dir => Ok(Some(path)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => {
            Err(error).with_context(|| format!("inspect TUI preferences path {}", path.display()))
        }
    }
}

fn validate_tui_preferences_data_dir_components(data_dir: &Path) -> anyhow::Result<PathBuf> {
    if data_dir.as_os_str().is_empty() {
        bail!("TUI preferences data directory must not be empty");
    }
    if data_dir
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        bail!(
            "TUI preferences data directory '{}' must not contain parent components",
            data_dir.display()
        );
    }

    let (absolute_data_dir, boundary) = if data_dir.is_absolute() {
        let mut boundary = PathBuf::new();
        for component in data_dir.components() {
            match component {
                Component::Prefix(_) | Component::RootDir => {
                    boundary.push(component.as_os_str());
                }
                Component::CurDir => {}
                Component::Normal(_) => {
                    boundary.push(component.as_os_str());
                    break;
                }
                Component::ParentDir => {
                    bail!(
                        "TUI preferences data directory '{}' contains a parent component",
                        data_dir.display()
                    );
                }
            }
        }
        (data_dir.to_path_buf(), boundary)
    } else {
        let current_dir = std::env::current_dir()
            .context("resolve current directory for relative TUI preferences data path")?;
        (current_dir.join(data_dir), current_dir)
    };

    let relative = absolute_data_dir.strip_prefix(&boundary).with_context(|| {
        format!(
            "resolve TUI preferences data path '{}' beneath lexical boundary '{}'",
            absolute_data_dir.display(),
            boundary.display()
        )
    })?;
    let mut current = boundary;
    for component in relative.components() {
        let Component::Normal(component) = component else {
            if matches!(component, Component::CurDir) {
                continue;
            }
            bail!(
                "TUI preferences data directory '{}' contains an unsupported path component",
                data_dir.display()
            );
        };
        current.push(component);
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                bail!(
                    "refusing symbolic link in TUI preferences data path '{}'",
                    current.display()
                );
            }
            Ok(metadata) if !metadata.is_dir() => {
                bail!(
                    "TUI preferences data path component '{}' is not a directory",
                    current.display()
                );
            }
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => break,
            Err(error) => {
                return Err(error).with_context(|| {
                    format!(
                        "inspect TUI preferences data path component {}",
                        current.display()
                    )
                });
            }
        }
    }
    Ok(absolute_data_dir)
}

fn reject_symlink_at_path(path: &Path, description: &str) -> anyhow::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            bail!(
                "refusing symbolic link at {description} '{}'",
                path.display()
            );
        }
        Ok(_) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => {
            Err(error).with_context(|| format!("inspect {description} {}", path.display()))
        }
    }
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

fn live_terminal_color_enabled(output_is_terminal: bool) -> bool {
    live_terminal_color_diagnostics_for(output_is_terminal).effective_auto_color
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

fn live_terminal_color_auto_enabled(output_is_terminal: bool) -> bool {
    if !output_is_terminal || std::env::var_os("NO_COLOR").is_some() {
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
    summary: &LiveDriveSummary,
) -> anyhow::Result<bool> {
    let mut actions = Vec::new();
    let mut redraw_requested = false;
    let row_count = current_screened_row_count(state, screen_request, summary)?;
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
        apply_live_tui_action(action, ui_state, state, screen_request, summary)?;
    }
    Ok(true)
}

#[allow(clippy::too_many_arguments)]
fn poll_live_tui_actions<S>(
    ui_state: Option<&mut WorkstationUiState>,
    state: &LiveMarketState,
    screen_request: &mut ScreenRequest,
    metadata: &[MetadataEnrichment],
    color_mode: RatatuiColorMode,
    progress_mode: LiveProgressMode,
    started: Instant,
    summary: &LiveDriveSummary,
    mut tui_frame_sink: Option<&mut S>,
) -> anyhow::Result<Option<LiveStopReason>>
where
    S: LiveTuiFrameSink + ?Sized,
{
    let Some(ui_state) = ui_state else {
        return Ok(None);
    };
    let event_redraw = apply_pending_tui_actions(ui_state, state, screen_request, summary)?;
    if live_tui_redraw_requested(event_redraw, tui_frame_sink.as_deref_mut()) {
        render_live_progress(
            LiveProgressContext {
                state,
                screen_request,
                metadata,
                color_mode,
                tui_state: Some(ui_state),
                started,
                summary,
                mode: progress_mode,
            },
            tui_frame_sink,
        )?;
    }

    Ok(operator_stop_reason(ui_state))
}

fn live_tui_redraw_requested<S>(event_redraw: bool, tui_frame_sink: Option<&mut S>) -> bool
where
    S: LiveTuiFrameSink + ?Sized,
{
    let viewport_changed = tui_frame_sink.is_some_and(LiveTuiFrameSink::viewport_changed);
    event_redraw || viewport_changed
}

fn operator_stop_reason(ui_state: &WorkstationUiState) -> Option<LiveStopReason> {
    ui_state
        .quit_requested()
        .then_some(LiveStopReason::OperatorQuit)
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
    if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
        return Some(WorkstationAction::Quit);
    }

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
        KeyCode::Right => Some(WorkstationAction::NextPane),
        KeyCode::Left => Some(WorkstationAction::PreviousPane),
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
        MouseEventKind::ScrollUp => Some(mouse_scroll_action(
            mouse.column,
            mouse.row,
            terminal_size,
            WorkstationScrollDirection::Up,
        )),
        MouseEventKind::ScrollDown => Some(mouse_scroll_action(
            mouse.column,
            mouse.row,
            terminal_size,
            WorkstationScrollDirection::Down,
        )),
        MouseEventKind::Down(_) => terminal_size.map(|(width, height)| {
            if let Some(action) =
                mouse_header_command_action(mouse.column, mouse.row, width, height, ui_state)
            {
                action
            } else if let Some(action) =
                mouse_selected_quote_rail_action(mouse.column, mouse.row, width)
            {
                action
            } else if let Some(action) =
                mouse_market_internals_rail_action(mouse.column, mouse.row, width, ui_state)
            {
                action
            } else if let Some(pane) =
                mouse_header_pane_for_position(mouse.column, mouse.row, width, ui_state)
            {
                mouse_focus_or_zoom_pane_action(pane, ui_state)
            } else if let Some(action) =
                mouse_panel_tab_action(mouse.column, mouse.row, width, height, ui_state)
            {
                action
            } else if let Some(action) =
                mouse_status_action_strip_action(mouse.column, mouse.row, width, height, ui_state)
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

fn mouse_focus_or_zoom_pane_action(
    pane: WorkstationPane,
    ui_state: &WorkstationUiState,
) -> WorkstationAction {
    if pane == ui_state.focused_pane() {
        WorkstationAction::TogglePaneZoom
    } else {
        WorkstationAction::FocusPane(pane)
    }
}

fn mouse_scroll_action(
    column: u16,
    row: u16,
    terminal_size: Option<(u16, u16)>,
    direction: WorkstationScrollDirection,
) -> WorkstationAction {
    terminal_size.map_or_else(
        || match direction {
            WorkstationScrollDirection::Up => WorkstationAction::Up,
            WorkstationScrollDirection::Down => WorkstationAction::Down,
        },
        |(width, height)| {
            WorkstationAction::ScrollPane(
                mouse_pane_for_position(column, row, width, height),
                direction,
            )
        },
    )
}

fn mouse_header_command_action(
    column: u16,
    row: u16,
    width: u16,
    height: u16,
    ui_state: &WorkstationUiState,
) -> Option<WorkstationAction> {
    mouse_micro_command_action(column, row, height, ui_state)
        .or_else(|| mouse_top_command_strip_action(column, row, width, ui_state))
        .or_else(|| mouse_adaptive_desk_command_action(column, row, width, ui_state))
        .or_else(|| mouse_compact_command_cluster_action(column, row, width))
}

fn mouse_micro_command_action(
    column: u16,
    row: u16,
    height: u16,
    ui_state: &WorkstationUiState,
) -> Option<WorkstationAction> {
    if height >= 20 || row != 2 {
        return None;
    }

    let start_column = 1 + "CMD ".len() as u16;
    mouse_compact_command_rail_hit(column, start_column)
        .or_else(|| mouse_micro_pane_action(column, start_column, ui_state))
}

fn mouse_micro_pane_action(
    column: u16,
    command_start_column: u16,
    ui_state: &WorkstationUiState,
) -> Option<WorkstationAction> {
    let pane_start =
        command_start_column.saturating_add("g / p s t d z sp ? q | PANES ".len() as u16);
    mouse_micro_pane_hit(column, pane_start)
        .map(|pane| mouse_focus_or_zoom_pane_action(pane, ui_state))
}

fn mouse_micro_pane_hit(column: u16, start_column: u16) -> Option<WorkstationPane> {
    let labels = [
        (WorkstationPane::Watchlist, "1W"),
        (WorkstationPane::Detail, "2D"),
        (WorkstationPane::Chart, "3C"),
        (WorkstationPane::Book, "4B"),
        (WorkstationPane::Tape, "5T"),
        (WorkstationPane::Status, "6S"),
    ];
    let mut cursor = start_column;
    for (index, (pane, label)) in labels.iter().enumerate() {
        if index > 0 {
            cursor = cursor.saturating_add(1);
        }
        let label_width = label.len() as u16;
        if column >= cursor && column < cursor.saturating_add(label_width) {
            return Some(*pane);
        }
        cursor = cursor.saturating_add(label_width);
    }
    None
}

fn mouse_market_internals_rail_action(
    column: u16,
    row: u16,
    width: u16,
    ui_state: &WorkstationUiState,
) -> Option<WorkstationAction> {
    if row != mouse_market_internals_rail_row(width) {
        return None;
    }

    let pane = if width < 90 {
        mouse_compact_market_internals_pane(column)
    } else {
        mouse_full_market_internals_pane(column)
    };
    Some(mouse_focus_or_zoom_pane_action(pane, ui_state))
}

fn mouse_market_internals_rail_row(width: u16) -> u16 {
    if width < 90 {
        3
    } else if width >= 220 {
        6
    } else if width >= 132 {
        5
    } else {
        4
    }
}

fn mouse_compact_market_internals_pane(column: u16) -> WorkstationPane {
    if column < 35 {
        WorkstationPane::Watchlist
    } else if column < 53 {
        WorkstationPane::Status
    } else if column < 64 {
        WorkstationPane::Tape
    } else {
        WorkstationPane::Book
    }
}

fn mouse_full_market_internals_pane(column: u16) -> WorkstationPane {
    if column < 68 {
        WorkstationPane::Watchlist
    } else if column < 98 {
        WorkstationPane::Status
    } else if column < 126 {
        WorkstationPane::Tape
    } else {
        WorkstationPane::Book
    }
}

fn mouse_selected_quote_rail_action(
    column: u16,
    row: u16,
    width: u16,
) -> Option<WorkstationAction> {
    let quote_row = mouse_selected_quote_rail_row(width)?;
    if row != quote_row {
        return None;
    }

    let pane = if column < 38 {
        WorkstationPane::Detail
    } else if column < 112 {
        WorkstationPane::Book
    } else {
        WorkstationPane::Tape
    };
    Some(WorkstationAction::FocusPane(pane))
}

fn mouse_selected_quote_rail_row(width: u16) -> Option<u16> {
    if width >= 220 {
        Some(4)
    } else if width >= 132 {
        Some(3)
    } else {
        None
    }
}

fn mouse_top_command_strip_action(
    column: u16,
    row: u16,
    width: u16,
    ui_state: &WorkstationUiState,
) -> Option<WorkstationAction> {
    if width < 220 || row != 2 {
        return None;
    }

    let nav_start = 1 + "CMD DOCK ".len() as u16 + "NAV ".len() as u16;
    let nav_labels = [
        (
            "[w/1]WATCH",
            WorkstationAction::FocusPane(WorkstationPane::Watchlist),
        ),
        (
            "[i/2]DETAIL",
            WorkstationAction::FocusPane(WorkstationPane::Detail),
        ),
        (
            "[c/3]CHART",
            WorkstationAction::FocusPane(WorkstationPane::Chart),
        ),
        (
            "[b/4]BOOK",
            WorkstationAction::FocusPane(WorkstationPane::Book),
        ),
        (
            "[r/5]TAPE",
            WorkstationAction::FocusPane(WorkstationPane::Tape),
        ),
        (
            "[o/6]OPS",
            WorkstationAction::FocusPane(WorkstationPane::Status),
        ),
    ];
    if let Some(action) = mouse_double_spaced_action_label_hit(column, nav_start, &nav_labels) {
        return match action {
            WorkstationAction::FocusPane(pane) if pane == ui_state.focused_pane() => {
                Some(WorkstationAction::TogglePaneZoom)
            }
            _ => Some(action),
        };
    }

    let ops_start = nav_start
        .saturating_add(double_spaced_labels_width(&nav_labels))
        .saturating_add("  | OPS ".len() as u16);
    let ops_labels = [
        ("g SYMBOL".to_owned(), WorkstationAction::OpenSymbolSearch),
        ("/ FILTER".to_owned(), WorkstationAction::CycleFilter),
        ("p PRESET".to_owned(), WorkstationAction::CyclePreset),
        ("s SORT".to_owned(), WorkstationAction::CycleSort),
        (
            format!("t WIN:{}", ui_state.chart_window().label()),
            WorkstationAction::CycleChartWindow,
        ),
        (
            format!("d DEN:{}", ui_state.density().label()),
            WorkstationAction::ToggleDensity,
        ),
        (
            format!(
                "z {}",
                if ui_state.pane_expanded() {
                    "grid"
                } else {
                    "zoom"
                }
            ),
            WorkstationAction::TogglePaneZoom,
        ),
        (
            format!("sp {}", if ui_state.paused() { "paused" } else { "live" }),
            WorkstationAction::TogglePause,
        ),
        ("? HELP".to_owned(), WorkstationAction::ToggleHelp),
        ("q QUIT".to_owned(), WorkstationAction::Quit),
    ];
    mouse_double_spaced_owned_action_label_hit(column, ops_start, &ops_labels)
}

fn double_spaced_labels_width(labels: &[(&str, WorkstationAction)]) -> u16 {
    labels
        .iter()
        .enumerate()
        .fold(0_u16, |width, (index, (label, _))| {
            width
                .saturating_add(if index > 0 { 2 } else { 0 })
                .saturating_add(label.len() as u16)
        })
}

fn mouse_double_spaced_action_label_hit(
    column: u16,
    start_column: u16,
    labels: &[(&str, WorkstationAction)],
) -> Option<WorkstationAction> {
    let mut cursor = start_column;
    for (index, (label, action)) in labels.iter().enumerate() {
        if index > 0 {
            cursor = cursor.saturating_add(2);
        }
        let label_width = label.len() as u16;
        if column >= cursor && column < cursor.saturating_add(label_width) {
            return Some(*action);
        }
        cursor = cursor.saturating_add(label_width);
    }
    None
}

fn mouse_double_spaced_owned_action_label_hit(
    column: u16,
    start_column: u16,
    labels: &[(String, WorkstationAction)],
) -> Option<WorkstationAction> {
    let mut cursor = start_column;
    for (index, (label, action)) in labels.iter().enumerate() {
        if index > 0 {
            cursor = cursor.saturating_add(2);
        }
        let label_width = label.len() as u16;
        if column >= cursor && column < cursor.saturating_add(label_width) {
            return Some(*action);
        }
        cursor = cursor.saturating_add(label_width);
    }
    None
}

fn mouse_adaptive_desk_command_action(
    column: u16,
    row: u16,
    width: u16,
    ui_state: &WorkstationUiState,
) -> Option<WorkstationAction> {
    if !(90..220).contains(&width) || row != 2 {
        return None;
    }

    let start_column = if width < 132 {
        1 + "DESK ".len() as u16 + "CMD ".len() as u16
    } else {
        let pane_start = 1 + "DESK ".len() as u16;
        pane_start
            .saturating_add(desk_pane_labels_width(ui_state))
            .saturating_add(" | CMD ".len() as u16)
    };
    mouse_compact_command_rail_hit(column, start_column)
}

fn desk_pane_labels_width(ui_state: &WorkstationUiState) -> u16 {
    let labels = [
        (WorkstationPane::Watchlist, "WATCHLIST 1"),
        (WorkstationPane::Detail, "DETAIL 2"),
        (WorkstationPane::Chart, "CHART 3"),
        (WorkstationPane::Book, "BOOK 4"),
        (WorkstationPane::Tape, "TAPE 5"),
        (WorkstationPane::Status, "OPS 6"),
    ];
    labels
        .iter()
        .enumerate()
        .fold(0_u16, |width, (index, (pane, label))| {
            let spacing = if index > 0 { 1 } else { 0 };
            let label_width = if ui_state.focused_pane() == *pane {
                label.len().saturating_add(2)
            } else {
                label.len()
            } as u16;
            width.saturating_add(spacing).saturating_add(label_width)
        })
}

fn mouse_compact_command_rail_hit(column: u16, start_column: u16) -> Option<WorkstationAction> {
    if column < start_column {
        return None;
    }
    match column.saturating_sub(start_column) {
        0 => Some(WorkstationAction::OpenSymbolSearch),
        2 => Some(WorkstationAction::CycleFilter),
        4 => Some(WorkstationAction::CyclePreset),
        6 => Some(WorkstationAction::CycleSort),
        8 => Some(WorkstationAction::CycleChartWindow),
        10 => Some(WorkstationAction::ToggleDensity),
        12 => Some(WorkstationAction::TogglePaneZoom),
        14 | 15 => Some(WorkstationAction::TogglePause),
        17 => Some(WorkstationAction::ToggleHelp),
        19 => Some(WorkstationAction::Quit),
        _ => None,
    }
}

fn mouse_compact_command_cluster_action(
    column: u16,
    row: u16,
    width: u16,
) -> Option<WorkstationAction> {
    if width >= 90 || row != 2 {
        return None;
    }

    let cluster_start = 1 + "CONTROLS [1W] 2D 3C 4B 5T 6S | w/i/c/b/r/o | j/k ent ".len() as u16;
    mouse_short_command_cluster_hit(column, cluster_start)
}

fn mouse_short_command_cluster_hit(column: u16, start_column: u16) -> Option<WorkstationAction> {
    if column < start_column {
        return None;
    }
    match column.saturating_sub(start_column) {
        0 => Some(WorkstationAction::CycleFilter),
        1 => Some(WorkstationAction::CyclePreset),
        2 => Some(WorkstationAction::CycleSort),
        3 => Some(WorkstationAction::CycleChartWindow),
        4 => Some(WorkstationAction::ToggleDensity),
        5 => Some(WorkstationAction::TogglePaneZoom),
        6 | 7 => Some(WorkstationAction::TogglePause),
        9 => Some(WorkstationAction::FocusPane(WorkstationPane::Status)),
        10 => Some(WorkstationAction::ToggleHelp),
        12 => Some(WorkstationAction::Quit),
        _ => None,
    }
}

fn mouse_status_action_strip_action(
    column: u16,
    row: u16,
    width: u16,
    height: u16,
    ui_state: &WorkstationUiState,
) -> Option<WorkstationAction> {
    if width < 90 || row != height.saturating_sub(1) {
        return None;
    }

    let zoom_label = if ui_state.pane_expanded() {
        "z unzoom"
    } else {
        "z zoom"
    };
    let start_column = "ACTION STRIP ".len() as u16;

    if width < 132 {
        let labels = [
            ("j/k", WorkstationAction::Down),
            ("ent", WorkstationAction::FocusPane(WorkstationPane::Detail)),
            ("tab", WorkstationAction::NextView),
            ("g", WorkstationAction::OpenSymbolSearch),
            (zoom_label, WorkstationAction::TogglePaneZoom),
            ("d", WorkstationAction::ToggleDensity),
            ("sp", WorkstationAction::TogglePause),
            ("/", WorkstationAction::CycleFilter),
            ("p", WorkstationAction::CyclePreset),
            ("s", WorkstationAction::CycleSort),
            ("t", WorkstationAction::CycleChartWindow),
            ("?", WorkstationAction::ToggleHelp),
            ("q", WorkstationAction::Quit),
        ];
        return mouse_spaced_action_label_hit(column, start_column, &labels);
    }

    let labels = [
        ("j/k row", WorkstationAction::Down),
        (
            "ent detail",
            WorkstationAction::FocusPane(WorkstationPane::Detail),
        ),
        ("tab view", WorkstationAction::NextView),
        ("g symbol", WorkstationAction::OpenSymbolSearch),
        (zoom_label, WorkstationAction::TogglePaneZoom),
        ("d density", WorkstationAction::ToggleDensity),
        ("space pause", WorkstationAction::TogglePause),
        ("/ filter", WorkstationAction::CycleFilter),
        ("p preset", WorkstationAction::CyclePreset),
        ("s sort", WorkstationAction::CycleSort),
        ("t win", WorkstationAction::CycleChartWindow),
        ("? help", WorkstationAction::ToggleHelp),
        ("q quit", WorkstationAction::Quit),
    ];
    mouse_spaced_action_label_hit(column, start_column, &labels)
}

fn mouse_spaced_action_label_hit(
    column: u16,
    start_column: u16,
    labels: &[(&str, WorkstationAction)],
) -> Option<WorkstationAction> {
    let mut cursor = start_column;
    for (index, (label, action)) in labels.iter().enumerate() {
        if index > 0 {
            cursor = cursor.saturating_add(1);
        }
        let label_width = label.len() as u16;
        if column >= cursor && column < cursor.saturating_add(label_width) {
            return Some(*action);
        }
        cursor = cursor.saturating_add(label_width);
    }
    None
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
        5
    } else if width >= 132 {
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
    summary: &LiveDriveSummary,
) -> anyhow::Result<()> {
    match action {
        WorkstationAction::SubmitCommand => {
            submit_live_command(ui_state, state, screen_request, summary)?;
        }
        _ => {
            let row_count = current_screened_row_count(state, screen_request, summary)?;
            ui_state.apply(action, row_count);
        }
    }
    Ok(())
}

fn submit_live_command(
    ui_state: &mut WorkstationUiState,
    state: &LiveMarketState,
    screen_request: &mut ScreenRequest,
    summary: &LiveDriveSummary,
) -> anyhow::Result<bool> {
    let Some(command) = ui_state.command().cloned() else {
        return Ok(false);
    };
    let input = command.input().trim();
    let mut candidate = screen_request.clone();
    let snapshots = live_feature_snapshots(state, summary, now_ms_i64()?);

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
    summary: &LiveDriveSummary,
) -> anyhow::Result<usize> {
    let snapshots = live_feature_snapshots(state, summary, now_ms_i64()?);
    Ok(hls_screen::ScreenEngine
        .apply(&snapshots, screen_request)?
        .len())
}

fn restore_live_tui_terminal() {
    let mut stderr = io::stderr();
    let _ = execute!(stderr, DisableMouseCapture);
    let _ = execute!(stderr, Show);
    let _ = execute!(stderr, LeaveAlternateScreen);
    let _ = disable_raw_mode();
}

pub(crate) fn handle_terminal_panic(delegate: impl FnOnce()) {
    TERMINAL_OPERATION_COORDINATOR.handle_panic(restore_live_tui_terminal, delegate);
}

struct LiveTuiGuard {
    enabled: bool,
}

impl LiveTuiGuard {
    fn enable(enabled: bool) -> anyhow::Result<Self> {
        if !enabled {
            return Ok(Self { enabled: false });
        }
        TERMINAL_OPERATION_COORDINATOR.with_operation(|_| {
            if let Err(state) = TERMINAL_OPERATION_COORDINATOR.begin_activation() {
                bail!(
                    "cannot activate the live TUI while its terminal session is {}; retry after the current session closes",
                    state.label()
                );
            }

            let activation = (|| -> anyhow::Result<()> {
                enable_raw_mode()?;
                let mut stderr = io::stderr();
                execute!(stderr, EnterAlternateScreen)?;
                execute!(stderr, EnableMouseCapture)?;
                execute!(stderr, Hide)?;
                Ok(())
            })();
            if let Err(err) = activation {
                restore_live_tui_terminal();
                TERMINAL_OPERATION_COORDINATOR.set_state(LiveTuiSessionState::Inactive);
                return Err(err);
            }
            if let Err(state) = TERMINAL_OPERATION_COORDINATOR.publish_active() {
                restore_live_tui_terminal();
                TERMINAL_OPERATION_COORDINATOR.set_state(LiveTuiSessionState::Inactive);
                bail!(
                    "live TUI activation was interrupted while the terminal session became {}; terminal state was restored, so retry the command",
                    state.label()
                );
            }
            Ok(Self { enabled: true })
        })
    }
}

impl Drop for LiveTuiGuard {
    fn drop(&mut self) {
        if self.enabled {
            TERMINAL_OPERATION_COORDINATOR.finish_session(restore_live_tui_terminal);
        }
    }
}

trait LiveTuiFrameSink {
    fn viewport_changed(&mut self) -> bool {
        false
    }

    fn draw(
        &mut self,
        model: &RatatuiFrameModel,
        color_mode: RatatuiColorMode,
    ) -> anyhow::Result<()>;
}

#[derive(Default)]
struct LiveTuiPresentationCache {
    last_market_frame: Option<RatatuiFrameModel>,
}

impl LiveTuiPresentationCache {
    fn present(&mut self, model: &RatatuiFrameModel) -> RatatuiFrameModel {
        if model.ui_paused() {
            let presented = self.last_market_frame.as_ref().map_or_else(
                || model.clone(),
                |frozen| model.clone().with_market_presentation_from(frozen),
            );
            if self.last_market_frame.is_none() {
                self.last_market_frame = Some(model.clone());
            }
            presented
        } else {
            self.last_market_frame = Some(model.clone());
            model.clone()
        }
    }
}

struct LiveTuiRenderer<W: Write = io::Stderr> {
    terminal: Option<Terminal<CrosstermBackend<W>>>,
    enforcement: LiveTuiSessionEnforcement,
    last_viewport: Option<RatatuiViewport>,
    presentation: LiveTuiPresentationCache,
    alert_runtime: Option<LiveAlertRuntime>,
}

impl LiveTuiRenderer<io::Stderr> {
    fn new(enforcement: LiveTuiSessionEnforcement) -> anyhow::Result<Self> {
        let terminal = TERMINAL_OPERATION_COORDINATOR
            .with_session_operation(enforcement, || -> anyhow::Result<_> {
                let stderr = io::stderr();
                let backend = CrosstermBackend::new(stderr);
                let mut terminal = Terminal::new(backend)?;
                ratatui::backend::Backend::clear(terminal.backend_mut())?;
                Ok(terminal)
            })
            .map_err(live_tui_session_operation_error)??;
        Ok(Self {
            terminal: Some(terminal),
            enforcement,
            last_viewport: None,
            presentation: LiveTuiPresentationCache::default(),
            alert_runtime: None,
        })
    }
}

impl<W: Write> LiveTuiRenderer<W> {
    fn with_alert_runtime(mut self, alert_runtime: Option<LiveAlertRuntime>) -> Self {
        self.alert_runtime = alert_runtime;
        self
    }

    fn alert_records(&self) -> Vec<TuiAlertRecord> {
        self.alert_runtime
            .as_ref()
            .map_or_else(Vec::new, LiveAlertRuntime::records)
    }

    #[cfg(test)]
    fn with_fixed_viewport_writer(
        writer: W,
        viewport: RatatuiViewport,
        enforcement: LiveTuiSessionEnforcement,
    ) -> anyhow::Result<Self> {
        let terminal = TERMINAL_OPERATION_COORDINATOR
            .with_session_operation(enforcement, || -> anyhow::Result<_> {
                let backend = CrosstermBackend::new(writer);
                let mut terminal = Terminal::with_options(
                    backend,
                    TerminalOptions {
                        viewport: Viewport::Fixed(Rect::new(0, 0, viewport.width, viewport.height)),
                    },
                )?;
                ratatui::backend::Backend::clear(terminal.backend_mut())?;
                Ok(terminal)
            })
            .map_err(live_tui_session_operation_error)??;
        Ok(Self {
            terminal: Some(terminal),
            enforcement,
            last_viewport: None,
            presentation: LiveTuiPresentationCache::default(),
            alert_runtime: None,
        })
    }
}

impl<W: Write> LiveTuiFrameSink for LiveTuiRenderer<W> {
    fn viewport_changed(&mut self) -> bool {
        // A resize signal can be lost during first-frame handoff on some PTYs.
        let Ok((width, height)) = terminal_size() else {
            return false;
        };
        self.last_viewport
            .is_some_and(|viewport| viewport != RatatuiViewport { width, height })
    }

    fn draw(
        &mut self,
        model: &RatatuiFrameModel,
        color_mode: RatatuiColorMode,
    ) -> anyhow::Result<()> {
        let alerts = evaluate_live_alerts(&mut self.alert_runtime, model.rows(), now_ms_i64()?)?;
        let model = self.presentation.present(model).with_alerts(alerts);
        let terminal = self
            .terminal
            .as_mut()
            .context("live TUI renderer terminal is unavailable")?;
        let completed = TERMINAL_OPERATION_COORDINATOR
            .with_session_operation(self.enforcement, || {
                apply_crossterm_color_policy(color_mode);
                terminal.draw(|frame| {
                    hls_tui::ratatui_app::render_ratatui_frame(frame, &model, color_mode);
                })
            })
            .map_err(live_tui_session_operation_error)??;
        self.last_viewport = Some(RatatuiViewport {
            width: completed.area.width,
            height: completed.area.height,
        });
        Ok(())
    }
}

fn apply_crossterm_color_policy(color_mode: RatatuiColorMode) {
    Colored::set_ansi_color_disabled(color_mode == RatatuiColorMode::NoColor);
}

impl<W: Write> Drop for LiveTuiRenderer<W> {
    fn drop(&mut self) {
        let Some(terminal) = self.terminal.take() else {
            return;
        };
        TERMINAL_OPERATION_COORDINATOR.with_operation(|_| drop(terminal));
    }
}

fn live_tui_session_operation_error(state: LiveTuiSessionState) -> anyhow::Error {
    anyhow::anyhow!(
        "live TUI terminal session is {}; refusing terminal output after panic interruption",
        state.label()
    )
}

fn live_table_title(recording_active: bool) -> &'static str {
    if recording_active {
        "RECORDING Hyperliquid spot live screen"
    } else {
        "READ-ONLY Hyperliquid spot live screen"
    }
}

fn shutdown_listener_setup<T>(result: io::Result<T>) -> anyhow::Result<T> {
    result.context("install shutdown signal listener")
}

#[cfg(any(test, not(any(unix, windows))))]
fn shutdown_signal_stop_reason(result: io::Result<()>) -> anyhow::Result<LiveStopReason> {
    result
        .context("wait for OS shutdown signal")
        .map(|()| LiveStopReason::Signal)
}

#[cfg(unix)]
fn unix_signal_stop_reason(signal: &str, delivery: Option<()>) -> anyhow::Result<LiveStopReason> {
    delivery
        .with_context(|| format!("{signal} shutdown listener closed before receiving a signal"))
        .map(|()| LiveStopReason::Signal)
}

#[cfg(unix)]
fn sigterm_stop_reason(delivery: Option<()>) -> anyhow::Result<LiveStopReason> {
    unix_signal_stop_reason("SIGTERM", delivery)
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

        let signal: ShutdownSignal = Box::pin(async move {
            tokio::select! {
                delivery = interrupt.recv() => unix_signal_stop_reason("SIGINT", delivery),
                delivery = terminate.recv() => sigterm_stop_reason(delivery),
            }
        });
        Ok(signal)
    }

    #[cfg(windows)]
    {
        let mut interrupt = shutdown_listener_setup(tokio::signal::windows::ctrl_c())?;
        let signal: ShutdownSignal = Box::pin(async move {
            interrupt
                .recv()
                .await
                .context("CTRL_C shutdown listener closed before receiving a signal")
                .map(|()| LiveStopReason::Signal)
        });
        Ok(signal)
    }

    #[cfg(not(any(unix, windows)))]
    {
        Ok(Box::pin(async {
            shutdown_signal_stop_reason(tokio::signal::ctrl_c().await)
        }))
    }
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

fn live_interval(period: Duration) -> Interval {
    let mut timer = interval(period);
    timer.set_missed_tick_behavior(MissedTickBehavior::Skip);
    timer
}

fn websocket_inactive(last_market_data: Instant, now: Instant) -> bool {
    now.saturating_duration_since(last_market_data) >= WS_MARKET_DATA_TIMEOUT
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

    #[tokio::test]
    async fn live_runtime_intervals_skip_missed_ticks() {
        let timer = live_interval(Duration::from_millis(100));
        assert_eq!(
            timer.missed_tick_behavior(),
            tokio::time::MissedTickBehavior::Skip
        );
    }

    #[test]
    fn websocket_inactivity_uses_a_monotonic_deadline() {
        let last_market_data = Instant::now();
        assert!(!websocket_inactive(
            last_market_data,
            last_market_data + WS_MARKET_DATA_TIMEOUT - Duration::from_millis(1)
        ));
        assert!(websocket_inactive(
            last_market_data,
            last_market_data + WS_MARKET_DATA_TIMEOUT
        ));
    }

    #[test]
    fn subscriptions_and_control_frames_share_one_outbound_rate_window() {
        let started = tokio::time::Instant::now();
        let mut limiter = RollingMessageRateLimiter::default();

        for _ in 0..924 {
            assert_eq!(limiter.next_available_at(started), None);
            limiter.record(started);
        }
        let second_batch = started + Duration::from_secs(1);
        for _ in 0..924 {
            assert_eq!(limiter.next_available_at(second_batch), None);
            limiter.record(second_batch);
        }
        let third_batch = started + Duration::from_secs(2);
        for _ in 0..50 {
            assert_eq!(limiter.next_available_at(third_batch), None);
            limiter.record(third_batch);
        }
        // Heartbeats and protocol pongs use this same limiter as subscriptions.
        for _ in 0..2 {
            assert_eq!(limiter.next_available_at(third_batch), None);
            limiter.record(third_batch);
        }

        assert_eq!(
            limiter.next_available_at(third_batch),
            Some(started + crate::commands::ws_rate_limit::WS_OUTBOUND_RATE_WINDOW)
        );
        assert_eq!(
            limiter.next_available_at(
                started + crate::commands::ws_rate_limit::WS_OUTBOUND_RATE_WINDOW
            ),
            None
        );
    }

    #[tokio::test(start_paused = true)]
    async fn thirtieth_live_connection_waits_for_the_rolling_window() {
        let started = tokio::time::Instant::now();
        let mut limiter = RollingRateLimiter::new(
            LIVE_CONNECTION_RATE_BUDGET,
            crate::commands::ws_rate_limit::WS_OUTBOUND_RATE_WINDOW,
        );
        for _ in 0..LIVE_CONNECTION_RATE_BUDGET {
            limiter.record(started);
        }

        let wait = wait_for_live_connection_slot(
            &mut limiter,
            pending::<anyhow::Result<LiveStopReason>>(),
        );
        tokio::pin!(wait);
        tokio::select! {
            result = &mut wait => panic!("30th attempt released early: {result:?}"),
            _ = sleep(Duration::from_secs(59)) => {}
        }
        assert_eq!(
            wait.await.expect("rate wait succeeds"),
            CancellableSendOutcome::Sent(())
        );
        assert_eq!(
            tokio::time::Instant::now(),
            started + Duration::from_secs(60)
        );
    }

    #[tokio::test(start_paused = true)]
    async fn live_connection_rate_wait_is_cancelled_by_stop() {
        let started = tokio::time::Instant::now();
        let mut limiter = RollingRateLimiter::new(1, Duration::from_secs(60));
        limiter.record(started);

        assert_eq!(
            wait_for_live_connection_slot(
                &mut limiter,
                std::future::ready(Ok(LiveStopReason::Signal)),
            )
            .await
            .expect("stop succeeds"),
            CancellableSendOutcome::Stopped(LiveStopReason::Signal)
        );
        assert_eq!(tokio::time::Instant::now(), started);
    }

    #[tokio::test(start_paused = true)]
    async fn ready_stop_beats_an_immediately_available_live_connection_slot() {
        let started = tokio::time::Instant::now();
        let mut limiter = RollingRateLimiter::new(1, Duration::from_secs(60));

        assert_eq!(
            wait_for_live_connection_slot(
                &mut limiter,
                std::future::ready(Ok(LiveStopReason::Signal)),
            )
            .await
            .expect("stop succeeds"),
            CancellableSendOutcome::Stopped(LiveStopReason::Signal)
        );
        assert_eq!(limiter.next_available_at(started), None);
        limiter.record(started);
        assert_eq!(
            limiter.next_available_at(started),
            Some(started + Duration::from_secs(60))
        );
    }

    #[test]
    fn stopped_fixture_backfill_does_not_continue_to_post_processing() {
        assert!(!fixture_continues_after_backfill(
            backfill::BackfillRunOutcome::Stopped
        ));
        assert!(fixture_continues_after_backfill(
            backfill::BackfillRunOutcome::Completed
        ));
    }

    #[test]
    fn all_symbol_subscription_plan_degrades_to_global_mids_and_contexts() {
        let symbols = (0..700).map(|index| format!("@{index}")).collect();
        let plan = live_subscription_plan(symbols, true, DEFAULT_MAX_SUBSCRIPTIONS);

        assert_eq!(plan.subscription_count(), 701);
        assert_eq!(plan.per_symbol_stream_count(), 1);
        assert_eq!(plan.global_stream_count(), 1);
        assert!(plan.validate().is_ok());
    }

    #[test]
    fn current_size_all_symbol_plan_keeps_trades_and_bbo_under_headroom() {
        let symbols = (0..309).map(|index| format!("@{index}")).collect();
        let plan = live_subscription_plan(symbols, true, DEFAULT_MAX_SUBSCRIPTIONS);

        assert_eq!(plan.subscription_count(), 928);
        assert_eq!(plan.per_symbol_stream_count(), 3);
        assert_eq!(plan.global_stream_count(), 1);
        assert!(plan.validate().is_ok());
    }

    #[test]
    fn oversized_all_symbol_plan_falls_back_to_one_global_feed() {
        let symbols = (0..1_200).map(|index| format!("@{index}")).collect();
        let plan = live_subscription_plan(symbols, true, DEFAULT_MAX_SUBSCRIPTIONS);

        assert_eq!(plan.subscription_count(), 1);
        assert_eq!(plan.per_symbol_stream_count(), 0);
        assert_eq!(plan.global_stream_count(), 1);
        assert!(plan.validate().is_ok());
    }

    #[test]
    fn live_tui_diagnostics_do_not_bypass_frame_sink() {
        let mut diagnostics = LiveDiagnostics::default();

        assert_eq!(
            diagnostics.route(false, "non-TUI warning"),
            Some("non-TUI warning".to_owned())
        );
        assert_eq!(diagnostics.route(true, "TUI reconnect warning"), None);
        assert_eq!(
            diagnostics.take_deferred(),
            vec!["TUI reconnect warning".to_owned()]
        );

        for index in 0..(MAX_DEFERRED_LIVE_DIAGNOSTICS + 2) {
            assert_eq!(diagnostics.route(true, format!("warning-{index}")), None);
        }
        let deferred = diagnostics.take_deferred();
        assert_eq!(deferred.len(), MAX_DEFERRED_LIVE_DIAGNOSTICS);
        assert_eq!(deferred.first().map(String::as_str), Some("warning-2"));
    }

    #[test]
    fn live_closeout_orders_terminal_teardown_before_blocking_work() {
        let order = std::cell::RefCell::new(Vec::new());

        let (recording, preferences) = after_live_tui_teardown(
            || {
                order
                    .borrow_mut()
                    .extend(["renderer drop", "guard drop", "deferred diagnostics"]);
            },
            || {
                order.borrow_mut().push("recorder finish");
                "recording complete"
            },
            || {
                order.borrow_mut().push("preferences save");
                "preferences complete"
            },
        );

        assert_eq!(recording, "recording complete");
        assert_eq!(preferences, "preferences complete");
        assert_eq!(
            order.into_inner(),
            vec![
                "renderer drop",
                "guard drop",
                "deferred diagnostics",
                "recorder finish",
                "preferences save",
            ]
        );
    }

    #[test]
    fn other_thread_hook_blocks_until_activation_operation_finishes() {
        let coordinator = std::sync::Arc::new(TerminalOperationCoordinator::new());
        let (events_tx, events_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let hook_barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
        let (hook_done_tx, hook_done_rx) = mpsc::channel();

        let activation_coordinator = std::sync::Arc::clone(&coordinator);
        let activation_events = events_tx.clone();
        let activation = thread::spawn(move || {
            activation_coordinator.with_operation(|reentrant| {
                assert!(!reentrant);
                activation_coordinator
                    .begin_activation()
                    .expect("activation begins");
                activation_events
                    .send("activation start")
                    .expect("activation start sent");
                release_rx.recv().expect("activation released");
                activation_events
                    .send("activation finish")
                    .expect("activation finish sent");
                activation_coordinator
                    .publish_active()
                    .expect("activation publishes active");
            });
        });

        assert_eq!(
            events_rx.recv().expect("activation starts"),
            "activation start"
        );
        let hook_coordinator = std::sync::Arc::clone(&coordinator);
        let hook_events = events_tx.clone();
        let hook_ready = std::sync::Arc::clone(&hook_barrier);
        let hook = thread::spawn(move || {
            hook_ready.wait();
            hook_coordinator.handle_panic(
                || hook_events.send("restore").expect("restore sent"),
                || hook_events.send("panic output").expect("panic output sent"),
            );
            hook_done_tx.send(()).expect("hook completion sent");
        });
        hook_barrier.wait();

        assert!(matches!(
            hook_done_rx.try_recv(),
            Err(mpsc::TryRecvError::Empty)
        ));
        assert!(matches!(
            events_rx.try_recv(),
            Err(mpsc::TryRecvError::Empty)
        ));

        release_tx.send(()).expect("release activation");
        activation.join().expect("activation thread joins");
        hook.join().expect("hook thread joins");
        assert_eq!(
            events_rx.recv().expect("activation finishes"),
            "activation finish"
        );
        assert_eq!(events_rx.recv().expect("restore follows"), "restore");
        assert_eq!(
            events_rx.recv().expect("panic output follows"),
            "panic output"
        );
        assert_eq!(coordinator.state(), LiveTuiSessionState::Interrupted);
    }

    #[test]
    fn other_thread_hook_cannot_interleave_active_draw() {
        let coordinator = std::sync::Arc::new(TerminalOperationCoordinator::new());
        coordinator.with_operation(|_| {
            coordinator.begin_activation().expect("activation begins");
            coordinator.publish_active().expect("activation completes");
        });
        let (events_tx, events_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let hook_barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
        let (hook_done_tx, hook_done_rx) = mpsc::channel();

        let draw_coordinator = std::sync::Arc::clone(&coordinator);
        let draw_events = events_tx.clone();
        let draw = thread::spawn(move || {
            draw_coordinator
                .with_session_operation(LiveTuiSessionEnforcement::Interactive, || {
                    draw_events.send("draw start").expect("draw start sent");
                    release_rx.recv().expect("draw released");
                    draw_events.send("draw finish").expect("draw finish sent");
                })
                .expect("draw is serialized");
        });

        assert_eq!(events_rx.recv().expect("draw starts"), "draw start");
        let hook_coordinator = std::sync::Arc::clone(&coordinator);
        let hook_events = events_tx.clone();
        let hook_ready = std::sync::Arc::clone(&hook_barrier);
        let hook = thread::spawn(move || {
            hook_ready.wait();
            hook_coordinator.handle_panic(
                || hook_events.send("restore").expect("restore sent"),
                || hook_events.send("panic output").expect("panic output sent"),
            );
            hook_done_tx.send(()).expect("hook completion sent");
        });
        hook_barrier.wait();

        assert!(matches!(
            hook_done_rx.try_recv(),
            Err(mpsc::TryRecvError::Empty)
        ));
        assert!(matches!(
            events_rx.try_recv(),
            Err(mpsc::TryRecvError::Empty)
        ));

        release_tx.send(()).expect("release draw");
        draw.join().expect("draw thread joins");
        hook.join().expect("hook thread joins");
        assert_eq!(events_rx.recv().expect("draw finishes"), "draw finish");
        assert_eq!(events_rx.recv().expect("restore follows"), "restore");
        assert_eq!(
            events_rx.recv().expect("panic output follows"),
            "panic output"
        );
        assert_eq!(coordinator.state(), LiveTuiSessionState::Interrupted);
    }

    #[test]
    fn interrupted_session_rejects_subsequent_draw_operation() {
        let coordinator = TerminalOperationCoordinator::new();
        coordinator.with_operation(|_| {
            coordinator.begin_activation().expect("activation begins");
            coordinator.publish_active().expect("activation completes");
        });
        coordinator.handle_panic(|| {}, || {});
        let wrote = std::cell::Cell::new(false);

        let result = coordinator
            .with_session_operation(LiveTuiSessionEnforcement::Interactive, || wrote.set(true));

        assert_eq!(result, Err(LiveTuiSessionState::Interrupted));
        assert!(!wrote.get());
        assert!(!coordinator.finish_session(|| panic!("interrupted session restored twice")));
        assert_eq!(coordinator.state(), LiveTuiSessionState::Inactive);
    }

    #[test]
    fn same_thread_panic_path_reenters_coordinator_without_deadlock() {
        let coordinator = TerminalOperationCoordinator::new();
        let order = std::cell::RefCell::new(Vec::new());

        coordinator.with_operation(|outer_reentrant| {
            assert!(!outer_reentrant);
            coordinator.begin_activation().expect("activation begins");
            coordinator.handle_panic(
                || order.borrow_mut().push("restore"),
                || order.borrow_mut().push("panic output"),
            );
            order.borrow_mut().push("returned");
        });

        assert_eq!(
            order.into_inner(),
            vec!["restore", "panic output", "returned"]
        );
        assert_eq!(coordinator.state(), LiveTuiSessionState::Inactive);
    }

    #[test]
    fn sequential_interactive_sessions_restore_and_reactivate() {
        let coordinator = TerminalOperationCoordinator::new();
        let restore_count = std::cell::Cell::new(0);

        for _ in 0..2 {
            coordinator.with_operation(|_| {
                coordinator.begin_activation().expect("activation begins");
                coordinator.publish_active().expect("activation completes");
            });
            assert!(coordinator.finish_session(|| {
                restore_count.set(restore_count.get() + 1);
            }));
            assert_eq!(coordinator.state(), LiveTuiSessionState::Inactive);
        }

        assert_eq!(restore_count.get(), 2);
    }

    #[test]
    fn unmanaged_renderer_operation_works_without_interactive_session() {
        let coordinator = TerminalOperationCoordinator::new();
        let rendered = std::cell::Cell::new(false);

        coordinator
            .with_session_operation(LiveTuiSessionEnforcement::Unmanaged, || rendered.set(true))
            .expect("unmanaged render is allowed while inactive");

        assert!(rendered.get());
        assert_eq!(coordinator.state(), LiveTuiSessionState::Inactive);
    }

    #[test]
    fn unbounded_tui_duration_requires_interactive_stdio() {
        let err =
            validate_live_duration(0, true, false, LiveTerminalCapabilities::new(false, true))
                .expect_err("unbounded TUI needs interactive stdio");

        assert!(
            err.to_string()
                .contains("both stdin and stderr attached to a terminal")
        );
        assert!(
            validate_live_duration(0, true, false, LiveTerminalCapabilities::new(true, true),)
                .is_ok()
        );
    }

    #[test]
    fn fixture_once_skips_unbounded_tty_validation() {
        let noninteractive = LiveTerminalCapabilities::new(false, false);

        assert!(validate_live_duration(0, true, true, noninteractive).is_ok());
        assert!(validate_live_duration(0, false, true, noninteractive).is_ok());
        assert!(validate_live_duration(0, true, false, noninteractive).is_err());
    }

    #[test]
    fn live_run_lifetime_zero_is_unbounded() {
        let now = tokio::time::Instant::now();

        assert_eq!(
            LiveRunLifetime::from_duration_secs(0, now).expect("unbounded lifetime"),
            LiveRunLifetime::Unbounded
        );
    }

    #[test]
    fn live_run_lifetime_positive_duration_is_bounded() {
        let now = tokio::time::Instant::now();

        assert_eq!(
            LiveRunLifetime::from_duration_secs(15, now).expect("bounded lifetime"),
            LiveRunLifetime::Bounded(now + Duration::from_secs(15))
        );
    }

    #[test]
    fn live_run_lifetime_overflow_returns_an_error() {
        let err = LiveRunLifetime::from_duration_secs(u64::MAX, tokio::time::Instant::now())
            .expect_err("overflowing duration must fail without panic");

        assert!(err.to_string().contains("too large"));
    }

    #[test]
    fn live_run_lifetime_only_bounded_lifetimes_expire() {
        let now = tokio::time::Instant::now();
        let bounded = LiveRunLifetime::from_duration_secs(15, now).expect("bounded lifetime");
        let unbounded = LiveRunLifetime::from_duration_secs(0, now).expect("unbounded lifetime");

        assert!(bounded.has_expired_by(now + Duration::from_secs(15)));
        assert!(!unbounded.has_expired_by(now + Duration::from_secs(15)));
    }

    #[test]
    fn shutdown_listener_errors_are_not_successful_signal_stops() {
        let setup_err = shutdown_listener_setup::<()>(Err(std::io::Error::other("setup failed")))
            .expect_err("listener setup failure propagates");
        assert!(
            setup_err
                .to_string()
                .contains("install shutdown signal listener")
        );

        let delivery_err =
            shutdown_signal_stop_reason(Err(std::io::Error::other("delivery failed")))
                .expect_err("signal delivery failure propagates");
        assert!(
            delivery_err
                .to_string()
                .contains("wait for OS shutdown signal")
        );
    }

    #[test]
    fn live_drive_summary_preserves_clean_stop_reasons() {
        let mut summary = LiveDriveSummary {
            reconnects: 1,
            ..LiveDriveSummary::default()
        };
        summary.mark_stopped(LiveStopReason::Signal);

        assert_eq!(summary.stop_reason, Some(LiveStopReason::Signal));
        assert_eq!(summary.stop_reason_label(), Some("signal"));
        assert!(summary.is_clean_shutdown());
        assert!(!summary.is_no_market_data_failure());
    }

    #[test]
    fn reconnect_gaps_lower_confidence_for_every_affected_symbol() {
        let mut summary = LiveDriveSummary::default();
        summary.mark_gap(&["HYPE/USDC".to_owned(), "BTC/USDC".to_owned()]);
        let state = LiveMarketState::new([
            "HYPE/USDC".to_owned(),
            "BTC/USDC".to_owned(),
            "SOL/USDC".to_owned(),
        ]);
        let snapshots = FeatureEngine::default().snapshots_with_confidence_inputs(
            &state,
            1_710_000_000_000,
            &summary.confidence_inputs(),
        );

        for symbol in ["HYPE/USDC", "BTC/USDC"] {
            let row = snapshots
                .iter()
                .find(|row| row.symbol == symbol)
                .expect("affected row");
            assert!(
                row.confidence
                    .has_reason(hls_core::confidence::ConfidenceReason::ReconnectGap)
            );
        }
        let unaffected = snapshots
            .iter()
            .find(|row| row.symbol == "SOL/USDC")
            .expect("unaffected row");
        assert!(
            !unaffected
                .confidence
                .has_reason(hls_core::confidence::ConfidenceReason::ReconnectGap)
        );
    }

    #[test]
    fn live_drive_summary_reports_truthful_recorder_state() {
        let ready = LiveDriveSummary::default();
        assert_eq!(ready.recorder_status(), "REC ready");

        let active = LiveDriveSummary {
            recording_active: true,
            ..LiveDriveSummary::default()
        };
        assert_eq!(active.recorder_status(), "REC active");
    }

    #[test]
    fn live_drive_summary_rejects_unstopped_no_message_runs() {
        let summary = LiveDriveSummary {
            reconnects: 1,
            ..LiveDriveSummary::default()
        };

        assert!(summary.is_no_market_data_failure());
        assert!(!summary.is_clean_shutdown());
    }

    #[test]
    fn duration_elapsed_outage_remains_a_run_failure() {
        let mut summary = LiveDriveSummary {
            reconnects: 1,
            ..LiveDriveSummary::default()
        };
        summary.mark_stopped(LiveStopReason::DurationElapsed);

        assert!(summary.is_clean_shutdown());
        assert!(summary.is_no_market_data_failure());
    }

    #[test]
    fn control_frames_do_not_make_a_no_market_data_run_successful() {
        let mut summary = LiveDriveSummary {
            ws_messages: 10,
            market_events: 0,
            ..LiveDriveSummary::default()
        };
        summary.mark_stopped(LiveStopReason::DurationElapsed);

        assert!(summary.is_no_market_data_failure());
    }

    #[test]
    fn operator_and_signal_outages_stop_cleanly() {
        for stop_reason in [LiveStopReason::OperatorQuit, LiveStopReason::Signal] {
            let mut summary = LiveDriveSummary {
                reconnects: 1,
                ..LiveDriveSummary::default()
            };
            summary.mark_stopped(stop_reason);

            assert!(summary.is_clean_shutdown());
            assert!(!summary.is_no_market_data_failure());
        }
    }

    #[test]
    fn operator_quit_reason_is_available_during_connection_waits() {
        let mut state = WorkstationUiState::default();
        state.apply(WorkstationAction::Quit, 1);

        assert_eq!(
            operator_stop_reason(&state),
            Some(LiveStopReason::OperatorQuit)
        );
    }

    #[test]
    fn operator_quit_reason_is_available_during_subscription_startup() {
        let mut state = WorkstationUiState::default();
        state.apply(WorkstationAction::Quit, 1);

        assert_eq!(
            operator_stop_reason(&state),
            Some(LiveStopReason::OperatorQuit)
        );
    }

    #[tokio::test]
    async fn cancellable_send_stop_wins_over_pending_write() {
        for key in [
            KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE),
            KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
            KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
        ] {
            let mut state = WorkstationUiState::default();
            let action = key_to_workstation_action(key, &state).expect("key maps to an action");
            state.apply(action, 1);
            let stop_reason = operator_stop_reason(&state).expect("key requests operator stop");
            let send = pending::<Result<(), &'static str>>();
            let stop = std::future::ready(Ok(stop_reason));

            let outcome =
                tokio::time::timeout(Duration::from_millis(100), cancellable_send(send, stop))
                    .await
                    .expect("operator stop must not wait for a pending write")
                    .expect("stop future succeeds");

            assert_eq!(
                outcome,
                CancellableSendOutcome::Stopped(LiveStopReason::OperatorQuit)
            );
        }
    }

    #[tokio::test]
    async fn cancellable_send_preserves_completed_write_result() {
        let send = std::future::ready(Err::<(), _>("write failed"));
        let stop = pending::<anyhow::Result<LiveStopReason>>();

        assert_eq!(
            cancellable_send(send, stop)
                .await
                .expect("selection succeeds"),
            CancellableSendOutcome::Sent(Err("write failed"))
        );
    }

    #[test]
    fn websocket_reply_write_failure_preserves_gap_context_for_reconnect() {
        let outcome = reconnect_after_ws_send_failure(
            7,
            10,
            Some(20),
            30,
            "send WebSocket pong",
            "connection reset",
            true,
        );

        match outcome {
            ConnectionOutcome::Reconnect {
                conn_id,
                gap_started_at_ns,
                gap_ended_at_ns,
                reason,
                received_market_data,
            } => {
                assert_eq!(conn_id, 7);
                assert_eq!(gap_started_at_ns, 20);
                assert_eq!(gap_ended_at_ns, 30);
                assert_eq!(reason, "send WebSocket pong: connection reset");
                assert!(received_market_data);
            }
            ConnectionOutcome::Stopped(reason) => {
                panic!("write failure must reconnect, not stop with {reason:?}")
            }
        }
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
                KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
                &state
            ),
            Some(WorkstationAction::Quit)
        );
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
            key_to_workstation_action(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE), &state),
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
            key_to_workstation_action(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE), &state),
            Some(WorkstationAction::PreviousPane)
        );
        let mut command_state = WorkstationUiState::default();
        command_state.apply(WorkstationAction::CycleFilter, 1);
        assert_eq!(
            key_to_workstation_action(
                KeyEvent::new(KeyCode::Right, KeyModifiers::NONE),
                &command_state
            ),
            None
        );
        assert_eq!(
            key_to_workstation_action(
                KeyEvent::new(KeyCode::Left, KeyModifiers::NONE),
                &command_state
            ),
            None
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
    fn live_tui_mouse_events_map_to_pointer_aware_actions() {
        let state = WorkstationUiState::default();
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::ScrollUp,
                    column: 10,
                    row: 11,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((160, 48)),
                20,
            ),
            Some(WorkstationAction::ScrollPane(
                WorkstationPane::Watchlist,
                WorkstationScrollDirection::Up
            ))
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::ScrollDown,
                    column: 70,
                    row: 12,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((160, 48)),
                20,
            ),
            Some(WorkstationAction::ScrollPane(
                WorkstationPane::Detail,
                WorkstationScrollDirection::Down
            ))
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::ScrollDown,
                    column: 70,
                    row: 30,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((160, 48)),
                20,
            ),
            Some(WorkstationAction::ScrollPane(
                WorkstationPane::Chart,
                WorkstationScrollDirection::Down
            ))
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
                    column: 8,
                    row: 2,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((160, 48)),
                20,
            ),
            Some(WorkstationAction::TogglePaneZoom)
        );
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
                    column: 10,
                    row: 2,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((72, 24)),
                20,
            ),
            Some(WorkstationAction::TogglePaneZoom)
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
    fn live_tui_mouse_clicks_active_top_bar_pane_as_zoom_control() {
        let state = WorkstationUiState::default();

        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 20,
                    row: 2,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((240, 56)),
                20,
            ),
            Some(WorkstationAction::TogglePaneZoom)
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 36,
                    row: 2,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((240, 56)),
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
                    column: 20,
                    row: 2,
                    modifiers: KeyModifiers::NONE,
                },
                &command_state,
                Some((240, 56)),
                20,
            ),
            None
        );
    }

    #[test]
    fn live_tui_mouse_clicks_standard_wide_selected_quote_rail() {
        let state = WorkstationUiState::default();

        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 8,
                    row: 3,
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
                    column: 52,
                    row: 3,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((160, 48)),
                20,
            ),
            Some(WorkstationAction::FocusPane(WorkstationPane::Book))
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 126,
                    row: 3,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((160, 48)),
                20,
            ),
            Some(WorkstationAction::FocusPane(WorkstationPane::Tape))
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 12,
                    row: 4,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((160, 48)),
                20,
            ),
            Some(WorkstationAction::TogglePaneZoom)
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 22,
                    row: 4,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((160, 48)),
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
                    column: 52,
                    row: 3,
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
    fn live_tui_mouse_clicks_market_internals_rail() {
        let state = WorkstationUiState::default();

        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 18,
                    row: 5,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((160, 48)),
                20,
            ),
            Some(WorkstationAction::TogglePaneZoom)
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 78,
                    row: 5,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((160, 48)),
                20,
            ),
            Some(WorkstationAction::FocusPane(WorkstationPane::Status))
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 112,
                    row: 5,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((160, 48)),
                20,
            ),
            Some(WorkstationAction::FocusPane(WorkstationPane::Tape))
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 136,
                    row: 5,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((160, 48)),
                20,
            ),
            Some(WorkstationAction::FocusPane(WorkstationPane::Book))
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 12,
                    row: 3,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((72, 24)),
                20,
            ),
            Some(WorkstationAction::TogglePaneZoom)
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 45,
                    row: 3,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((72, 24)),
                20,
            ),
            Some(WorkstationAction::FocusPane(WorkstationPane::Status))
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 59,
                    row: 3,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((72, 24)),
                20,
            ),
            Some(WorkstationAction::FocusPane(WorkstationPane::Tape))
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 68,
                    row: 3,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((72, 24)),
                20,
            ),
            Some(WorkstationAction::FocusPane(WorkstationPane::Book))
        );

        let mut command_state = WorkstationUiState::default();
        command_state.apply(WorkstationAction::CycleFilter, 1);
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 112,
                    row: 5,
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
    fn live_tui_mouse_clicks_visible_command_controls() {
        let state = WorkstationUiState::default();

        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 91,
                    row: 2,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((240, 56)),
                20,
            ),
            Some(WorkstationAction::OpenSymbolSearch)
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 101,
                    row: 2,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((240, 56)),
                20,
            ),
            Some(WorkstationAction::CycleFilter)
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 111,
                    row: 2,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((240, 56)),
                20,
            ),
            Some(WorkstationAction::CyclePreset)
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 121,
                    row: 2,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((240, 56)),
                20,
            ),
            Some(WorkstationAction::CycleSort)
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 129,
                    row: 2,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((240, 56)),
                20,
            ),
            Some(WorkstationAction::CycleChartWindow)
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 140,
                    row: 2,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((240, 56)),
                20,
            ),
            Some(WorkstationAction::ToggleDensity)
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 156,
                    row: 2,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((240, 56)),
                20,
            ),
            Some(WorkstationAction::TogglePaneZoom)
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 164,
                    row: 2,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((240, 56)),
                20,
            ),
            Some(WorkstationAction::TogglePause)
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 173,
                    row: 2,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((240, 56)),
                20,
            ),
            Some(WorkstationAction::ToggleHelp)
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 181,
                    row: 2,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((240, 56)),
                20,
            ),
            Some(WorkstationAction::Quit)
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 34,
                    row: 2,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((80, 16)),
                20,
            ),
            Some(WorkstationAction::TogglePaneZoom)
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 37,
                    row: 2,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((80, 16)),
                20,
            ),
            Some(WorkstationAction::FocusPane(WorkstationPane::Detail))
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 40,
                    row: 2,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((80, 16)),
                20,
            ),
            Some(WorkstationAction::FocusPane(WorkstationPane::Chart))
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 49,
                    row: 2,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((80, 16)),
                20,
            ),
            Some(WorkstationAction::FocusPane(WorkstationPane::Status))
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 5,
                    row: 2,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((80, 16)),
                20,
            ),
            Some(WorkstationAction::OpenSymbolSearch)
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 7,
                    row: 2,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((80, 16)),
                20,
            ),
            Some(WorkstationAction::CycleFilter)
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 13,
                    row: 2,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((80, 16)),
                20,
            ),
            Some(WorkstationAction::CycleChartWindow)
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 17,
                    row: 2,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((80, 16)),
                20,
            ),
            Some(WorkstationAction::TogglePaneZoom)
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 19,
                    row: 2,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((80, 16)),
                20,
            ),
            Some(WorkstationAction::TogglePause)
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 24,
                    row: 2,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((80, 16)),
                20,
            ),
            Some(WorkstationAction::Quit)
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 10,
                    row: 2,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((120, 40)),
                20,
            ),
            Some(WorkstationAction::OpenSymbolSearch)
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 12,
                    row: 2,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((120, 40)),
                20,
            ),
            Some(WorkstationAction::CycleFilter)
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 20,
                    row: 2,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((120, 40)),
                20,
            ),
            Some(WorkstationAction::ToggleDensity)
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 24,
                    row: 2,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((120, 40)),
                20,
            ),
            Some(WorkstationAction::TogglePause)
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 29,
                    row: 2,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((120, 40)),
                20,
            ),
            Some(WorkstationAction::Quit)
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 63,
                    row: 2,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((180, 48)),
                20,
            ),
            Some(WorkstationAction::OpenSymbolSearch)
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 65,
                    row: 2,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((180, 48)),
                20,
            ),
            Some(WorkstationAction::CycleFilter)
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 75,
                    row: 2,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((180, 48)),
                20,
            ),
            Some(WorkstationAction::TogglePaneZoom)
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 80,
                    row: 2,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((180, 48)),
                20,
            ),
            Some(WorkstationAction::ToggleHelp)
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 54,
                    row: 2,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((72, 24)),
                20,
            ),
            Some(WorkstationAction::CycleFilter)
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 55,
                    row: 2,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((72, 24)),
                20,
            ),
            Some(WorkstationAction::CyclePreset)
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 56,
                    row: 2,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((72, 24)),
                20,
            ),
            Some(WorkstationAction::CycleSort)
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 57,
                    row: 2,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((72, 24)),
                20,
            ),
            Some(WorkstationAction::CycleChartWindow)
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 58,
                    row: 2,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((72, 24)),
                20,
            ),
            Some(WorkstationAction::ToggleDensity)
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 59,
                    row: 2,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((72, 24)),
                20,
            ),
            Some(WorkstationAction::TogglePaneZoom)
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 60,
                    row: 2,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((72, 24)),
                20,
            ),
            Some(WorkstationAction::TogglePause)
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 61,
                    row: 2,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((72, 24)),
                20,
            ),
            Some(WorkstationAction::TogglePause)
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 63,
                    row: 2,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((72, 24)),
                20,
            ),
            Some(WorkstationAction::FocusPane(WorkstationPane::Status))
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 64,
                    row: 2,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((72, 24)),
                20,
            ),
            Some(WorkstationAction::ToggleHelp)
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 66,
                    row: 2,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((72, 24)),
                20,
            ),
            Some(WorkstationAction::Quit)
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 41,
                    row: 47,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((160, 48)),
                20,
            ),
            Some(WorkstationAction::OpenSymbolSearch)
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 50,
                    row: 47,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((160, 48)),
                20,
            ),
            Some(WorkstationAction::TogglePaneZoom)
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 57,
                    row: 47,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((160, 48)),
                20,
            ),
            Some(WorkstationAction::ToggleDensity)
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 67,
                    row: 47,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((160, 48)),
                20,
            ),
            Some(WorkstationAction::TogglePause)
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 79,
                    row: 47,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((160, 48)),
                20,
            ),
            Some(WorkstationAction::CycleFilter)
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 104,
                    row: 47,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((160, 48)),
                20,
            ),
            Some(WorkstationAction::CycleChartWindow)
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 117,
                    row: 47,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((160, 48)),
                20,
            ),
            Some(WorkstationAction::Quit)
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 25,
                    row: 31,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((100, 32)),
                20,
            ),
            Some(WorkstationAction::OpenSymbolSearch)
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 34,
                    row: 31,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((100, 32)),
                20,
            ),
            Some(WorkstationAction::ToggleDensity)
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 36,
                    row: 31,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((100, 32)),
                20,
            ),
            Some(WorkstationAction::TogglePause)
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 39,
                    row: 31,
                    modifiers: KeyModifiers::NONE,
                },
                &state,
                Some((100, 32)),
                20,
            ),
            Some(WorkstationAction::CycleFilter)
        );

        let mut command_state = WorkstationUiState::default();
        command_state.apply(WorkstationAction::CycleFilter, 1);
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 98,
                    row: 2,
                    modifiers: KeyModifiers::NONE,
                },
                &command_state,
                Some((240, 56)),
                20,
            ),
            None
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 41,
                    row: 47,
                    modifiers: KeyModifiers::NONE,
                },
                &command_state,
                Some((160, 48)),
                20,
            ),
            None
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 5,
                    row: 2,
                    modifiers: KeyModifiers::NONE,
                },
                &command_state,
                Some((80, 16)),
                20,
            ),
            None
        );
        assert_eq!(
            mouse_to_workstation_action(
                MouseEvent {
                    kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
                    column: 37,
                    row: 2,
                    modifiers: KeyModifiers::NONE,
                },
                &command_state,
                Some((80, 16)),
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
                    column: 70,
                    row: 30,
                    modifiers: KeyModifiers::NONE,
                }),
                &state,
                Some((160, 48)),
                20,
            ),
            LiveTuiEventEffect::Action(WorkstationAction::ScrollPane(
                WorkstationPane::Chart,
                WorkstationScrollDirection::Down
            ))
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

    #[derive(Default)]
    struct CountingTuiFrameSink {
        draws: usize,
        color_modes: Vec<RatatuiColorMode>,
        viewport_change_pending: bool,
    }

    impl LiveTuiFrameSink for CountingTuiFrameSink {
        fn viewport_changed(&mut self) -> bool {
            std::mem::take(&mut self.viewport_change_pending)
        }

        fn draw(
            &mut self,
            _model: &RatatuiFrameModel,
            color_mode: RatatuiColorMode,
        ) -> anyhow::Result<()> {
            self.draws += 1;
            self.color_modes.push(color_mode);
            Ok(())
        }
    }

    #[test]
    fn live_progress_reuses_supplied_tui_frame_sink() {
        let state = LiveMarketState::new(vec!["HYPE/USDC".to_owned()]);
        let screen_request = ScreenRequest::default();
        let summary = LiveDriveSummary {
            ws_messages: 12,
            market_events: 34,
            ..LiveDriveSummary::default()
        };
        let mut sink = CountingTuiFrameSink::default();

        for _ in 0..2 {
            render_live_progress(
                LiveProgressContext {
                    state: &state,
                    screen_request: &screen_request,
                    metadata: &[],
                    color_mode: RatatuiColorMode::Color,
                    tui_state: None,
                    started: Instant::now(),
                    summary: &summary,
                    mode: LiveProgressMode::Live,
                },
                Some(&mut sink),
            )
            .expect("progress render through persistent sink");
        }

        assert_eq!(sink.draws, 2);
        assert_eq!(
            sink.color_modes,
            vec![RatatuiColorMode::Color, RatatuiColorMode::Color]
        );
    }

    #[test]
    fn viewport_change_requests_one_redraw_without_a_terminal_event() {
        let mut sink = CountingTuiFrameSink {
            viewport_change_pending: true,
            ..CountingTuiFrameSink::default()
        };

        assert!(live_tui_redraw_requested(false, Some(&mut sink)));
        assert!(!live_tui_redraw_requested(false, Some(&mut sink)));
    }

    fn count_full_screen_clears(output: &[u8]) -> usize {
        output
            .windows(b"\x1b[2J".len())
            .filter(|window| *window == b"\x1b[2J")
            .count()
    }

    #[derive(Clone, Default)]
    struct CapturedWriter(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);

    impl io::Write for CapturedWriter {
        fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
            self.0
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .extend_from_slice(buffer);
            Ok(buffer.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl CapturedWriter {
        fn output(&self) -> Vec<u8> {
            self.0
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .clone()
        }
    }

    #[test]
    fn live_tui_renderer_reuses_terminal_and_diffs_identical_frames() {
        let model = live_tui_model(
            &[],
            "fixture renderer regression",
            &ScreenRequest::default(),
            None,
            vec![],
            vec![],
            LiveTuiStatus::new("fixture", "REC ready", "static fixture"),
        );
        let capture = CapturedWriter::default();
        let mut renderer = LiveTuiRenderer::with_fixed_viewport_writer(
            capture.clone(),
            RatatuiViewport {
                width: 120,
                height: 40,
            },
            LiveTuiSessionEnforcement::Unmanaged,
        )
        .expect("fixed viewport renderer");

        let constructor_output = capture.output();
        assert_eq!(count_full_screen_clears(&constructor_output), 1);
        let constructor_end = constructor_output.len();

        renderer
            .draw(&model, RatatuiColorMode::Color)
            .expect("first frame draws");
        let first_output = capture.output();
        let first_frame_end = first_output.len();
        assert!(
            first_output[constructor_end..first_frame_end]
                .windows(b"WATCHLIST".len())
                .any(|window| window == b"WATCHLIST")
        );

        renderer
            .draw(&model, RatatuiColorMode::Color)
            .expect("identical frame draws through the same terminal");
        let output = capture.output();
        let repeated_frame = &output[first_frame_end..];

        assert_eq!(count_full_screen_clears(&output), 1);
        assert_eq!(count_full_screen_clears(repeated_frame), 0);
        assert!(
            !repeated_frame
                .windows(b"WATCHLIST".len())
                .any(|window| window == b"WATCHLIST"),
            "Ratatui should diff an identical second frame instead of repainting it"
        );
        assert!(repeated_frame.len() < first_frame_end - constructor_end);
    }

    #[test]
    fn paused_live_tui_freezes_market_data_but_keeps_runtime_status_current() {
        let mut rows = FeatureEngine::default().snapshots(
            &LiveMarketState::new(["HYPE/USDC".to_owned()]),
            1_710_000_000_000,
        );
        let row = rows.first_mut().expect("fixture row");
        row.price = Some(35.0);
        row.mid_px = Some(35.0);

        let request = ScreenRequest::default();
        let live_model = live_tui_model(
            &rows,
            "pause regression",
            &request,
            None,
            vec![],
            vec![],
            LiveTuiStatus::new("LIVE", "REC ready", "ws=1 events=1 reconnects=0 gaps=0"),
        );
        let mut cache = LiveTuiPresentationCache::default();
        let initial = cache.present(&live_model);

        rows[0].price = Some(99.0);
        rows[0].mid_px = Some(99.0);
        let mut paused_state = WorkstationUiState::default();
        paused_state.apply(WorkstationAction::TogglePause, 1);
        let paused_model = live_tui_model(
            &rows,
            "pause regression",
            &request,
            Some(&paused_state),
            vec![],
            vec![],
            LiveTuiStatus::new(
                "LIVE",
                "REC active",
                "ws=222 events=333 reconnects=0 gaps=0",
            ),
        );
        let paused = cache.present(&paused_model);

        let viewport = RatatuiViewport {
            width: 160,
            height: 48,
        };
        let initial_output =
            render_live_tui_snapshot(&initial, Some(viewport), RatatuiColorMode::NoColor)
                .expect("initial frame renders");
        let paused_output =
            render_live_tui_snapshot(&paused, Some(viewport), RatatuiColorMode::NoColor)
                .expect("paused frame renders");

        assert!(initial_output.contains("35."), "{initial_output}");
        assert!(paused_output.contains("35."), "{paused_output}");
        assert!(!paused_output.contains("99."), "{paused_output}");
        assert!(paused_output.contains("paused"), "{paused_output}");
        assert!(paused_output.contains("REC active"), "{paused_output}");
        assert!(paused_output.contains("222"), "{paused_output}");

        paused_state.apply(WorkstationAction::TogglePause, 1);
        let resumed_model = live_tui_model(
            &rows,
            "pause regression",
            &request,
            Some(&paused_state),
            vec![],
            vec![],
            LiveTuiStatus::new(
                "LIVE",
                "REC active",
                "ws=223 events=334 reconnects=0 gaps=0",
            ),
        );
        let resumed = cache.present(&resumed_model);
        let resumed_output =
            render_live_tui_snapshot(&resumed, Some(viewport), RatatuiColorMode::NoColor)
                .expect("resumed frame renders");
        assert!(resumed_output.contains("99."), "{resumed_output}");
    }

    #[test]
    fn live_tui_presentation_payload_is_bounded_per_symbol() {
        let mut state = LiveMarketState::new(["@107".to_owned()]);
        for index in 0..100_u64 {
            state
                .apply(MarketEvent::Trade(TradeEvent {
                    recv_ts_ns: index * 1_000_000,
                    exchange_ts_ms: index as i64,
                    hl_coin: "@107".to_owned(),
                    side: hls_core::market_state::TradeSide::Buy,
                    price: 35.0,
                    size: 1.0,
                    notional: 35.0,
                    hash: format!("0x{index:x}"),
                    tid: index,
                    unique_trade_id: format!("@107:{index}:{index}"),
                }))
                .expect("trade applies");
        }

        let trades = live_tui_trades(&state);
        assert_eq!(trades.len(), MAX_TUI_EVENTS_PER_SYMBOL);
        assert_eq!(trades.first().map(|trade| trade.tid), Some(36));
        assert_eq!(trades.last().map(|trade| trade.tid), Some(99));
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
    fn live_tui_alerts_require_new_observable_market_evidence() {
        let mut playbook: AlertPlaybook = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/microstructure/alert_playbook_tui_watch.json"
        ))
        .expect("alert playbook parses");
        playbook.rules[0].cooldown_ms = 0;
        let events = parse_ws_ndjson(include_str!(
            "../../../../tests/fixtures/microstructure/resilience_shock.ndjson"
        ))
        .expect("fixture parses");
        let mut state = LiveMarketState::new(["@107".to_owned()]);
        for event in events {
            state.apply(event).expect("fixture event applies");
        }
        let mut snapshots = FeatureEngine::default().snapshots(&state, latest_update_ms(&state));
        let mut runtime = LiveAlertRuntime::new(playbook).expect("runtime initializes");

        runtime
            .evaluate(&snapshots, 1_710_002_014_000)
            .expect("first evaluation succeeds");
        runtime
            .evaluate(&snapshots, 1_710_002_015_000)
            .expect("unchanged redraw succeeds");
        assert_eq!(runtime.records().len(), 1);

        snapshots[0].spread_shock_bps = snapshots[0].spread_shock_bps.map(|value| value + 1.0);
        runtime
            .evaluate(&snapshots, 1_710_002_016_000)
            .expect("changed observation succeeds");
        assert_eq!(runtime.records().len(), 2);
    }

    #[test]
    fn invalid_live_arguments_fail_preflight_before_runtime_side_effects() {
        let cases = [
            (vec!["hls-live", "--refresh-secs", "0"], "--refresh-secs"),
            (vec!["hls-live", "--top", "0"], "--top"),
            (
                vec!["hls-live", "--symbols", "@107", "--all-symbols"],
                "--symbols",
            ),
            (vec!["hls-live", "--raw"], "--record"),
            (vec!["hls-live", "--run-id", "orphan"], "--record"),
            (vec!["hls-live", "--backfill-gaps"], "--record"),
            (
                vec!["hls-live", "--record", "--raw", "--backfill-gaps"],
                "normalized",
            ),
            (
                vec![
                    "hls-live",
                    "--record",
                    "--backfill-gaps",
                    "--backfill-interval",
                    "2m",
                ],
                "supported public candle interval",
            ),
            (
                vec![
                    "hls-live",
                    "--record",
                    "--backfill-gaps",
                    "--rest-url",
                    "http://example.com",
                ],
                "HTTPS or an HTTP loopback",
            ),
            (vec!["hls-live", "--max-subscriptions", "1001"], "official"),
            (
                vec!["hls-live", "--duration-secs", "18446744073709551615"],
                "too large",
            ),
        ];
        let terminals = LiveTerminalCapabilities::new(true, true);

        for (argv, expected) in cases {
            let parsed = LiveArgsParseHarness::try_parse_from(argv).expect("clap accepts shape");
            let error = validate_live_args(&parsed.args, terminals)
                .expect_err("semantic preflight rejects invalid arguments");
            assert!(error.to_string().contains(expected), "{error:#}");
        }
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
        assert_eq!(
            preflight_tui_preferences(temp.path()).preferences,
            preferences
        );
        let updated_preferences = WorkstationUiPreferences {
            view: WorkstationView::Quality,
            density: WorkstationDensity::Compact,
            chart_window: WorkstationChartWindow::SixtyMinutes,
        };
        save_tui_preferences(temp.path(), updated_preferences)
            .expect("existing regular preferences update atomically");
        assert_eq!(
            preflight_tui_preferences(temp.path()).preferences,
            updated_preferences
        );
        let entries: Vec<_> = fs::read_dir(temp.path())
            .expect("preference directory reads")
            .map(|entry| entry.expect("preference entry reads").file_name())
            .collect();
        assert_eq!(entries, vec![TUI_PREFERENCES_FILE]);
    }

    #[cfg(unix)]
    #[test]
    fn live_tui_preferences_load_rejects_symlink_destination() {
        use std::os::unix::fs::symlink;

        let temp = tempfile::tempdir().expect("tempdir");
        let outside = temp.path().join("outside.toml");
        fs::write(
            &outside,
            r#"
view = "flow"
density = "dense"
chart_window = "30m"
"#,
        )
        .expect("outside sentinel writes");
        let data_dir = temp.path().join("data");
        fs::create_dir(&data_dir).expect("data directory creates");
        symlink(&outside, tui_preferences_path(&data_dir)).expect("preference symlink creates");

        let error = try_load_tui_preferences(&data_dir)
            .expect_err("preference load must reject a symbolic-link destination");

        assert!(error.to_string().contains("symbolic link"), "{error:#}");
    }

    #[cfg(unix)]
    #[test]
    fn live_tui_preferences_save_rejects_symlink_without_touching_target() {
        use std::os::unix::fs::symlink;

        let temp = tempfile::tempdir().expect("tempdir");
        let outside = temp.path().join("outside.toml");
        let sentinel = "outside sentinel\n";
        fs::write(&outside, sentinel).expect("outside sentinel writes");
        let data_dir = temp.path().join("data");
        fs::create_dir(&data_dir).expect("data directory creates");
        symlink(&outside, tui_preferences_path(&data_dir)).expect("preference symlink creates");

        let error = save_tui_preferences(&data_dir, WorkstationUiPreferences::default())
            .expect_err("preference save must reject a symbolic-link destination");

        assert!(error.to_string().contains("symbolic link"), "{error:#}");
        assert_eq!(
            fs::read_to_string(&outside).expect("outside sentinel reads"),
            sentinel
        );
    }

    #[cfg(unix)]
    #[test]
    fn live_tui_preferences_reject_symlinked_data_directory() {
        use std::os::unix::fs::symlink;

        let temp = tempfile::tempdir().expect("tempdir");
        let outside_dir = temp.path().join("outside");
        fs::create_dir(&outside_dir).expect("outside directory creates");
        fs::write(
            tui_preferences_path(&outside_dir),
            "view = \"flow\"\ndensity = \"dense\"\nchart_window = \"30m\"\n",
        )
        .expect("outside preferences write");
        let data_dir = temp.path().join("linked-data");
        symlink(&outside_dir, &data_dir).expect("data-directory symlink creates");

        let load_error = try_load_tui_preferences(&data_dir)
            .expect_err("preference load must reject a symbolic-link parent");
        let save_error = save_tui_preferences(&data_dir, WorkstationUiPreferences::default())
            .expect_err("preference save must reject a symbolic-link parent");

        assert!(
            load_error.to_string().contains("symbolic link"),
            "{load_error:#}"
        );
        assert!(
            save_error.to_string().contains("symbolic link"),
            "{save_error:#}"
        );
        assert!(
            fs::read_to_string(tui_preferences_path(&outside_dir))
                .expect("outside preferences read")
                .contains("view = \"flow\"")
        );
    }

    #[cfg(unix)]
    #[test]
    fn live_tui_preferences_reject_intermediate_symlink_parent() {
        use std::os::unix::fs::symlink;

        let temp = tempfile::tempdir().expect("tempdir");
        let outside_dir = temp.path().join("outside/nested");
        fs::create_dir_all(&outside_dir).expect("outside directory creates");
        let outside_preferences = tui_preferences_path(&outside_dir);
        let sentinel = "view = \"flow\"\ndensity = \"dense\"\nchart_window = \"30m\"\n";
        fs::write(&outside_preferences, sentinel).expect("outside preferences write");
        let requested_root = temp.path().join("requested");
        fs::create_dir(&requested_root).expect("requested root creates");
        let linked_parent = requested_root.join("linked-parent");
        symlink(
            outside_dir.parent().expect("outside directory has parent"),
            &linked_parent,
        )
        .expect("intermediate parent symlink creates");
        let data_dir = linked_parent.join("nested");

        let load_result = try_load_tui_preferences(&data_dir);
        let save_result = save_tui_preferences(&data_dir, WorkstationUiPreferences::default());

        assert!(
            load_result
                .as_ref()
                .is_err_and(|error| error.to_string().contains("symbolic link")),
            "load followed intermediate parent: {load_result:?}"
        );
        assert!(
            save_result
                .as_ref()
                .is_err_and(|error| error.to_string().contains("symbolic link")),
            "save followed intermediate parent: {save_result:?}"
        );
        assert_eq!(
            fs::read_to_string(&outside_preferences).expect("outside preferences read"),
            sentinel
        );
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
            preflight_tui_preferences(temp.path()).preferences,
            WorkstationUiPreferences::default()
        );

        fs::write(&path, "not valid toml = [").expect("write malformed preference");
        assert_eq!(
            preflight_tui_preferences(temp.path()).preferences,
            WorkstationUiPreferences::default()
        );
    }

    #[test]
    fn live_tui_preflight_captures_malformed_preferences_before_activation() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::write(tui_preferences_path(temp.path()), "not valid toml = [")
            .expect("write malformed preference");

        let preflight = preflight_tui_preferences(temp.path());

        assert_eq!(preflight.preferences, WorkstationUiPreferences::default());
        assert!(
            preflight
                .warning
                .as_deref()
                .is_some_and(|warning| warning.contains("tui preferences load skipped"))
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
        let summary = LiveDriveSummary::default();

        ui_state.apply(WorkstationAction::CycleFilter, 1);
        for ch in "spread_bps < 20".chars() {
            ui_state.apply(WorkstationAction::CommandChar(ch), 1);
        }
        assert!(
            submit_live_command(&mut ui_state, &state, &mut request, &summary)
                .expect("valid applies")
        );
        assert_eq!(request.where_expr.as_deref(), Some("spread_bps < 20"));
        assert!(ui_state.command().is_none());

        ui_state.apply(WorkstationAction::CycleFilter, 1);
        for ch in "symbol > 10".chars() {
            ui_state.apply(WorkstationAction::CommandChar(ch), 1);
        }
        assert!(
            submit_live_command(&mut ui_state, &state, &mut request, &summary)
                .expect("invalid handled")
        );

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
        let summary = LiveDriveSummary::default();

        ui_state.apply(WorkstationAction::CyclePreset, 1);
        for ch in "thin_books".chars() {
            ui_state.apply(WorkstationAction::CommandChar(ch), 1);
        }
        assert!(
            submit_live_command(&mut ui_state, &state, &mut request, &summary)
                .expect("preset applies")
        );
        assert_eq!(request.preset.as_deref(), Some("thin_books"));
        assert!(request.where_expr.is_none());
        assert!(request.sort.is_none());

        ui_state.apply(WorkstationAction::CycleSort, 1);
        for ch in "spread_bps:asc".chars() {
            ui_state.apply(WorkstationAction::CommandChar(ch), 1);
        }
        assert!(
            submit_live_command(&mut ui_state, &state, &mut request, &summary)
                .expect("sort applies")
        );
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
        let summary = LiveDriveSummary::default();

        ui_state.apply(WorkstationAction::OpenSymbolSearch, 1);
        for ch in "@107".chars() {
            ui_state.apply(WorkstationAction::CommandChar(ch), 1);
        }
        assert!(
            submit_live_command(&mut ui_state, &state, &mut request, &summary)
                .expect("symbol applies")
        );
        assert_eq!(ui_state.selected_index(1), Some(0));
        assert_eq!(ui_state.selected_symbol(), Some("@107"));
        assert!(ui_state.command().is_none());
        assert_eq!(request, ScreenRequest::default());

        ui_state.apply(WorkstationAction::OpenSymbolSearch, 1);
        for ch in "NOPE".chars() {
            ui_state.apply(WorkstationAction::CommandChar(ch), 1);
        }
        assert!(
            submit_live_command(&mut ui_state, &state, &mut request, &summary)
                .expect("miss handled")
        );
        assert_eq!(
            ui_state.command_error(),
            Some("no visible symbol matches 'NOPE'")
        );
        assert!(ui_state.command().is_some());
        assert_eq!(request, ScreenRequest::default());
    }

    #[test]
    fn live_recorder_drop_joins_worker_as_unclean() {
        let temp = tempfile::tempdir().expect("tempdir");
        let data_dir = temp.path().to_path_buf();
        let run_id = "live-worker-drop-test";
        let recorder = LiveRecorder::new(&data_dir, run_id, vec!["@107".to_owned()], true, true)
            .expect("live recorder starts");

        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        recorder
            .send(LiveRecordCommand::WaitForRelease {
                entered: entered_tx,
                release: release_rx,
            })
            .expect("worker wait command enqueues");
        entered_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("worker reaches wait command");

        let (drop_done_tx, drop_done_rx) = mpsc::channel();
        let drop_thread = thread::spawn(move || {
            drop(recorder);
            let _ = drop_done_tx.send(());
        });
        assert!(
            drop_done_rx
                .recv_timeout(Duration::from_millis(50))
                .is_err(),
            "recorder Drop must wait for its worker"
        );
        release_tx.send(()).expect("worker released");
        drop_done_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("recorder drop completes after worker release");
        drop_thread.join().expect("drop thread joins");

        let registry = MetadataRegistry::open(data_dir.join("hls.sqlite"))
            .expect("metadata registry reopens after recorder drop");
        let run = registry
            .get_run(run_id)
            .expect("get run")
            .expect("run exists");
        assert_eq!(run.clean_shutdown, Some(false));
        assert!(run.ended_at_ms.is_some());
    }

    #[test]
    fn live_recorder_worker_error_still_persists_unclean_shutdown() {
        let temp = tempfile::tempdir().expect("tempdir");
        let data_dir = temp.path().to_path_buf();
        let run_id = "live-worker-error-test";
        let recorder = LiveRecorder::new(&data_dir, run_id, vec!["@107".to_owned()], true, false)
            .expect("live recorder starts");

        recorder
            .record_raw_line(1_710_000_000_123_456_789, 3, "not-json".to_owned())
            .expect("invalid raw line enqueues before worker parses it");
        drop(recorder);

        let registry = MetadataRegistry::open(data_dir.join("hls.sqlite"))
            .expect("metadata registry reopens after worker error");
        let run = registry
            .get_run(run_id)
            .expect("get run")
            .expect("run exists");
        assert_eq!(run.clean_shutdown, Some(false));
        assert!(run.ended_at_ms.is_some());
    }

    #[test]
    fn live_recorder_worker_initialization_failure_terminalizes_inserted_run() {
        let temp = tempfile::tempdir().expect("tempdir");
        let data_dir = temp.path().to_path_buf();
        let run_id = "live-worker-init-failure-test";
        let normalized_path =
            data_dir.join(format!("normalized/events/run={run_id}/part-000000.ndjson"));
        fs::create_dir_all(
            normalized_path
                .parent()
                .expect("normalized path has parent"),
        )
        .expect("normalized directory creates");
        fs::write(&normalized_path, "existing evidence\n").expect("conflicting file writes");

        let construction_error =
            match LiveRecorder::new(&data_dir, run_id, vec!["@107".to_owned()], false, true) {
                Ok(_) => panic!("existing normalized evidence must fail worker construction"),
                Err(error) => error,
            };
        assert!(
            construction_error.to_string().contains("exists"),
            "{construction_error}"
        );

        let registry = MetadataRegistry::open(data_dir.join("hls.sqlite"))
            .expect("metadata registry reopens after initialization failure");
        let run = registry
            .get_run(run_id)
            .expect("get run")
            .expect("failed run remains as audit evidence");
        assert_eq!(run.clean_shutdown, Some(false));
        assert!(run.ended_at_ms.is_some());
    }

    #[test]
    fn live_recorder_spawn_failure_happens_before_run_insertion() {
        let temp = tempfile::tempdir().expect("tempdir");
        let data_dir = temp.path().to_path_buf();
        let run_id = "live-worker-spawn-failure-test";

        let error = match LiveRecorder::new_with_spawner(
            &data_dir,
            run_id,
            vec!["@107".to_owned()],
            true,
            true,
            |_worker| Err(io::Error::other("injected thread spawn failure")),
        ) {
            Ok(_) => panic!("injected thread spawn failure must fail construction"),
            Err(error) => error,
        };

        assert!(error.to_string().contains("injected thread spawn failure"));
        let registry = MetadataRegistry::open(data_dir.join("hls.sqlite"))
            .expect("metadata registry opens after spawn failure");
        assert_eq!(
            registry
                .get_run(run_id)
                .expect("get run after spawn failure"),
            None,
            "thread spawn failure must occur before inserting audit state"
        );
    }

    #[test]
    fn live_recorder_writer_closeout_failure_terminalizes_run_as_unclean() {
        let temp = tempfile::tempdir().expect("tempdir");
        let data_dir = temp.path().to_path_buf();
        let run_id = "live-worker-closeout-failure-test";
        let mut worker =
            LiveRecorderWorker::new(&data_dir, run_id, vec!["@107".to_owned()], true, false)
                .expect("worker starts");
        worker
            .record_raw_line(
                1_710_000_000_123_456_789,
                3,
                r#"{"channel":"trades","data":[]}"#,
            )
            .expect("raw event records");
        let raw_path = data_dir.join(format!("raw/ws/run={run_id}/part-000000.ndjson.zst"));
        fs::create_dir_all(raw_path.parent().expect("raw path has parent"))
            .expect("raw directory creates");
        fs::write(&raw_path, "existing evidence\n").expect("conflicting raw file writes");

        worker
            .finish(true)
            .expect_err("raw writer closeout must preserve the existing file and fail");

        let registry = MetadataRegistry::open(data_dir.join("hls.sqlite"))
            .expect("metadata registry reopens after closeout failure");
        let run = registry
            .get_run(run_id)
            .expect("get run")
            .expect("failed run remains as audit evidence");
        assert_eq!(run.clean_shutdown, Some(false));
        assert!(run.ended_at_ms.is_some());
    }

    #[test]
    fn live_recorder_file_registration_failures_are_combined_before_terminalization() {
        use hls_store::metadata::FileRegistryEntry;

        let temp = tempfile::tempdir().expect("tempdir");
        let data_dir = temp.path().to_path_buf();
        let run_id = "live-worker-registration-failure-test";
        let mut worker =
            LiveRecorderWorker::new(&data_dir, run_id, vec!["@107".to_owned()], true, true)
                .expect("worker starts");
        let line = r#"{"channel":"trades","data":[{"coin":"@107","side":"B","px":"35.00","sz":"2.0","time":1710000000000,"hash":"0xabc","tid":11}]}"#;
        worker
            .record_raw_line(1_710_000_000_123_456_789, 3, line)
            .expect("raw event records");
        worker
            .record_events(&parse_ws_message(line).expect("normalized event parses"))
            .expect("normalized event records");
        let raw_path = format!("raw/ws/run={run_id}/part-000000.ndjson.zst");
        let normalized_path = format!("normalized/events/run={run_id}/part-000000.ndjson");
        for (path, event_type) in [
            (&raw_path, "raw_ws"),
            (&normalized_path, "normalized_jsonl"),
        ] {
            worker
                .registry
                .insert_file(&FileRegistryEntry {
                    path: path.clone(),
                    event_type: event_type.to_owned(),
                    symbol: None,
                    start_ts_ms: None,
                    end_ts_ms: None,
                    rows: 0,
                    bytes: 0,
                    created_at_ms: 0,
                    run_id: run_id.to_owned(),
                })
                .expect("conflicting registry evidence inserts");
        }

        let error = worker
            .finish(true)
            .expect_err("duplicate file evidence must fail closeout");

        assert!(error.to_string().contains(&raw_path), "{error}");
        assert!(error.to_string().contains(&normalized_path), "{error}");
        let registry = MetadataRegistry::open(data_dir.join("hls.sqlite"))
            .expect("metadata registry reopens after registration failure");
        let run = registry
            .get_run(run_id)
            .expect("get run")
            .expect("failed run remains as audit evidence");
        assert_eq!(run.clean_shutdown, Some(false));
        assert!(run.ended_at_ms.is_some());
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
        assert_eq!(run.clean_shutdown, Some(true));
        let gaps = registry.list_gaps("live-worker-test").expect("gaps list");
        assert_eq!(gaps[0].reason, "test reconnect");
        assert_eq!(gaps[0].affected_symbols, vec!["@107".to_owned()]);
        assert!(
            !gaps[0].recovered,
            "a reconnect only restores the live connection, not the missing evidence"
        );
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
