use std::path::PathBuf;

use anyhow::{Context, bail};
use clap::Args;
use hls_store::replay::{ReplayOptions, replay_run, verify_or_insert_confidence_parity};
use hls_tui::app::{render_confidence_summary, render_table_with_title};

use crate::commands::record::parse_symbols;

#[derive(Debug, Args)]
pub struct ReplayArgs {
    #[arg(long)]
    pub run_id: String,

    #[arg(long)]
    pub symbols: Option<String>,

    #[arg(long, default_value = ".hls")]
    pub data_dir: PathBuf,

    #[arg(long)]
    pub verify_parity: bool,
}

pub async fn run(args: ReplayArgs) -> anyhow::Result<()> {
    let options = ReplayOptions::new(
        &args.data_dir,
        &args.run_id,
        parse_symbols(args.symbols.as_deref()),
    );
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
