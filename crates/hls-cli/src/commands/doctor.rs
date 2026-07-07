use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::Context;
use clap::Args;
use hls_core::config::{default_config_for_data_dir, load_config};
use hls_hyperliquid::rest::HyperliquidRestClient;
use serde_json::json;

#[derive(Debug, Args)]
pub struct DoctorArgs {
    #[arg(long, default_value = ".hls")]
    pub data_dir: PathBuf,

    #[arg(long)]
    pub live: bool,

    #[arg(long)]
    pub json: bool,
}

pub async fn run(args: DoctorArgs) -> anyhow::Result<()> {
    fs::create_dir_all(&args.data_dir)
        .with_context(|| format!("create data directory {}", args.data_dir.display()))?;

    let config_path = args.data_dir.join("config.toml");
    let config = match load_config(&config_path) {
        Ok(config) => config,
        Err(_) => default_config_for_data_dir(args.data_dir.clone()),
    };
    let config_readable = config_path.exists() && load_config(&config_path).is_ok();
    let data_dir_writable = check_writable(&args.data_dir)?;
    let read_only_ok =
        config.safety.read_only && !config.safety.wallet_enabled && !config.safety.trading_enabled;
    let live_rest_ok = if args.live {
        Some(HyperliquidRestClient::default().spot_meta().await.is_ok())
    } else {
        None
    };

    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "config_path": config_path,
                "config_readable": config_readable,
                "data_dir_writable": data_dir_writable,
                "read_only": read_only_ok,
                "live_rest": live_rest_ok,
            }))?
        );
    } else {
        println!("config: {}", config_path.display());
        println!(
            "config readable: {}",
            if config_readable { "ok" } else { "missing" }
        );
        println!(
            "data-dir writable: {}",
            if data_dir_writable { "ok" } else { "fail" }
        );
        println!(
            "read-only safety: {}",
            if read_only_ok { "ok" } else { "fail" }
        );

        if let Some(live_ok) = live_rest_ok {
            println!("live REST: {}", if live_ok { "ok" } else { "fail" });
        }
    }

    Ok(())
}

fn check_writable(data_dir: &Path) -> anyhow::Result<bool> {
    let probe = data_dir.join(".hlscreen-doctor-write-probe");
    fs::write(&probe, b"ok").with_context(|| format!("write probe {}", probe.display()))?;
    fs::remove_file(&probe).with_context(|| format!("remove probe {}", probe.display()))?;
    Ok(true)
}
