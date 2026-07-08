use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::{HlsError, HlsResult};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScoreComponentKind {
    Liquidity,
    Momentum,
    MeanReversion,
    SpreadCost,
    SignedFlow,
    Confidence,
    Resilience,
    Metadata,
    Custom,
}

impl ScoreComponentKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Liquidity => "liquidity",
            Self::Momentum => "momentum",
            Self::MeanReversion => "mean_reversion",
            Self::SpreadCost => "spread_cost",
            Self::SignedFlow => "signed_flow",
            Self::Confidence => "confidence",
            Self::Resilience => "resilience",
            Self::Metadata => "metadata",
            Self::Custom => "custom",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScoreDirection {
    Positive,
    Negative,
    Neutral,
}

impl ScoreDirection {
    pub fn from_contribution(value: f64) -> Self {
        if value > 0.0 {
            Self::Positive
        } else if value < 0.0 {
            Self::Negative
        } else {
            Self::Neutral
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Positive => "positive",
            Self::Negative => "negative",
            Self::Neutral => "neutral",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScoreComponent {
    pub name: String,
    pub kind: ScoreComponentKind,
    pub value: f64,
    pub raw_value: f64,
    pub normalized_value: f64,
    pub weight: f64,
    pub signed_contribution: f64,
    pub direction: ScoreDirection,
    pub evidence_window: Option<String>,
}

impl ScoreComponent {
    pub fn new(name: impl Into<String>, kind: ScoreComponentKind, value: f64) -> Self {
        let value = finite_or_zero(value);
        Self {
            name: name.into(),
            kind,
            value,
            raw_value: value,
            normalized_value: value,
            weight: 1.0,
            signed_contribution: value,
            direction: ScoreDirection::from_contribution(value),
            evidence_window: None,
        }
    }

    pub fn weighted(
        name: impl Into<String>,
        kind: ScoreComponentKind,
        raw_value: f64,
        normalized_value: f64,
        weight: f64,
        evidence_window: impl Into<String>,
    ) -> Self {
        let raw_value = finite_or_zero(raw_value);
        let normalized_value = finite_or_zero(normalized_value);
        let weight = finite_or_zero(weight);
        let signed_contribution = normalized_value * weight;
        Self {
            name: name.into(),
            kind,
            value: signed_contribution,
            raw_value,
            normalized_value,
            weight,
            signed_contribution,
            direction: ScoreDirection::from_contribution(signed_contribution),
            evidence_window: Some(evidence_window.into()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScoreBreakdown {
    pub version: String,
    pub symbol: String,
    pub raw_total: f64,
    pub adjusted_total: f64,
    pub confidence_score: u8,
    pub components: Vec<ScoreComponent>,
    pub unavailable_evidence: Vec<String>,
}

impl ScoreBreakdown {
    pub fn from_components(
        symbol: impl Into<String>,
        confidence_score: u8,
        components: Vec<ScoreComponent>,
    ) -> Self {
        Self::try_from_components(symbol, confidence_score, components)
            .expect("score component names must be unique")
    }

    pub fn try_from_components(
        symbol: impl Into<String>,
        confidence_score: u8,
        components: Vec<ScoreComponent>,
    ) -> HlsResult<Self> {
        validate_unique_names(&components)?;
        let raw_total = clamp_score(
            components
                .iter()
                .map(|component| component.signed_contribution)
                .sum(),
        );
        let confidence_score = confidence_score.min(100);
        let adjusted_total = clamp_score(raw_total * (f64::from(confidence_score) / 100.0));

        Ok(Self {
            version: "score_breakdown.v1".to_owned(),
            symbol: symbol.into(),
            raw_total,
            adjusted_total,
            confidence_score,
            components,
            unavailable_evidence: Vec::new(),
        })
    }

    pub fn with_unavailable_evidence(mut self, unavailable_evidence: Vec<String>) -> Self {
        unavailable_evidence
            .into_iter()
            .filter(|evidence| !evidence.trim().is_empty())
            .for_each(|evidence| self.unavailable_evidence.push(evidence));
        self.unavailable_evidence.sort();
        self.unavailable_evidence.dedup();
        self
    }

    pub fn confidence_penalty(&self) -> f64 {
        self.adjusted_total - self.raw_total
    }

    pub fn component(&self, name: &str) -> Option<&ScoreComponent> {
        self.components
            .iter()
            .find(|component| component.name == name)
    }
}

fn validate_unique_names(components: &[ScoreComponent]) -> HlsResult<()> {
    let mut names = HashSet::new();
    for component in components {
        if component.name.trim().is_empty() {
            return Err(HlsError::Config(
                "score component name cannot be empty".to_owned(),
            ));
        }
        if !names.insert(component.name.clone()) {
            return Err(HlsError::Config(format!(
                "duplicate score component '{}'",
                component.name
            )));
        }
    }
    Ok(())
}

fn clamp_score(value: f64) -> f64 {
    if !value.is_finite() {
        return 0.0;
    }
    value.clamp(0.0, 100.0)
}

fn finite_or_zero(value: f64) -> f64 {
    if value.is_finite() { value } else { 0.0 }
}
