use std::collections::{BTreeMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::health::HealthSnapshot;
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MetricSample {
    pub name: String,
    pub kind: MetricKind,
    pub description: String,
    pub labels: BTreeMap<String, String>,
    pub value: f64,
}

impl MetricSample {
    pub fn new(definition: MetricDefinition, labels: BTreeMap<String, String>, value: f64) -> Self {
        Self {
            name: definition.name,
            kind: definition.kind,
            description: definition.description,
            labels,
            value,
        }
    }

    fn validate(&self) -> HlsResult<()> {
        let definition = self.definition();
        validate_definition(&definition)?;
        for value in self.labels.values() {
            if value.trim().is_empty() {
                return Err(HlsError::Config(format!(
                    "metric '{}' label values cannot be empty",
                    self.name
                )));
            }
        }
        if !self.value.is_finite() {
            return Err(HlsError::Config(format!(
                "metric '{}' sample value must be finite",
                self.name
            )));
        }
        Ok(())
    }

    fn definition(&self) -> MetricDefinition {
        MetricDefinition::new(
            self.name.clone(),
            self.kind,
            self.description.clone(),
            self.labels.keys().cloned(),
        )
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub generated_at_ms: u128,
    pub samples: Vec<MetricSample>,
    pub prometheus_text: String,
}

impl MetricsSnapshot {
    pub fn new(generated_at_ms: u128, samples: Vec<MetricSample>) -> HlsResult<Self> {
        for sample in &samples {
            sample.validate()?;
        }
        let prometheus_text = render_prometheus_text(&samples)?;
        Ok(Self {
            generated_at_ms,
            samples,
            prometheus_text,
        })
    }
}

pub fn doctor_metrics_snapshot(
    generated_at_ms: u128,
    read_only_ok: bool,
    data_dir_writable: bool,
    live_rest_ok: Option<bool>,
    health: Option<&HealthSnapshot>,
) -> HlsResult<MetricsSnapshot> {
    let mut samples = vec![
        MetricSample::new(
            MetricDefinition::new(
                "hls_read_only_safety_ok",
                MetricKind::Gauge,
                "Whether the local configuration is in read-only public-data mode.",
                std::iter::empty::<&str>(),
            ),
            BTreeMap::new(),
            bool_value(read_only_ok),
        ),
        MetricSample::new(
            MetricDefinition::new(
                "hls_data_dir_writable",
                MetricKind::Gauge,
                "Whether the configured local data directory can be written.",
                std::iter::empty::<&str>(),
            ),
            BTreeMap::new(),
            bool_value(data_dir_writable),
        ),
    ];

    if let Some(live_rest_ok) = live_rest_ok {
        samples.push(MetricSample::new(
            MetricDefinition::new(
                "hls_live_rest_ok",
                MetricKind::Gauge,
                "Whether the public Hyperliquid REST health probe succeeded.",
                ["source"],
            ),
            BTreeMap::from([("source".to_owned(), "public_rest".to_owned())]),
            bool_value(live_rest_ok),
        ));
    }

    if let Some(health) = health {
        samples.extend([
            MetricSample::new(
                MetricDefinition::new(
                    "hls_health_status",
                    MetricKind::Gauge,
                    "Current low-cardinality health status encoded as a one-hot sample.",
                    ["status"],
                ),
                BTreeMap::from([("status".to_owned(), health.status.as_str().to_owned())]),
                1.0,
            ),
            MetricSample::new(
                MetricDefinition::new(
                    "hls_writer_backlog_events",
                    MetricKind::Gauge,
                    "Current bounded writer backlog in local events.",
                    std::iter::empty::<&str>(),
                ),
                BTreeMap::new(),
                health.writer_backlog as f64,
            ),
            MetricSample::new(
                MetricDefinition::new(
                    "hls_reconnects_total",
                    MetricKind::Counter,
                    "Total reconnects observed in the current local health snapshot.",
                    std::iter::empty::<&str>(),
                ),
                BTreeMap::new(),
                health.reconnect_count as f64,
            ),
            MetricSample::new(
                MetricDefinition::new(
                    "hls_data_gaps_total",
                    MetricKind::Counter,
                    "Total explicit public data gaps observed in the current health snapshot.",
                    std::iter::empty::<&str>(),
                ),
                BTreeMap::new(),
                health.gap_count as f64,
            ),
        ]);
    }

    MetricsSnapshot::new(generated_at_ms, samples)
}

fn render_prometheus_text(samples: &[MetricSample]) -> HlsResult<String> {
    let mut rendered = String::new();
    let mut rendered_defs = HashSet::new();
    for sample in samples {
        sample.validate()?;
        if rendered_defs.insert(sample.name.clone()) {
            rendered.push_str("# HELP ");
            rendered.push_str(&sample.name);
            rendered.push(' ');
            rendered.push_str(&sample.description);
            rendered.push('\n');
            rendered.push_str("# TYPE ");
            rendered.push_str(&sample.name);
            rendered.push(' ');
            rendered.push_str(metric_kind_name(sample.kind));
            rendered.push('\n');
        }
        rendered.push_str(&sample.name);
        if !sample.labels.is_empty() {
            rendered.push('{');
            for (index, (key, value)) in sample.labels.iter().enumerate() {
                if index > 0 {
                    rendered.push(',');
                }
                rendered.push_str(key);
                rendered.push_str("=\"");
                rendered.push_str(&escape_label_value(value));
                rendered.push('"');
            }
            rendered.push('}');
        }
        rendered.push(' ');
        rendered.push_str(&format_metric_value(sample.value));
        rendered.push('\n');
    }
    Ok(rendered)
}

fn bool_value(value: bool) -> f64 {
    if value { 1.0 } else { 0.0 }
}

fn metric_kind_name(kind: MetricKind) -> &'static str {
    match kind {
        MetricKind::Counter => "counter",
        MetricKind::Gauge => "gauge",
        MetricKind::Histogram => "histogram",
    }
}

fn escape_label_value(value: &str) -> String {
    value
        .replace('\\', r"\\")
        .replace('\n', r"\n")
        .replace('"', r#"\""#)
}

fn format_metric_value(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{value:.0}")
    } else {
        value.to_string()
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
