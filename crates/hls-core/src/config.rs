use std::{fs, path::Path, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{HlsError, HlsResult};

const DEFAULT_REST_BASE_URL: &str = "https://api.hyperliquid.xyz";
const DEFAULT_WS_BASE_URL: &str = "wss://api.hyperliquid.xyz/ws";

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct AppConfig {
    pub data_dir: PathBuf,
    pub network: NetworkConfig,
    pub recording: RecordingConfig,
    pub universe: UniverseConfig,
    pub streams: StreamConfig,
    pub features: FeatureConfig,
    pub terminal: TerminalConfig,
    pub safety: SafetyConfig,
}

impl AppConfig {
    pub fn validate(&self) -> HlsResult<()> {
        if self.universe.top_n == 0 {
            return Err(HlsError::Config(
                "universe.top_n must be greater than zero".to_owned(),
            ));
        }

        if self.streams.l2_book {
            return Err(HlsError::Config(
                "full order-book ingestion is out of scope for v1; keep streams.l2_book=false"
                    .to_owned(),
            ));
        }

        if !self.safety.read_only {
            return Err(HlsError::Config(
                "safety.read_only must remain true".to_owned(),
            ));
        }

        if self.safety.wallet_enabled || self.safety.trading_enabled {
            return Err(HlsError::Config(
                "wallet and trading surfaces are forbidden in hlscreen v1".to_owned(),
            ));
        }

        Ok(())
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            data_dir: PathBuf::from(".hls"),
            network: NetworkConfig::default(),
            recording: RecordingConfig::default(),
            universe: UniverseConfig::default(),
            streams: StreamConfig::default(),
            features: FeatureConfig::default(),
            terminal: TerminalConfig::default(),
            safety: SafetyConfig::default(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct NetworkConfig {
    pub environment: String,
    pub rest_base_url: String,
    pub ws_base_url: String,
    pub heartbeat_secs: u64,
    pub stale_after_secs: u64,
    pub max_reconnect_attempts: u32,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            environment: "mainnet".to_owned(),
            rest_base_url: DEFAULT_REST_BASE_URL.to_owned(),
            ws_base_url: DEFAULT_WS_BASE_URL.to_owned(),
            heartbeat_secs: 30,
            stale_after_secs: 10,
            max_reconnect_attempts: 12,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct RecordingConfig {
    pub raw: bool,
    pub normalized: bool,
    pub max_raw_file_mb: u32,
    pub flush_secs: u64,
}

impl Default for RecordingConfig {
    fn default() -> Self {
        Self {
            raw: true,
            normalized: true,
            max_raw_file_mb: 256,
            flush_secs: 5,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct UniverseConfig {
    pub top_n: usize,
    pub include: Vec<String>,
    pub exclude: Vec<String>,
    pub subscription_headroom: usize,
}

impl Default for UniverseConfig {
    fn default() -> Self {
        Self {
            top_n: 150,
            include: Vec::new(),
            exclude: Vec::new(),
            subscription_headroom: 20,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct StreamConfig {
    pub trades: bool,
    pub bbo: bool,
    pub all_mids: bool,
    pub active_asset_ctx: bool,
    pub candles_1m: bool,
    pub l2_book: bool,
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            trades: true,
            bbo: true,
            all_mids: true,
            active_asset_ctx: true,
            candles_1m: true,
            l2_book: false,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct FeatureConfig {
    pub windows: Vec<String>,
    pub baseline_window: String,
}

impl Default for FeatureConfig {
    fn default() -> Self {
        Self {
            windows: vec!["1m".to_owned(), "5m".to_owned(), "1h".to_owned()],
            baseline_window: "1h".to_owned(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct TerminalConfig {
    pub refresh_hz: u32,
}

impl Default for TerminalConfig {
    fn default() -> Self {
        Self { refresh_hz: 5 }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct SafetyConfig {
    pub read_only: bool,
    pub wallet_enabled: bool,
    pub trading_enabled: bool,
}

impl Default for SafetyConfig {
    fn default() -> Self {
        Self {
            read_only: true,
            wallet_enabled: false,
            trading_enabled: false,
        }
    }
}

pub fn load_config(path: impl AsRef<Path>) -> HlsResult<AppConfig> {
    load_config_str(&fs::read_to_string(path)?)
}

pub fn load_config_str(raw: &str) -> HlsResult<AppConfig> {
    let config: AppConfig = toml::from_str(raw)?;
    config.validate()?;
    Ok(config)
}

pub fn default_config_for_data_dir(data_dir: impl Into<PathBuf>) -> AppConfig {
    AppConfig {
        data_dir: data_dir.into(),
        ..AppConfig::default()
    }
}

pub fn config_to_toml(config: &AppConfig) -> HlsResult<String> {
    config.validate()?;
    Ok(toml::to_string_pretty(config)?)
}
