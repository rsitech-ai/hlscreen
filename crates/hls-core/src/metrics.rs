use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::{HlsError, HlsResult};

const HIGH_CARDINALITY_LABELS: &[&str] = &[
    "symbol", "hl_coin", "coin", "run_id", "wallet", "account", "address", "tx_hash", "trade_id",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetricKind {
    Counter,
    Gauge,
    Histogram,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MetricDefinition {
    pub name: String,
    pub kind: MetricKind,
    pub description: String,
    pub labels: Vec<String>,
}

impl MetricDefinition {
    pub fn new(
        name: impl Into<String>,
        kind: MetricKind,
        description: impl Into<String>,
        labels: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            name: name.into(),
            kind,
            description: description.into(),
            labels: labels.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MetricsRegistry {
    pub definitions: Vec<MetricDefinition>,
}

impl MetricsRegistry {
    pub fn new(definitions: Vec<MetricDefinition>) -> Self {
        Self { definitions }
    }

    pub fn validate(&self) -> HlsResult<()> {
        let mut names = HashSet::new();
        for definition in &self.definitions {
            validate_definition(definition)?;
            if !names.insert(definition.name.clone()) {
                return Err(HlsError::Config(format!(
                    "duplicate metric definition '{}'",
                    definition.name
                )));
            }
        }
        Ok(())
    }
}

fn validate_definition(definition: &MetricDefinition) -> HlsResult<()> {
    if !definition.name.starts_with("hls_") {
        return Err(HlsError::Config(format!(
            "metric '{}' must start with hls_",
            definition.name
        )));
    }
    if definition.description.trim().is_empty() {
        return Err(HlsError::Config(format!(
            "metric '{}' description cannot be empty",
            definition.name
        )));
    }

    let mut labels = HashSet::new();
    for label in &definition.labels {
        if HIGH_CARDINALITY_LABELS.contains(&label.as_str()) {
            return Err(HlsError::Config(format!(
                "metric '{}' uses high-cardinality label '{}'",
                definition.name, label
            )));
        }
        if !is_snake_case_identifier(label) {
            return Err(HlsError::Config(format!(
                "metric '{}' label '{}' must be snake_case",
                definition.name, label
            )));
        }
        if !labels.insert(label.clone()) {
            return Err(HlsError::Config(format!(
                "metric '{}' repeats label '{}'",
                definition.name, label
            )));
        }
    }

    Ok(())
}

fn is_snake_case_identifier(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_')
        && !value.starts_with('_')
        && !value.ends_with('_')
        && !value.contains("__")
}
