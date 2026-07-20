use std::{fs, path::PathBuf};

use anyhow::{Context, bail};
use clap::Args;
use hls_core::{
    extension::{ExtensionManifest, ExtensionRuntime, RowAnnotation},
    market_state::{FeatureSnapshot, LiveMarketState},
};
use hls_features::engine::FeatureEngine;
use hls_hyperliquid::ws::parser::parse_ws_ndjson;
use hls_store::replay::{ReplayOptions, replay_run};

#[derive(Debug, Args)]
pub struct ExtensionArgs {
    /// Validated local extension manifest JSON file.
    #[arg(long)]
    pub manifest: PathBuf,

    /// Exported WebAssembly function to invoke.
    #[arg(long, default_value = "annotate_row")]
    pub entrypoint: String,

    /// Feed identifier whose feature snapshot is passed to the extension.
    #[arg(long)]
    pub symbol: String,

    /// Recorded run to replay.
    #[arg(long)]
    pub run_id: Option<String>,

    /// Local recording directory.
    #[arg(long, default_value = ".hls")]
    pub data_dir: PathBuf,

    #[arg(long, hide = true)]
    pub fixture_file: Option<PathBuf>,

    /// Emit extension annotations as JSON.
    #[arg(long)]
    pub json: bool,
}

pub async fn run(args: ExtensionArgs) -> anyhow::Result<()> {
    let manifest = read_manifest(&args.manifest)?;
    let snapshot = if let Some(fixture_file) = &args.fixture_file {
        snapshot_from_fixture(fixture_file, &args.symbol)?
    } else {
        let Some(run_id) = &args.run_id else {
            bail!("extension requires --run-id unless --fixture-file is provided");
        };
        snapshot_from_replay(&args.data_dir, run_id, &args.symbol)?
    };

    let manifest_dir = args
        .manifest
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| std::path::Path::new("."));
    let runtime = ExtensionRuntime::with_default_limits()?;
    let annotations =
        runtime.invoke_row_annotations(&manifest, manifest_dir, &args.entrypoint, &snapshot)?;

    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "manifest": manifest.name,
                "entrypoint": args.entrypoint,
                "symbol": snapshot.symbol,
                "read_only": true,
                "annotations": annotations,
            }))?
        );
    } else {
        print_text(
            &manifest.name,
            &args.entrypoint,
            &snapshot.symbol,
            &annotations,
        );
    }

    Ok(())
}

fn read_manifest(path: &PathBuf) -> anyhow::Result<ExtensionManifest> {
    let raw = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    serde_json::from_str(&raw).with_context(|| format!("parse {}", path.display()))
}

fn snapshot_from_fixture(fixture_file: &PathBuf, symbol: &str) -> anyhow::Result<FeatureSnapshot> {
    let raw = fs::read_to_string(fixture_file)
        .with_context(|| format!("read {}", fixture_file.display()))?;
    let events = parse_ws_ndjson(&raw)?;
    let mut state = LiveMarketState::new([symbol.to_owned()]);
    for event in events {
        state.apply(event)?;
    }
    let now_ms = state
        .states()
        .filter_map(|symbol_state| symbol_state.last_update_ms)
        .max()
        .unwrap_or_default();
    snapshot_from_rows(
        FeatureEngine::default().snapshots(&state, now_ms),
        symbol,
        "fixture",
    )
}

fn snapshot_from_replay(
    data_dir: &PathBuf,
    run_id: &str,
    symbol: &str,
) -> anyhow::Result<FeatureSnapshot> {
    let summary = replay_run(ReplayOptions::new(
        data_dir,
        run_id,
        vec![symbol.to_owned()],
    ))
    .with_context(|| format!("replay recording run '{run_id}'"))?;
    snapshot_from_rows(summary.snapshots, symbol, "replay")
}

fn snapshot_from_rows(
    snapshots: Vec<FeatureSnapshot>,
    symbol: &str,
    source: &str,
) -> anyhow::Result<FeatureSnapshot> {
    snapshots
        .into_iter()
        .find(|snapshot| snapshot.symbol == symbol)
        .with_context(|| format!("symbol '{symbol}' was not found in {source} rows"))
}

fn print_text(manifest_name: &str, entrypoint: &str, symbol: &str, annotations: &[RowAnnotation]) {
    println!(
        "extension manifest={manifest_name} entrypoint={entrypoint} symbol={symbol} read_only=true annotations={}",
        annotations.len()
    );
    for annotation in annotations {
        println!(
            "annotation symbol={} label={} detail={}",
            annotation.symbol, annotation.label, annotation.detail
        );
    }
}
