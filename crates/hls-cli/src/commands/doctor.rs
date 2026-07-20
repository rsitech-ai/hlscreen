use std::{
    fs,
    io::{self, IsTerminal},
    path::{Path, PathBuf},
};

use anyhow::Context;
use clap::Args;
use hls_core::config::{default_config_for_data_dir, load_config};
use hls_core::metrics::doctor_metrics_snapshot;
use hls_core::time::now_millis;
use hls_hyperliquid::rest::HyperliquidRestClient;
use hls_tui::health::render_health_pane;
use serde_json::json;

use crate::commands::health::require_live_health;
use crate::{HLS_RENDERER_ID, HLS_VERSION, commands::live::live_terminal_color_diagnostics};

#[derive(Debug, Args)]
pub struct DoctorArgs {
    /// Local data directory to inspect; created when terminal-only diagnostics are not selected.
    #[arg(long, default_value = ".hls")]
    pub data_dir: PathBuf,

    /// Check the public REST endpoint and render a live health snapshot.
    #[arg(long)]
    pub live: bool,

    /// Emit machine-readable JSON instead of text diagnostics.
    #[arg(long)]
    pub json: bool,

    /// Report terminal, renderer, and color-detection state without touching the data directory.
    #[arg(long, conflicts_with_all = ["live", "simulate_health"])]
    pub terminal: bool,

    #[arg(long, hide = true)]
    pub simulate_health: Option<String>,
}

pub async fn run(args: DoctorArgs) -> anyhow::Result<()> {
    if args.terminal {
        return run_terminal_diagnostics(args.json);
    }

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
    let metrics = if args.live {
        Some(doctor_metrics_snapshot(
            now_millis()?,
            read_only_ok,
            data_dir_writable,
            live_rest_ok,
            health.as_ref(),
        )?)
    } else {
        None
    };

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
                "metrics": metrics,
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
        if let Some(health) = &health {
            print!("{}", render_health_pane(health));
        }
    }

    Ok(())
}

fn run_terminal_diagnostics(json_output: bool) -> anyhow::Result<()> {
    let binary = std::env::current_exe().context("resolve current hls executable")?;
    let working_directory = std::env::current_dir().context("resolve current working directory")?;
    let stdin_tty = io::stdin().is_terminal();
    let stderr_tty = io::stderr().is_terminal();
    let color = live_terminal_color_diagnostics();
    let environment = [
        ("TERM", env_display("TERM")),
        ("COLORTERM", env_display("COLORTERM")),
        ("TMUX", env_display("TMUX")),
        ("NO_COLOR", env_display("NO_COLOR")),
        ("HLS_FORCE_COLOR", env_display("HLS_FORCE_COLOR")),
        ("CLICOLOR_FORCE", env_display("CLICOLOR_FORCE")),
        ("FORCE_COLOR", env_display("FORCE_COLOR")),
    ];

    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "binary": binary,
                "version": HLS_VERSION,
                "renderer": HLS_RENDERER_ID,
                "working_directory": working_directory,
                "stdin_tty": stdin_tty,
                "stderr_tty": stderr_tty,
                "environment": environment.into_iter().collect::<std::collections::BTreeMap<_, _>>(),
                "force_color": color.force_color,
                "auto_color": color.auto_color,
                "effective_auto_color": color.effective_auto_color,
                "tui_default_color": "always",
            }))?
        );
        return Ok(());
    }

    println!("binary: {}", binary.display());
    println!("version: {HLS_VERSION}");
    println!("renderer: {HLS_RENDERER_ID}");
    println!("working directory: {}", working_directory.display());
    println!("stdin tty: {stdin_tty}");
    println!("stderr tty: {stderr_tty}");
    for (name, value) in environment {
        println!("{name}: {value}");
    }
    println!("force-color override: {}", enabled_label(color.force_color));
    println!("auto-color detection: {}", enabled_label(color.auto_color));
    println!(
        "effective auto color: {}",
        enabled_label(color.effective_auto_color)
    );
    println!("tui default color: always");
    Ok(())
}

fn env_display(name: &str) -> String {
    std::env::var_os(name)
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|| "<unset>".to_owned())
}

fn enabled_label(enabled: bool) -> &'static str {
    if enabled { "enabled" } else { "disabled" }
}

fn check_writable(data_dir: &Path) -> anyhow::Result<bool> {
    let probe = data_dir.join(".hlscreen-doctor-write-probe");
    fs::write(&probe, b"ok").with_context(|| format!("write probe {}", probe.display()))?;
    fs::remove_file(&probe).with_context(|| format!("remove probe {}", probe.display()))?;
    Ok(true)
}
