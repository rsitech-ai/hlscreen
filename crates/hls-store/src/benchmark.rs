use std::{
    fs,
    path::{Path, PathBuf},
    time::Instant,
};

use hls_core::{
    HlsError, HlsResult,
    market_state::{FeatureSnapshot, LiveMarketState, MarketEvent},
};
use hls_features::engine::FeatureEngine;
use hls_hyperliquid::ws::parser::parse_ws_ndjson;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BenchmarkManifest {
    pub schema_version: u32,
    pub fixture_id: String,
    pub description: String,
    pub input_files: Vec<String>,
    pub expected_hash: String,
    pub max_feature_latency_us: u64,
    pub tags: Vec<String>,
}

impl BenchmarkManifest {
    pub fn from_json(value: &str) -> HlsResult<Self> {
        serde_json::from_str(value)
            .map_err(|err| HlsError::Parse(format!("parse benchmark manifest: {err}")))
    }

    pub fn validate(&self) -> HlsResult<()> {
        if self.schema_version != 1 {
            return Err(HlsError::Config(format!(
                "unsupported benchmark manifest schema_version {}; expected 1",
                self.schema_version
            )));
        }
        if self.fixture_id.trim().is_empty() {
            return Err(HlsError::Config(
                "benchmark fixture_id cannot be empty".to_owned(),
            ));
        }
        if self.description.trim().is_empty() {
            return Err(HlsError::Config(
                "benchmark description cannot be empty".to_owned(),
            ));
        }
        if self.input_files.is_empty() {
            return Err(HlsError::Config(
                "benchmark manifest requires at least one input file".to_owned(),
            ));
        }
        for input in &self.input_files {
            validate_public_relative_fixture(input)?;
        }
        if !is_sha256_hash(&self.expected_hash) {
            return Err(HlsError::Config(
                "benchmark expected_hash must use sha256:<64 hex chars>".to_owned(),
            ));
        }
        if self.max_feature_latency_us == 0 {
            return Err(HlsError::Config(
                "benchmark max_feature_latency_us must be greater than zero".to_owned(),
            ));
        }
        if self.tags.iter().any(|tag| tag == "private") {
            return Err(HlsError::Config(
                "benchmark tags must describe public fixtures only".to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BenchmarkReport {
    pub fixture_id: String,
    pub description: String,
    pub input_files: Vec<String>,
    pub events_read: u64,
    pub snapshot_ts_ms: i64,
    pub snapshot_count: usize,
    pub feature_latency_us: u64,
    pub max_feature_latency_us: u64,
    pub expected_hash: String,
    pub output_hash: String,
    pub matched: bool,
}

pub fn load_benchmark_manifest(path: impl AsRef<Path>) -> HlsResult<BenchmarkManifest> {
    let path = path.as_ref();
    let raw = fs::read_to_string(path).map_err(|err| {
        HlsError::External(format!("read benchmark manifest {}: {err}", path.display()))
    })?;
    BenchmarkManifest::from_json(&raw)
}

pub fn run_benchmark_pack(
    manifest_path: impl AsRef<Path>,
    repo_root: impl AsRef<Path>,
) -> HlsResult<BenchmarkReport> {
    let manifest = load_benchmark_manifest(manifest_path)?;
    run_benchmark_manifest(&manifest, repo_root)
}

pub fn run_benchmark_manifest(
    manifest: &BenchmarkManifest,
    repo_root: impl AsRef<Path>,
) -> HlsResult<BenchmarkReport> {
    manifest.validate()?;
    let repo_root = repo_root.as_ref();
    let mut events = Vec::new();
    for input in &manifest.input_files {
        let path = resolve_public_fixture(repo_root, input)?;
        let raw = fs::read_to_string(&path)
            .map_err(|err| HlsError::External(format!("read fixture {}: {err}", path.display())))?;
        events.extend(parse_ws_ndjson(&raw)?);
    }
    if events.is_empty() {
        return Err(HlsError::Config(format!(
            "benchmark '{}' produced no market events",
            manifest.fixture_id
        )));
    }

    let symbols = selected_symbols(&events);
    let mut state = LiveMarketState::new(symbols);
    for event in events.iter().cloned() {
        state.apply(event)?;
    }

    let snapshot_ts_ms = latest_update_ms(&state);
    let started = Instant::now();
    let snapshots = FeatureEngine::default().snapshots(&state, snapshot_ts_ms);
    let feature_latency_us = elapsed_us(started);
    let output_hash = hash_benchmark_output(&BenchmarkOutput {
        schema_version: 1,
        fixture_id: &manifest.fixture_id,
        events_read: events.len() as u64,
        snapshot_ts_ms,
        snapshots: &snapshots,
    })?;
    let matched = output_hash == manifest.expected_hash;

    Ok(BenchmarkReport {
        fixture_id: manifest.fixture_id.clone(),
        description: manifest.description.clone(),
        input_files: manifest.input_files.clone(),
        events_read: events.len() as u64,
        snapshot_ts_ms,
        snapshot_count: snapshots.len(),
        feature_latency_us,
        max_feature_latency_us: manifest.max_feature_latency_us,
        expected_hash: manifest.expected_hash.clone(),
        output_hash,
        matched,
    })
}

#[derive(Serialize)]
struct BenchmarkOutput<'a> {
    schema_version: u32,
    fixture_id: &'a str,
    events_read: u64,
    snapshot_ts_ms: i64,
    snapshots: &'a [FeatureSnapshot],
}

fn resolve_public_fixture(repo_root: &Path, input: &str) -> HlsResult<PathBuf> {
    validate_public_relative_fixture(input)?;
    let path = repo_root.join(input);
    if !path.exists() {
        return Err(HlsError::Config(format!(
            "benchmark input '{}' does not exist under {}",
            input,
            repo_root.display()
        )));
    }
    Ok(path)
}

fn selected_symbols(events: &[MarketEvent]) -> Vec<String> {
    let mut symbols: Vec<String> = events
        .iter()
        .filter_map(MarketEvent::hl_coin)
        .map(ToOwned::to_owned)
        .collect();
    symbols.sort();
    symbols.dedup();
    symbols
}

fn latest_update_ms(state: &LiveMarketState) -> i64 {
    state
        .states()
        .filter_map(|symbol_state| symbol_state.last_update_ms)
        .max()
        .unwrap_or_default()
}

fn elapsed_us(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX)
}

fn hash_benchmark_output(output: &BenchmarkOutput<'_>) -> HlsResult<String> {
    let encoded = serde_json::to_vec(output)
        .map_err(|err| HlsError::Parse(format!("serialize benchmark output: {err}")))?;
    let digest = Sha256::digest(encoded);
    Ok(format!("sha256:{digest:x}"))
}

fn validate_public_relative_fixture(input: &str) -> HlsResult<()> {
    let path = Path::new(input);
    if path.is_absolute()
        || input.contains("..")
        || input.contains("private")
        || input.contains("account")
        || !input.starts_with("tests/fixtures/microstructure/")
    {
        return Err(HlsError::Config(format!(
            "benchmark input '{input}' must be a relative public fixture under tests/fixtures/microstructure/"
        )));
    }
    Ok(())
}

fn is_sha256_hash(value: &str) -> bool {
    let Some(hash) = value.strip_prefix("sha256:") else {
        return false;
    };
    hash.len() == 64 && hash.chars().all(|ch| ch.is_ascii_hexdigit())
}
