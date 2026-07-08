use std::path::PathBuf;

use anyhow::bail;
use clap::Args;
use hls_store::benchmark::run_benchmark_pack;

#[derive(Debug, Args)]
pub struct BenchArgs {
    #[arg(long)]
    pub manifest: PathBuf,

    #[arg(long, default_value = ".")]
    pub repo_root: PathBuf,

    #[arg(long)]
    pub json: bool,
}

pub async fn run(args: BenchArgs) -> anyhow::Result<()> {
    let report = run_benchmark_pack(&args.manifest, &args.repo_root)?;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("benchmark: {}", report.fixture_id);
        println!("description: {}", report.description);
        println!("events_read={}", report.events_read);
        println!("snapshot_count={}", report.snapshot_count);
        println!("feature_latency_us={}", report.feature_latency_us);
        println!("max_feature_latency_us={}", report.max_feature_latency_us);
        println!("expected_hash={}", report.expected_hash);
        println!("output_hash={}", report.output_hash);
        println!("matched={}", report.matched);
    }

    if !report.matched {
        bail!(
            "benchmark hash drift for {}: expected {}, got {}",
            report.fixture_id,
            report.expected_hash,
            report.output_hash
        );
    }
    if report.feature_latency_us > report.max_feature_latency_us {
        bail!(
            "benchmark feature latency for {} exceeded limit: {}us > {}us",
            report.fixture_id,
            report.feature_latency_us,
            report.max_feature_latency_us
        );
    }

    Ok(())
}
