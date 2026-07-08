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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScoreComponent {
    pub name: String,
    pub kind: ScoreComponentKind,
    pub value: f64,
}

impl ScoreComponent {
    pub fn new(name: impl Into<String>, kind: ScoreComponentKind, value: f64) -> Self {
        Self {
            name: name.into(),
            kind,
            value,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScoreBreakdown {
    pub symbol: String,
    pub raw_total: f64,
    pub adjusted_total: f64,
    pub confidence_score: u8,
    pub components: Vec<ScoreComponent>,
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
        let raw_total = clamp_score(components.iter().map(|component| component.value).sum());
        let confidence_score = confidence_score.min(100);
        let adjusted_total = clamp_score(raw_total * (f64::from(confidence_score) / 100.0));

        Ok(Self {
            symbol: symbol.into(),
            raw_total,
            adjusted_total,
            confidence_score,
            components,
        })
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
