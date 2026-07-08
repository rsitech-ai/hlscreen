use std::path::Path;

use hls_core::{HlsError, HlsResult};
use serde::{Deserialize, Serialize};

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
