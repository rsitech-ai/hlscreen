use std::collections::{BTreeMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::health::HealthSnapshot;
use crate::telemetry::OperationsTelemetry;
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetricSupport {
    Canonical,
    Proxy,
    Unavailable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetricSamplingMode {
    ExchangeEventWindow,
    FixedTimeBucket,
    LatestPublicSnapshot,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MetricSamplingContract {
    pub metric_name: String,
    pub version: u32,
    pub window_ms: u64,
    pub minimum_observations: u64,
    pub sampling_mode: MetricSamplingMode,
    pub unit: String,
    pub absolute_tolerance: f64,
    pub relative_tolerance: f64,
}

impl MetricSamplingContract {
    pub fn validate(&self) -> HlsResult<()> {
        if !is_snake_case_identifier(&self.metric_name) {
            return Err(HlsError::Config(format!(
                "sampling contract metric '{}' must be snake_case",
                self.metric_name
            )));
        }
        if self.version == 0 {
            return Err(HlsError::Config(format!(
                "sampling contract '{}' version must be positive",
                self.metric_name
            )));
        }
        if self.window_ms == 0 {
            return Err(HlsError::Config(format!(
                "sampling contract '{}' window must be positive",
                self.metric_name
            )));
        }
        if self.minimum_observations == 0 {
            return Err(HlsError::Config(format!(
                "sampling contract '{}' observation floor must be positive",
                self.metric_name
            )));
        }
        if self.unit.trim().is_empty() {
            return Err(HlsError::Config(format!(
                "sampling contract '{}' unit cannot be empty",
                self.metric_name
            )));
        }
        if !self.absolute_tolerance.is_finite()
            || !self.relative_tolerance.is_finite()
            || self.absolute_tolerance < 0.0
            || self.relative_tolerance < 0.0
            || (self.absolute_tolerance == 0.0 && self.relative_tolerance == 0.0)
        {
            return Err(HlsError::Config(format!(
                "sampling contract '{}' tolerances must be finite, non-negative, and not both zero",
                self.metric_name
            )));
        }
        Ok(())
    }

    pub fn has_sufficient_observations(&self, observations: usize) -> bool {
        u64::try_from(observations).is_ok_and(|count| count >= self.minimum_observations)
    }

    pub fn validate_value(&self, actual: f64, expected: f64) -> HlsResult<()> {
        self.validate()?;
        if !actual.is_finite() || !expected.is_finite() {
            return Err(HlsError::Config(format!(
                "sampling contract '{}' values must be finite",
                self.metric_name
            )));
        }
        let allowed = self
            .absolute_tolerance
            .max(self.relative_tolerance * expected.abs());
        let difference = (actual - expected).abs();
        if difference > allowed {
            return Err(HlsError::Config(format!(
                "sampling contract '{}' value {actual} is outside tolerance {allowed} of expected {expected}",
                self.metric_name
            )));
        }
        Ok(())
    }
}

impl MetricSupport {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Canonical => "canonical",
            Self::Proxy => "proxy",
            Self::Unavailable => "unavailable",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MicrostructureMetricDefinition {
    pub name: String,
    pub formula: String,
    pub unit: String,
    pub required_inputs: Vec<String>,
    pub support: MetricSupport,
    pub caveat: Option<String>,
}

impl MicrostructureMetricDefinition {
    pub fn canonical(
        name: impl Into<String>,
        formula: impl Into<String>,
        unit: impl Into<String>,
        required_inputs: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            name: name.into(),
            formula: formula.into(),
            unit: unit.into(),
            required_inputs: required_inputs.into_iter().map(Into::into).collect(),
            support: MetricSupport::Canonical,
            caveat: None,
        }
    }

    pub fn proxy(
        name: impl Into<String>,
        formula: impl Into<String>,
        unit: impl Into<String>,
        required_inputs: impl IntoIterator<Item = impl Into<String>>,
        caveat: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            formula: formula.into(),
            unit: unit.into(),
            required_inputs: required_inputs.into_iter().map(Into::into).collect(),
            support: MetricSupport::Proxy,
            caveat: Some(caveat.into()),
        }
    }

    pub fn unavailable(
        name: impl Into<String>,
        formula: impl Into<String>,
        unit: impl Into<String>,
        required_inputs: impl IntoIterator<Item = impl Into<String>>,
        caveat: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            formula: formula.into(),
            unit: unit.into(),
            required_inputs: required_inputs.into_iter().map(Into::into).collect(),
            support: MetricSupport::Unavailable,
            caveat: Some(caveat.into()),
        }
    }

    pub fn validate(&self) -> HlsResult<()> {
        if !is_snake_case_identifier(&self.name) {
            return Err(HlsError::Config(format!(
                "microstructure metric '{}' must be snake_case",
                self.name
            )));
        }
        if self.formula.trim().is_empty() {
            return Err(HlsError::Config(format!(
                "microstructure metric '{}' formula cannot be empty",
                self.name
            )));
        }
        if self.unit.trim().is_empty() {
            return Err(HlsError::Config(format!(
                "microstructure metric '{}' unit cannot be empty",
                self.name
            )));
        }
        if self.required_inputs.is_empty() {
            return Err(HlsError::Config(format!(
                "microstructure metric '{}' requires at least one input",
                self.name
            )));
        }

        let mut inputs = HashSet::new();
        for input in &self.required_inputs {
            if !is_snake_case_identifier(input) {
                return Err(HlsError::Config(format!(
                    "microstructure metric '{}' input '{}' must be snake_case",
                    self.name, input
                )));
            }
            if input.contains("private")
                || input.contains("wallet")
                || input.contains("account")
                || input.contains("order")
            {
                return Err(HlsError::Config(format!(
                    "microstructure metric '{}' cannot require private or execution input '{}'",
                    self.name, input
                )));
            }
            if !inputs.insert(input.clone()) {
                return Err(HlsError::Config(format!(
                    "microstructure metric '{}' repeats input '{}'",
                    self.name, input
                )));
            }
        }

        if !matches!(self.support, MetricSupport::Canonical)
            && self
                .caveat
                .as_deref()
                .map(str::trim)
                .unwrap_or_default()
                .is_empty()
        {
            return Err(HlsError::Config(format!(
                "{} metric '{}' requires an explicit caveat",
                self.support.as_str(),
                self.name
            )));
        }

        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MicrostructureMetricSnapshot {
    pub name: String,
    pub support: MetricSupport,
    pub value: Option<f64>,
    pub unit: String,
    pub reason: Option<String>,
}

impl MicrostructureMetricSnapshot {
    pub fn canonical(contract: &MetricSamplingContract, observations: usize, value: f64) -> Self {
        if let Err(error) = contract.validate() {
            return Self::unavailable(
                contract.metric_name.clone(),
                contract.unit.clone(),
                format!("invalid canonical sampling contract: {error}"),
            );
        }
        if !contract.has_sufficient_observations(observations) {
            return Self::unavailable(
                contract.metric_name.clone(),
                contract.unit.clone(),
                format!(
                    "canonical sampling contract requires at least {} observations",
                    contract.minimum_observations
                ),
            );
        }
        if !value.is_finite() {
            return Self::unavailable(
                contract.metric_name.clone(),
                contract.unit.clone(),
                "canonical metric value must be finite",
            );
        }
        Self {
            name: contract.metric_name.clone(),
            support: MetricSupport::Canonical,
            value: Some(value),
            unit: contract.unit.clone(),
            reason: None,
        }
    }

    pub fn proxy(
        name: impl Into<String>,
        value: f64,
        unit: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            support: MetricSupport::Proxy,
            value: Some(value),
            unit: unit.into(),
            reason: Some(reason.into()),
        }
    }

    pub fn unavailable(
        name: impl Into<String>,
        unit: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            support: MetricSupport::Unavailable,
            value: None,
            unit: unit.into(),
            reason: Some(reason.into()),
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

pub fn operations_metrics_snapshot(
    generated_at_ms: u128,
    telemetry: &OperationsTelemetry,
) -> HlsResult<MetricsSnapshot> {
    let mut samples = vec![
        unlabelled_sample(
            "hls_reconnect_attempts_total",
            MetricKind::Counter,
            "Total public WebSocket connection attempts after the initial attempt.",
            telemetry.reconnect_attempts,
        ),
        unlabelled_sample(
            "hls_parser_drops_total",
            MetricKind::Counter,
            "Total public messages rejected by the parser.",
            telemetry.parser_drops,
        ),
        unlabelled_sample(
            "hls_stale_duration_ms_total",
            MetricKind::Counter,
            "Cumulative time that inbound public data was stale.",
            telemetry.stale_duration_ms,
        ),
        unlabelled_sample(
            "hls_unrepaired_gap_duration_ms",
            MetricKind::Gauge,
            "Current cumulative duration of gaps without exact tick-level repair.",
            telemetry.unrepaired_gap_duration_ms,
        ),
    ];
    if let Some(repair_latency_ms) = telemetry.repair_latency_ms {
        samples.push(unlabelled_sample(
            "hls_repair_latency_ms",
            MetricKind::Gauge,
            "Elapsed time for the most recent completed coarse public repair attempt.",
            repair_latency_ms,
        ));
    }
    MetricsSnapshot::new(generated_at_ms, samples)
}

fn unlabelled_sample(name: &str, kind: MetricKind, description: &str, value: u64) -> MetricSample {
    MetricSample::new(
        MetricDefinition::new(name, kind, description, std::iter::empty::<&str>()),
        BTreeMap::new(),
        value as f64,
    )
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
