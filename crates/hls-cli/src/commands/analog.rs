use std::path::PathBuf;

use anyhow::{Context, bail};
use clap::Args;
use hls_store::analog::{
    AnalogIndex, AnalogSearchOptions, AnalogSearchRunOptions, build_analog_index_for_run,
    search_analogs_in_index,
};

#[derive(Debug, Args)]
pub struct AnalogArgs {
    /// Target feed identifier to search when building an index from a replay.
    #[arg(long)]
    pub symbol: Option<String>,

    /// Recorded run to index and search.
    #[arg(long)]
    pub run_id: Option<String>,

    /// Local recording directory.
    #[arg(long, default_value = ".hls")]
    pub data_dir: PathBuf,

    /// Maximum number of nearest matches to return.
    #[arg(long, default_value_t = 5)]
    pub limit: usize,

    /// Minimum candidate count required before returning matches.
    #[arg(long, default_value_t = 1)]
    pub min_candidates: usize,

    /// Emit the report as JSON.
    #[arg(long)]
    pub json: bool,

    /// Write a schema-versioned local analog index JSON file while searching a replay run.
    #[arg(long)]
    pub write_index: Option<PathBuf>,

    /// Search a prebuilt local analog index JSON file instead of rescanning normalized events.
    #[arg(long)]
    pub index_file: Option<PathBuf>,
}

pub async fn run(args: AnalogArgs) -> anyhow::Result<()> {
    if args.index_file.is_some() && args.write_index.is_some() {
        bail!("--index-file and --write-index cannot be used together");
    }

    let search_options = AnalogSearchOptions {
        limit: args.limit,
        min_candidates: args.min_candidates,
    };
    let index = if let Some(index_file) = &args.index_file {
        AnalogIndex::read_json(index_file)
            .with_context(|| format!("read analog index '{}'", index_file.display()))?
    } else {
        let run_id = args
            .run_id
            .as_deref()
            .context("analog requires --run-id unless --index-file is provided")?;
        let symbol = args
            .symbol
            .as_deref()
            .context("analog requires --symbol unless --index-file is provided")?;
        let index = build_analog_index_for_run(AnalogSearchRunOptions::new(
            &args.data_dir,
            run_id,
            symbol,
            search_options.clone(),
        ))
        .with_context(|| format!("build analog index for run '{run_id}'"))?;
        if let Some(write_index) = &args.write_index {
            index
                .write_json(write_index)
                .with_context(|| format!("write analog index '{}'", write_index.display()))?;
            if !args.json {
                println!("analog_index_file={} read_only=true", write_index.display());
            }
        }
        index
    };

    let report = search_analogs_in_index(&index, search_options)
        .with_context(|| format!("search analog index '{}'", index.source_run_id))?;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }

    println!(
        "analog run={} target={} ts={} candidates={} read_only=true",
        report.run_id.as_deref().unwrap_or("-"),
        report.target_symbol,
        report.target_ts_ms,
        report.candidate_count
    );
    if let Some(reason) = &report.insufficient_evidence {
        println!("insufficient_evidence={reason}");
        return Ok(());
    }
    for analog_match in &report.matches {
        let drivers = analog_match
            .drivers
            .iter()
            .map(|driver| format!("{}:{:.4}", driver.field, driver.contribution))
            .collect::<Vec<_>>()
            .join(",");
        println!(
            "match symbol={} ts={} distance={:.6} drivers={}",
            analog_match.symbol, analog_match.snapshot_ts_ms, analog_match.distance, drivers
        );
    }

    Ok(())
}
