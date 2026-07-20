use std::path::PathBuf;

use anyhow::{Context, bail};
use clap::{Args, ValueEnum};
use hls_store::replay::{
    ReplayInputFormat, ReplayOptions, replay_run, verify_or_insert_confidence_parity,
};
use hls_tui::app::{render_confidence_summary, render_table_with_title};

use crate::commands::record::parse_symbols;

#[derive(Debug, Args)]
pub struct ReplayArgs {
    /// Recorded run to replay.
    #[arg(long)]
    pub run_id: String,

    /// Optional comma-separated feed identifiers to replay.
    #[arg(long)]
    pub symbols: Option<String>,

    /// Local recording directory.
    #[arg(long, default_value = ".hls")]
    pub data_dir: PathBuf,

    /// Write or compare the run's deterministic confidence baseline.
    #[arg(long)]
    pub verify_parity: bool,

    #[arg(
        long,
        value_enum,
        default_value_t = ReplayInput::Jsonl,
        help = "Replay input format: jsonl uses canonical normalized files; parquet requires exported normalized-event Parquet and schema.json"
    )]
    pub input: ReplayInput,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum ReplayInput {
    Jsonl,
    Parquet,
}

pub async fn run(args: ReplayArgs) -> anyhow::Result<()> {
    let options = ReplayOptions::new(
        &args.data_dir,
        &args.run_id,
        parse_symbols(args.symbols.as_deref()),
    )
    .with_input_format(args.input.into());
    let summary = replay_run(options.clone())
        .with_context(|| format!("replay recording run '{}'", args.run_id))?;

    if args.verify_parity {
        let report = verify_or_insert_confidence_parity(&options, &summary)
            .with_context(|| format!("verify replay parity for '{}'", args.run_id))?;
        println!(
            "replay_parity={}",
            if report.baseline_written {
                "baseline_written"
            } else if report.matched {
                "passed"
            } else {
                "drifted"
            }
        );
        println!("confidence_baseline={}", report.baseline_count);
        println!("confidence_replay={}", report.replay_count);
        println!("confidence_drift={}", report.drift_count);
        println!("confidence_missing={}", report.missing_count);
        println!("confidence_extra={}", report.extra_count);
        if !report.matched {
            bail!(
                "replay parity drift detected for '{}': {}",
                report.run_id,
                report.details.join("; ")
            );
        }
    }

    println!("{}", render_confidence_summary(&summary.snapshots));

    let output = render_table_with_title(&summary.snapshots, "READ-ONLY Hyperliquid spot replay");
    print!("{output}");

    Ok(())
}

impl From<ReplayInput> for ReplayInputFormat {
    fn from(value: ReplayInput) -> Self {
        match value {
            ReplayInput::Jsonl => Self::Jsonl,
            ReplayInput::Parquet => Self::Parquet,
        }
    }
}
