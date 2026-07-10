use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use anyhow::{Context, bail};
use clap::Args;
use hls_core::{
    alerts::{
        AlertAction, AlertCondition, AlertHistoryRecord, AlertKey, AlertPlaybook, AlertRule,
        AlertSeverity,
    },
    market_state::{FeatureSnapshot, LiveMarketState},
};
use hls_features::{alerts::AlertEvaluator, engine::FeatureEngine};
use hls_hyperliquid::ws::parser::parse_ws_ndjson;
use hls_store::replay::{ReplayOptions, replay_run};

#[derive(Debug, Args)]
pub struct AlertsArgs {
    #[arg(long)]
    pub symbol: Option<String>,

    #[arg(long)]
    pub run_id: Option<String>,

    #[arg(long, default_value = ".hls")]
    pub data_dir: PathBuf,

    #[arg(long, hide = true)]
    pub fixture_file: Option<PathBuf>,

    #[arg(long)]
    pub json: bool,

    #[arg(long)]
    pub playbook_file: Option<PathBuf>,

    /// Append emitted and suppressed local alert evidence as JSONL; no external delivery is performed.
    #[arg(long)]
    pub alert_history_file: Option<PathBuf>,

    /// Read local alert history JSONL and list recent records; no replay/live evaluation is performed.
    #[arg(long)]
    pub history_file: Option<PathBuf>,

    #[arg(long, default_value_t = 20)]
    pub limit: usize,

    #[arg(long, default_value_t = 250.0)]
    pub min_spread_shock_bps: f64,

    #[arg(long, default_value_t = 70)]
    pub max_confidence_score: u8,

    #[arg(long, default_value_t = 60_000)]
    pub cooldown_ms: i64,

    #[arg(long, default_value_t = 30_000)]
    pub source_interval_ms: i64,
}

pub async fn run(args: AlertsArgs) -> anyhow::Result<()> {
    if let Some(history_file) = &args.history_file {
        return list_alert_history(history_file, args.symbol.as_deref(), args.limit, args.json);
    }

    let symbol = args
        .symbol
        .as_deref()
        .context("alerts requires --symbol unless --history-file is provided")?;
    let (snapshots, now_ms) = if let Some(fixture_file) = &args.fixture_file {
        snapshots_from_fixture(fixture_file, symbol)?
    } else {
        let Some(run_id) = &args.run_id else {
            bail!("alerts requires --run-id unless --fixture-file is provided");
        };
        let summary = replay_run(ReplayOptions::new(
            &args.data_dir,
            run_id,
            vec![symbol.to_owned()],
        ))
        .with_context(|| format!("replay recording run '{run_id}'"))?;
        (summary.snapshots, summary.snapshot_ts_ms)
    };

    let snapshot = snapshots
        .iter()
        .find(|snapshot| snapshot.symbol == symbol)
        .with_context(|| format!("symbol '{symbol}' was not found in replayed rows"))?;

    let playbook = load_playbook(&args)?;
    playbook.validate()?;

    let mut evaluator = AlertEvaluator::default();
    if let Some(path) = &args.alert_history_file {
        restore_alert_cooldowns(path, &mut evaluator)?;
    }
    let evaluation = evaluator.evaluate(&playbook, std::slice::from_ref(snapshot), now_ms)?;
    if let Some(path) = &args.alert_history_file {
        append_alert_history(path, &evaluation)?;
    }

    if args.json {
        println!("{}", serde_json::to_string_pretty(&evaluation)?);
    } else {
        print_text(&evaluation);
    }

    Ok(())
}

#[derive(serde::Deserialize)]
struct PersistedCooldownRecord {
    kind: String,
    playbook_id: String,
    rule_id: String,
    symbol: String,
    #[serde(default)]
    triggered_at_ms: Option<i64>,
    action: AlertAction,
}

fn restore_alert_cooldowns(path: &Path, evaluator: &mut AlertEvaluator) -> anyhow::Result<()> {
    let raw = match fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error).with_context(|| format!("read {}", path.display())),
    };
    for (index, line) in raw.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let record: PersistedCooldownRecord = serde_json::from_str(line)
            .with_context(|| format!("parse {} line {}", path.display(), index + 1))?;
        if record.action != AlertAction::LocalOnly {
            bail!("alert history line {} is not local_only", index + 1);
        }
        if record.kind != "event" {
            continue;
        }
        let emitted_at_ms = record.triggered_at_ms.with_context(|| {
            format!(
                "alert history {} line {} event has no triggered_at_ms",
                path.display(),
                index + 1
            )
        })?;
        evaluator.remember_emission(
            AlertKey::new(&record.playbook_id, &record.rule_id, &record.symbol),
            emitted_at_ms,
        );
    }
    Ok(())
}

fn list_alert_history(
    path: &Path,
    symbol_filter: Option<&str>,
    limit: usize,
    json: bool,
) -> anyhow::Result<()> {
    let records = read_alert_history(path, symbol_filter, limit)?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "history_file": path,
                "records": records,
            }))?
        );
    } else {
        println!(
            "alert_history records={} file={}",
            records.len(),
            path.display()
        );
        for record in &records {
            println!(
                "{} {:?} {} {} {} confidence={} action=local_only reason={}",
                record.kind,
                record.severity,
                record.symbol,
                record.playbook_id,
                record.rule_id,
                record.confidence_score,
                record.reason
            );
        }
    }
    Ok(())
}

