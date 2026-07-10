use std::path::PathBuf;

use anyhow::Context;
use clap::{Args, ValueEnum};
use hls_store::{
    metadata::FileRegistryEntry,
    parquet::{export_feature_snapshots_to_parquet, export_normalized_events_to_parquet},
};

#[derive(Debug, Args)]
pub struct ExportParquetArgs {
    #[arg(long)]
    pub run_id: String,

    #[arg(long, default_value = ".hls")]
    pub data_dir: PathBuf,

    #[arg(long, value_enum, default_value_t = ParquetDataset::Events)]
    pub dataset: ParquetDataset,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum ParquetDataset {
    Events,
    Features,
    All,
}

pub async fn run(args: ExportParquetArgs) -> anyhow::Result<()> {
    let mut entries = Vec::new();
    if matches!(args.dataset, ParquetDataset::Events | ParquetDataset::All) {
        entries.push(
            export_normalized_events_to_parquet(&args.data_dir, &args.run_id)
                .with_context(|| format!("export normalized run '{}' to parquet", args.run_id))?,
        );
    }
    if matches!(args.dataset, ParquetDataset::Features | ParquetDataset::All) {
        entries.push(
            export_feature_snapshots_to_parquet(&args.data_dir, &args.run_id).with_context(
                || {
                    format!(
                        "export feature snapshots for run '{}' to parquet",
                        args.run_id
                    )
                },
            )?,
        );
    }

    for entry in entries {
        print_entry(&entry);
    }

    Ok(())
}

fn print_entry(entry: &FileRegistryEntry) {
    println!("parquet_run={}", entry.run_id);
    println!("event_type={}", entry.event_type);
    println!("rows={}", entry.rows);
    println!("bytes={}", entry.bytes);
    println!("path={}", entry.path);
}
