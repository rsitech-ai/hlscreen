use std::path::PathBuf;

use anyhow::Context;
use clap::{Args, ValueEnum};
use hls_store::{
    metadata::FileRegistryEntry,
    parquet::{
        export_all_to_parquet, export_feature_snapshots_to_parquet,
        export_normalized_events_to_parquet,
    },
};

#[derive(Debug, Args)]
pub struct ExportParquetArgs {
    /// Recorded run to export.
    #[arg(long)]
    pub run_id: String,

    /// Local recording directory and export destination root.
    #[arg(long, default_value = ".hls")]
    pub data_dir: PathBuf,

    /// Dataset family to export: normalized events, feature snapshots, or both.
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
    let entries = match args.dataset {
        ParquetDataset::Events => vec![
            export_normalized_events_to_parquet(&args.data_dir, &args.run_id)
                .with_context(|| format!("export normalized run '{}' to parquet", args.run_id))?,
        ],
        ParquetDataset::Features => vec![
            export_feature_snapshots_to_parquet(&args.data_dir, &args.run_id).with_context(
                || {
                    format!(
                        "export feature snapshots for run '{}' to parquet",
                        args.run_id
                    )
                },
            )?,
        ],
        ParquetDataset::All => export_all_to_parquet(&args.data_dir, &args.run_id)
            .with_context(|| format!("export all datasets for run '{}' to parquet", args.run_id))?,
    };

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