pub(crate) fn read_alert_history(
    path: &Path,
    symbol_filter: Option<&str>,
    limit: usize,
) -> anyhow::Result<Vec<AlertHistoryRecord>> {
    let raw = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let mut records = Vec::new();
    for (index, line) in raw.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let record: AlertHistoryRecord = serde_json::from_str(line)
            .with_context(|| format!("parse {} line {}", path.display(), index + 1))?;
        if record.action != AlertAction::LocalOnly {
            bail!("alert history line {} is not local_only", index + 1);
        }
        if symbol_filter.is_some_and(|symbol| symbol != record.symbol) {
            continue;
        }
        records.push(record);
    }
    records.reverse();
    records.truncate(limit);
    Ok(records)
}

fn snapshots_from_fixture(
    fixture_file: &PathBuf,
    symbol: &str,
) -> anyhow::Result<(Vec<FeatureSnapshot>, i64)> {
    let raw = fs::read_to_string(fixture_file)
        .with_context(|| format!("read {}", fixture_file.display()))?;
    let events = parse_ws_ndjson(&raw)?;
    let mut state = LiveMarketState::new([symbol.to_owned()]);
    for event in events {
        state.apply(event)?;
    }
    let now_ms = latest_update_ms(&state);
    Ok((FeatureEngine::default().snapshots(&state, now_ms), now_ms))
}

fn latest_update_ms(state: &LiveMarketState) -> i64 {
    state
        .states()
        .filter_map(|symbol_state| symbol_state.last_update_ms)
        .max()
        .unwrap_or_default()
}

fn load_playbook(args: &AlertsArgs) -> anyhow::Result<AlertPlaybook> {
    if let Some(path) = &args.playbook_file {
        return load_playbook_file(path);
    }

    Ok(spread_shock_playbook(args))
}

pub(crate) fn load_playbook_file(path: &PathBuf) -> anyhow::Result<AlertPlaybook> {
    let raw = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("json");
    let playbook = if extension.eq_ignore_ascii_case("toml") {
        toml::from_str(&raw).with_context(|| format!("parse {}", path.display()))?
    } else {
        serde_json::from_str(&raw).with_context(|| format!("parse {}", path.display()))?
    };
    Ok(playbook)
}

pub(crate) fn append_alert_history(
    path: &Path,
    evaluation: &hls_core::alerts::AlertEvaluation,
) -> anyhow::Result<()> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("open alert history {}", path.display()))?;

    for event in &evaluation.events {
        let mut record = serde_json::to_value(event)?;
        if let serde_json::Value::Object(ref mut object) = record {
            object.insert(
                "kind".to_owned(),
                serde_json::Value::String("event".to_owned()),
            );
        }
        writeln!(file, "{}", serde_json::to_string(&record)?)?;
    }
    for suppressed in &evaluation.suppressed {
        let mut record = serde_json::to_value(suppressed)?;
        if let serde_json::Value::Object(ref mut object) = record {
            object.insert(
                "kind".to_owned(),
                serde_json::Value::String("suppressed".to_owned()),
            );
        }
        writeln!(file, "{}", serde_json::to_string(&record)?)?;
    }
    Ok(())
}

fn spread_shock_playbook(args: &AlertsArgs) -> AlertPlaybook {
    AlertPlaybook {
        schema_version: 1,
        id: "spread-shock-watch".to_owned(),
        description: "Local read-only spread-shock plus confidence watch".to_owned(),
        rules: vec![AlertRule {
            id: "shock-low-confidence".to_owned(),
            description: "Spread shock while confidence is below the configured threshold"
                .to_owned(),
            severity: AlertSeverity::Watch,
            condition: AlertCondition::SpreadShockAndLowConfidence {
                min_spread_shock_bps: args.min_spread_shock_bps,
                max_confidence_score: args.max_confidence_score,
            },
            cooldown_ms: args.cooldown_ms,
            source_interval_ms: args.source_interval_ms,
            action: AlertAction::LocalOnly,
        }],
    }
}

fn print_text(evaluation: &hls_core::alerts::AlertEvaluation) {
    println!(
        "alerts local_only events={} suppressed={}",
        evaluation.events.len(),
        evaluation.suppressed.len()
    );
    if evaluation.events.is_empty() && evaluation.suppressed.is_empty() {
        println!("no alerts emitted");
        return;
    }

    for event in &evaluation.events {
        println!(
            "event severity={:?} symbol={} rule={} confidence={}/{} reason={} action=local_only",
            event.severity,
            event.symbol,
            event.rule_id,
            event.confidence_score,
            event.confidence_level.as_str(),
            event.reason
        );
    }
    for suppressed in &evaluation.suppressed {
        println!(
            "suppressed severity={:?} symbol={} rule={} remaining_ms={} confidence={}/{} reason={} action=local_only",
            suppressed.severity,
            suppressed.symbol,
            suppressed.rule_id,
            suppressed.cooldown_remaining_ms,
            suppressed.confidence_score,
            suppressed.confidence_level.as_str(),
            suppressed.reason
        );
    }
}
