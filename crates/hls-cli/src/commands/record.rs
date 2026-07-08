use std::{fs, path::PathBuf};

use anyhow::{Context, bail};
use clap::Args;
use hls_core::time::now_millis;
use hls_store::recorder::{RecordOptions, record_fixture_ndjson};

#[derive(Debug, Args)]
pub struct RecordArgs {
    #[arg(long)]
    pub symbols: Option<String>,

    #[arg(long)]
    pub run_id: Option<String>,

    #[arg(long)]
    pub raw: bool,

    #[arg(long)]
    pub normalized: bool,

    #[arg(long)]
    pub parquet: bool,

    #[arg(long, default_value = ".hls")]
    pub data_dir: PathBuf,

    #[arg(long, hide = true)]
    pub fixture_file: Option<PathBuf>,
}

pub async fn run(args: RecordArgs) -> anyhow::Result<()> {
    if args.parquet {
        bail!(
            "Parquet output is not implemented in this slice; use --normalized for replayable JSONL"
        );
    }

    let Some(fixture_file) = &args.fixture_file else {
        bail!(
            "network recording is not implemented in this slice; use --fixture-file for deterministic mock recording"
        );
    };

    let raw = fs::read_to_string(fixture_file)
        .with_context(|| format!("read {}", fixture_file.display()))?;
    let run_id = args.run_id.unwrap_or_else(default_run_id);
    let (raw_enabled, normalized_enabled) = enabled_outputs(args.raw, args.normalized);
    let options = RecordOptions::new(
        &args.data_dir,
        &run_id,
        parse_symbols(args.symbols.as_deref()),
        raw_enabled,
        normalized_enabled,
    );
    let summary = record_fixture_ndjson(&raw, options)?;

    println!("recording run: {}", summary.run_id);
    println!("raw_messages={}", summary.raw_messages);
    println!("normalized_events={}", summary.normalized_events);
    println!("raw_files={}", summary.raw_files.len());
    println!("normalized_files={}", summary.normalized_files.len());
    println!("clean_shutdown={}", summary.clean_shutdown);

    Ok(())
}

pub fn enabled_outputs(raw: bool, normalized: bool) -> (bool, bool) {
    if !raw && !normalized {
        return (true, true);
    }
    (raw, normalized)
}

pub fn parse_symbols(symbols: Option<&str>) -> Vec<String> {
    symbols
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|symbol| !symbol.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

pub fn default_run_id() -> String {
    match now_millis() {
        Ok(now) => format!("run-{now}"),
        Err(_) => "run-unknown-time".to_owned(),
    }
}
