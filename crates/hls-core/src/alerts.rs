use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::{HlsError, HlsResult, confidence::ConfidenceLevel};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertSeverity {
    Info,
    Watch,
    Critical,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertAction {
    LocalOnly,
    ExchangeAction,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AlertCondition {
    SpreadShockAndLowConfidence {
        min_spread_shock_bps: f64,
        max_confidence_score: u8,
    },
    FieldThreshold {
        field: AlertField,
        op: AlertComparisonOp,
        value: f64,
    },
    All {
        conditions: Vec<AlertCondition>,
    },
    Any {
        conditions: Vec<AlertCondition>,
    },
    Not {
        condition: Box<AlertCondition>,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertField {
    ConfidenceScore,
    SpreadBps,
    SpreadShockBps,
    TobDepthUsd,
    TobImbalance,
    SignedNotionalFlow30s,
    BboOfiProxy30s,
    Rv1m,
    Rv5m,
    DayNtlVlm,
}

impl AlertField {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ConfidenceScore => "confidence_score",
            Self::SpreadBps => "spread_bps",
            Self::SpreadShockBps => "spread_shock_bps",
            Self::TobDepthUsd => "tob_depth_usd",
            Self::TobImbalance => "tob_imbalance",
            Self::SignedNotionalFlow30s => "signed_notional_flow_30s",
            Self::BboOfiProxy30s => "bbo_ofi_proxy_30s",
            Self::Rv1m => "rv_1m",
            Self::Rv5m => "rv_5m",
            Self::DayNtlVlm => "day_ntl_vlm",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertComparisonOp {
    Gt,
    Gte,
    Lt,
    Lte,
    Eq,
}

impl AlertComparisonOp {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Gt => ">",
            Self::Gte => ">=",
            Self::Lt => "<",
            Self::Lte => "<=",
            Self::Eq => "==",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AlertRule {
    pub id: String,
    pub description: String,
    pub severity: AlertSeverity,
    pub condition: AlertCondition,
    pub cooldown_ms: i64,
    pub source_interval_ms: i64,
    pub action: AlertAction,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AlertPlaybook {
    pub schema_version: u32,
    pub id: String,
    pub description: String,
    pub rules: Vec<AlertRule>,
}

impl AlertPlaybook {
    pub fn validate(&self) -> HlsResult<()> {
        if self.schema_version != 1 {
            return Err(HlsError::Config(format!(
                "unsupported alert playbook schema_version {}; expected 1",
                self.schema_version
            )));
        }
        if self.id.trim().is_empty() {
            return Err(HlsError::Config("alert playbook id is required".to_owned()));
        }
        if self.rules.is_empty() {
            return Err(HlsError::Config(
                "alert playbook must define at least one rule".to_owned(),
            ));
        }

        let mut ids = HashSet::new();
        for rule in &self.rules {
            rule.validate()?;
            if !ids.insert(rule.id.clone()) {
                return Err(HlsError::Config(format!(
                    "duplicate alert rule id '{}'",
                    rule.id
                )));
            }
        }
        Ok(())
    }
}

impl AlertRule {
    pub fn validate(&self) -> HlsResult<()> {
        if self.id.trim().is_empty() {
            return Err(HlsError::Config("alert rule id is required".to_owned()));
        }
        if self.action != AlertAction::LocalOnly {
            return Err(HlsError::Config(format!(
                "alert rule '{}' must be local-only; exchange actions are not allowed",
                self.id
            )));
        }
        if self.cooldown_ms < 0 {
            return Err(HlsError::Config(format!(
                "alert rule '{}' cooldown_ms must be non-negative",
                self.id
            )));
        }
        if self.source_interval_ms <= 0 {
            return Err(HlsError::Config(format!(
                "alert rule '{}' source_interval_ms must be positive",
                self.id
            )));
        }
        self.condition.validate(&self.id)
    }
}

impl AlertCondition {
    fn validate(&self, rule_id: &str) -> HlsResult<()> {
        match self {
            Self::SpreadShockAndLowConfidence {
                min_spread_shock_bps,
                ..
            } if !min_spread_shock_bps.is_finite() || *min_spread_shock_bps <= 0.0 => {
                Err(HlsError::Config(format!(
                    "alert rule '{rule_id}' min_spread_shock_bps must be positive and finite"
                )))
            }
            Self::SpreadShockAndLowConfidence { .. } => Ok(()),
            Self::FieldThreshold { value, .. } if !value.is_finite() => Err(HlsError::Config(
                format!("alert rule '{rule_id}' field threshold value must be finite"),
            )),
            Self::FieldThreshold { .. } => Ok(()),
            Self::All { conditions } if conditions.is_empty() => Err(HlsError::Config(format!(
                "alert rule '{rule_id}' all condition must contain at least one child condition"
            ))),
            Self::All { conditions } => {
                for condition in conditions {
                    condition.validate(rule_id)?;
                }
                Ok(())
            }
            Self::Any { conditions } if conditions.is_empty() => Err(HlsError::Config(format!(
                "alert rule '{rule_id}' any condition must contain at least one child condition"
            ))),
            Self::Any { conditions } => {
                for condition in conditions {
                    condition.validate(rule_id)?;
                }
                Ok(())
            }
            Self::Not { condition } => condition.validate(rule_id),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertCooldownStatus {
    Emitted,
    Suppressed,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AlertEvent {
    pub playbook_id: String,
    pub rule_id: String,
    pub symbol: String,
    pub severity: AlertSeverity,
    pub triggered_at_ms: i64,
    pub reason: String,
    pub confidence_level: ConfidenceLevel,
    pub confidence_score: u8,
    pub source_interval_ms: i64,
    pub cooldown_status: AlertCooldownStatus,
    pub action: AlertAction,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SuppressedAlert {
    pub playbook_id: String,
    pub rule_id: String,
    pub symbol: String,
    pub severity: AlertSeverity,
    pub attempted_at_ms: i64,
    pub reason: String,
    pub confidence_level: ConfidenceLevel,
    pub confidence_score: u8,
    pub source_interval_ms: i64,
    pub cooldown_status: AlertCooldownStatus,
    pub cooldown_remaining_ms: i64,
    pub action: AlertAction,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AlertEvaluation {
    pub events: Vec<AlertEvent>,
    pub suppressed: Vec<SuppressedAlert>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AlertHistoryRecord {
    pub kind: String,
    pub playbook_id: String,
    pub rule_id: String,
    pub symbol: String,
    pub severity: AlertSeverity,
    pub reason: String,
    pub confidence_score: u8,
    pub action: AlertAction,
}

impl AlertHistoryRecord {
    pub fn from_event(event: &AlertEvent) -> Self {
        Self {
            kind: "event".to_owned(),
            playbook_id: event.playbook_id.clone(),
            rule_id: event.rule_id.clone(),
            symbol: event.symbol.clone(),
            severity: event.severity,
            reason: event.reason.clone(),
            confidence_score: event.confidence_score,
            action: event.action,
        }
    }

    pub fn from_suppressed(suppressed: &SuppressedAlert) -> Self {
        Self {
            kind: "suppressed".to_owned(),
            playbook_id: suppressed.playbook_id.clone(),
            rule_id: suppressed.rule_id.clone(),
            symbol: suppressed.symbol.clone(),
            severity: suppressed.severity,
            reason: suppressed.reason.clone(),
            confidence_score: suppressed.confidence_score,
            action: suppressed.action,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct AlertKey {
    pub playbook_id: String,
    pub rule_id: String,
    pub symbol: String,
}

impl AlertKey {
    pub fn new(playbook_id: &str, rule_id: &str, symbol: &str) -> Self {
        Self {
            playbook_id: playbook_id.to_owned(),
            rule_id: rule_id.to_owned(),
            symbol: symbol.to_owned(),
        }
    }
}
