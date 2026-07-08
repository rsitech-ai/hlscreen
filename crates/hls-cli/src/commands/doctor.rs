use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::Context;
use clap::Args;
use hls_core::config::{default_config_for_data_dir, load_config};
use hls_hyperliquid::rest::HyperliquidRestClient;
use serde_json::json;

use crate::commands::health::require_live_health;

#[derive(Debug, Args)]
pub struct DoctorArgs {
    #[arg(long, default_value = ".hls")]
    pub data_dir: PathBuf,

    #[arg(long)]
    pub live: bool,

    #[arg(long)]
    pub json: bool,

    #[arg(long, hide = true)]
    pub simulate_health: Option<String>,
}

pub async fn run(args: DoctorArgs) -> anyhow::Result<()> {
    fs::create_dir_all(&args.data_dir)
        .with_context(|| format!("create data directory {}", args.data_dir.display()))?;

    let config_path = args.data_dir.join("config.toml");
    let (config, config_readable, config_error) = match load_config(&config_path) {
        Ok(config) => (config, true, None),
        Err(error) if config_path.exists() => (
            default_config_for_data_dir(args.data_dir.clone()),
            false,
            Some(error.to_string()),
        ),
        Err(_) => (
            default_config_for_data_dir(args.data_dir.clone()),
            false,
            None,
        ),
    };
    let data_dir_writable = check_writable(&args.data_dir)?;
    let read_only_ok = config_error.is_none()
        && config.safety.read_only
        && !config.safety.wallet_enabled
        && !config.safety.trading_enabled;
    let live_rest_ok = if args.live && args.simulate_health.is_none() {
        Some(HyperliquidRestClient::default().spot_meta().await.is_ok())
    } else {
        None
    };
    let health = require_live_health(args.live, args.simulate_health.as_deref(), live_rest_ok)?;

    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "config_path": config_path,
                "config_readable": config_readable,
                "config_error": config_error,
                "data_dir_writable": data_dir_writable,
                "read_only": read_only_ok,
                "live_rest": live_rest_ok,
                "health": health,
            }))?
        );
    } else {
        println!("config: {}", config_path.display());
        println!(
            "config readable: {}",
            if config_readable {
                "ok"
            } else if config_path.exists() {
                "fail"
            } else {
                "missing"
            }
        );
        if let Some(config_error) = config_error {
            println!("config error: {config_error}");
        }
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
        if let Some(health) = health {
            println!("health: {}", health.status.as_str());
            for reason in health.degraded_reasons {
                println!("health reason: {reason}");
            }
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
