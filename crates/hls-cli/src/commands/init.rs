use std::{fs, path::PathBuf};

use anyhow::{Context, bail};
use clap::Args;
use hls_core::config::{config_to_toml, default_config_for_data_dir};
use hls_hyperliquid::rest::HyperliquidRestClient;

#[derive(Debug, Args)]
pub struct InitArgs {
    #[arg(long, default_value = ".hls")]
    pub data_dir: PathBuf,

    #[arg(long)]
    pub force: bool,

    #[arg(long)]
    pub check_live: bool,
}

pub async fn run(args: InitArgs) -> anyhow::Result<()> {
    fs::create_dir_all(&args.data_dir)
        .with_context(|| format!("create data directory {}", args.data_dir.display()))?;

    let config_path = args.data_dir.join("config.toml");
    if config_path.exists() && !args.force {
        bail!(
            "config already exists at {}; use --force to overwrite",
            config_path.display()
        );
    }

    let config = default_config_for_data_dir(args.data_dir.clone());
    fs::write(&config_path, config_to_toml(&config)?)
        .with_context(|| format!("write config {}", config_path.display()))?;

    println!("config: {}", config_path.display());
    println!("data_dir: {}", args.data_dir.display());
    println!("read_only=true");

    if args.check_live {
        HyperliquidRestClient::default().spot_meta().await?;
        println!("live_rest=ok");
    }

    Ok(())
}
