use std::collections::HashMap;

use hls_core::{
    HlsResult,
    alerts::{
        AlertAction, AlertCondition, AlertCooldownStatus, AlertEvaluation, AlertEvent, AlertKey,
        AlertPlaybook, SuppressedAlert,
    },
    market_state::FeatureSnapshot,
};

#[derive(Clone, Debug, Default)]
pub struct AlertEvaluator {
    last_emitted_ms: HashMap<AlertKey, i64>,
}

impl AlertEvaluator {
    pub fn remember_emission(&mut self, key: AlertKey, emitted_at_ms: i64) {
        self.last_emitted_ms
            .entry(key)
            .and_modify(|existing| *existing = (*existing).max(emitted_at_ms))
            .or_insert(emitted_at_ms);
    }

    pub fn evaluate(
        &mut self,
        playbook: &AlertPlaybook,
        snapshots: &[FeatureSnapshot],
        now_ms: i64,
    ) -> HlsResult<AlertEvaluation> {
        playbook.validate()?;
        let mut evaluation = AlertEvaluation::default();

        for rule in &playbook.rules {
            for snapshot in snapshots {
                let Some(reason) = condition_reason(&rule.condition, snapshot) else {
                    continue;
                };
                let key = AlertKey::new(&playbook.id, &rule.id, &snapshot.symbol);
                if let Some(last_emitted_ms) = self.last_emitted_ms.get(&key) {
                    let elapsed = now_ms.saturating_sub(*last_emitted_ms).max(0);
                    if elapsed < rule.cooldown_ms {
                        evaluation.suppressed.push(SuppressedAlert {
                            playbook_id: playbook.id.clone(),
                            rule_id: rule.id.clone(),
                            symbol: snapshot.symbol.clone(),
                            severity: rule.severity,
                            attempted_at_ms: now_ms,
                            reason,
                            confidence_level: snapshot.confidence.level,
                            confidence_score: snapshot.confidence.score,
                            source_interval_ms: rule.source_interval_ms,
                            cooldown_status: AlertCooldownStatus::Suppressed,
                            cooldown_remaining_ms: rule.cooldown_ms - elapsed,
                            action: AlertAction::LocalOnly,
                        });
                        continue;
                    }
                }

                self.last_emitted_ms.insert(key, now_ms);
                evaluation.events.push(AlertEvent {
                    playbook_id: playbook.id.clone(),
                    rule_id: rule.id.clone(),
                    symbol: snapshot.symbol.clone(),
                    severity: rule.severity,
                    triggered_at_ms: now_ms,
                    reason,
                    confidence_level: snapshot.confidence.level,
                    confidence_score: snapshot.confidence.score,
                    source_interval_ms: rule.source_interval_ms,
                    cooldown_status: AlertCooldownStatus::Emitted,
                    action: AlertAction::LocalOnly,
                });
            }
        }

        Ok(evaluation)
    }
}

fn condition_reason(condition: &AlertCondition, snapshot: &FeatureSnapshot) -> Option<String> {
    match condition {
        AlertCondition::SpreadShockAndLowConfidence {
            min_spread_shock_bps,
            max_confidence_score,
        } => {
            let spread_shock = snapshot.spread_shock_bps?;
            if spread_shock >= *min_spread_shock_bps
                && snapshot.confidence.score <= *max_confidence_score
            {
                Some(format!(
                    "spread shock {:.1} bps with confidence {} ({})",
                    spread_shock,
                    snapshot.confidence.score,
                    snapshot.confidence.level.as_str()
                ))
            } else {
                None
            }
        }
        AlertCondition::FieldThreshold { field, op, value } => {
            let actual = alert_field_value(*field, snapshot)?;
            if compare_threshold(actual, *op, *value) {
                Some(format!(
                    "{} {:.4} {} {:.4}",
                    field.as_str(),
                    actual,
                    op.as_str(),
                    value
                ))
            } else {
                None
            }
        }
        AlertCondition::All { conditions } => {
            let mut reasons = Vec::with_capacity(conditions.len());
            for condition in conditions {
                reasons.push(condition_reason(condition, snapshot)?);
            }
            Some(reasons.join("; "))
        }
        AlertCondition::Any { conditions } => {
            for condition in conditions {
                if let Some(reason) = condition_reason(condition, snapshot) {
                    return Some(format!("any({reason})"));
                }
            }
            None
        }
        AlertCondition::Not { condition } => {
            if condition_reason(condition, snapshot).is_none() {
                Some("not(condition matched false or unavailable)".to_owned())
            } else {
                None
            }
        }
    }
}

fn alert_field_value(
    field: hls_core::alerts::AlertField,
    snapshot: &FeatureSnapshot,
) -> Option<f64> {
    match field {
        hls_core::alerts::AlertField::ConfidenceScore => Some(f64::from(snapshot.confidence.score)),
        hls_core::alerts::AlertField::SpreadBps => snapshot.spread_bps,
        hls_core::alerts::AlertField::SpreadShockBps => snapshot.spread_shock_bps,
        hls_core::alerts::AlertField::TobDepthUsd => snapshot.tob_depth_usd,
        hls_core::alerts::AlertField::TobImbalance => snapshot.tob_imbalance,
        hls_core::alerts::AlertField::SignedNotionalFlow30s => snapshot.signed_notional_flow_30s,
        hls_core::alerts::AlertField::BboOfiProxy30s => snapshot.bbo_ofi_proxy_30s,
        hls_core::alerts::AlertField::Rv1m => snapshot.rv_1m,
        hls_core::alerts::AlertField::Rv5m => snapshot.rv_5m,
        hls_core::alerts::AlertField::DayNtlVlm => snapshot.day_ntl_vlm,
    }
}

fn compare_threshold(actual: f64, op: hls_core::alerts::AlertComparisonOp, value: f64) -> bool {
    match op {
        hls_core::alerts::AlertComparisonOp::Gt => actual > value,
        hls_core::alerts::AlertComparisonOp::Gte => actual >= value,
        hls_core::alerts::AlertComparisonOp::Lt => actual < value,
        hls_core::alerts::AlertComparisonOp::Lte => actual <= value,
        hls_core::alerts::AlertComparisonOp::Eq => (actual - value).abs() <= f64::EPSILON,
    }
}
