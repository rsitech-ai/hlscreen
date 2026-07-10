use hls_core::{
    alerts::{
        AlertAction, AlertComparisonOp, AlertCondition, AlertCooldownStatus, AlertField,
        AlertPlaybook, AlertRule, AlertSeverity,
    },
    confidence::{ConfidenceLevel, ConfidenceReason, DataConfidenceSnapshot},
    market_state::LiveMarketState,
};
use hls_features::alerts::AlertEvaluator;
use hls_features::engine::FeatureEngine;
use hls_hyperliquid::ws::parser::parse_ws_ndjson;

#[test]
fn alert_evaluator_emits_local_event_and_suppresses_cooldown_noise() {
    let mut snapshot = snapshot_from_fixture(
        include_str!("../../../tests/fixtures/microstructure/thin_brittle_book.ndjson"),
        1_710_002_014_000,
    );
    snapshot.confidence = DataConfidenceSnapshot::new("@107")
        .with_reason(ConfidenceReason::ReconnectGap)
        .with_reason(ConfidenceReason::SparseTrades);

    assert!(matches!(
        snapshot.confidence.level,
        ConfidenceLevel::Low | ConfidenceLevel::Untrusted
    ));

    let playbook = AlertPlaybook {
        schema_version: 1,
        id: "spread-shock-watch".to_owned(),
        description: "Local alert for spread shocks with degraded confidence.".to_owned(),
        rules: vec![AlertRule {
            id: "shock-low-confidence".to_owned(),
            description: "Wide spread shock while confidence is low.".to_owned(),
            severity: AlertSeverity::Watch,
            condition: AlertCondition::SpreadShockAndLowConfidence {
                min_spread_shock_bps: 250.0,
                max_confidence_score: 70,
            },
            cooldown_ms: 60_000,
            source_interval_ms: 30_000,
            action: AlertAction::LocalOnly,
        }],
    };

    let mut evaluator = AlertEvaluator::default();
    let first = evaluator
        .evaluate(&playbook, &[snapshot.clone()], 1_710_002_014_000)
        .expect("first evaluation succeeds");
    assert_eq!(first.events.len(), 1);
    assert_eq!(first.suppressed.len(), 0);
    assert_eq!(first.events[0].playbook_id, "spread-shock-watch");
    assert_eq!(first.events[0].rule_id, "shock-low-confidence");
    assert_eq!(first.events[0].symbol, "@107");
    assert_eq!(first.events[0].action, AlertAction::LocalOnly);
    assert_eq!(
        first.events[0].cooldown_status,
        AlertCooldownStatus::Emitted
    );
    assert!(first.events[0].reason.contains("spread shock"));
    assert_eq!(first.events[0].confidence_level, snapshot.confidence.level);
    assert_eq!(first.events[0].confidence_score, snapshot.confidence.score);
    assert_eq!(first.events[0].source_interval_ms, 30_000);

    let second = evaluator
        .evaluate(&playbook, &[snapshot], 1_710_002_020_000)
        .expect("second evaluation succeeds");
    assert_eq!(second.events.len(), 0);
    assert_eq!(second.suppressed.len(), 1);
    assert_eq!(second.suppressed[0].cooldown_remaining_ms, 54_000);
    assert_eq!(second.suppressed[0].action, AlertAction::LocalOnly);
}

#[test]
fn alert_evaluator_emits_typed_threshold_grammar_event() {
    let snapshot = snapshot_from_fixture(
        include_str!("../../../tests/fixtures/microstructure/thin_brittle_book.ndjson"),
        1_710_002_014_000,
    );
    let playbook = AlertPlaybook {
        schema_version: 1,
        id: "threshold-watch".to_owned(),
        description: "Local typed threshold alert.".to_owned(),
        rules: vec![AlertRule {
            id: "shock-stack".to_owned(),
            description: "Spread shock and weak confidence.".to_owned(),
            severity: AlertSeverity::Watch,
            condition: AlertCondition::All {
                conditions: vec![
                    AlertCondition::FieldThreshold {
                        field: AlertField::SpreadShockBps,
                        op: AlertComparisonOp::Gte,
                        value: 250.0,
                    },
                    AlertCondition::FieldThreshold {
                        field: AlertField::ConfidenceScore,
                        op: AlertComparisonOp::Lte,
                        value: 100.0,
                    },
                ],
            },
            cooldown_ms: 60_000,
            source_interval_ms: 30_000,
            action: AlertAction::LocalOnly,
        }],
    };

    let mut evaluator = AlertEvaluator::default();
    let evaluation = evaluator
        .evaluate(&playbook, &[snapshot], 1_710_002_014_000)
        .expect("evaluation succeeds");

    assert_eq!(evaluation.events.len(), 1);
    assert_eq!(evaluation.events[0].rule_id, "shock-stack");
    assert!(evaluation.events[0].reason.contains("spread_shock_bps"));
    assert!(evaluation.events[0].reason.contains("confidence_score"));
    assert_eq!(evaluation.events[0].action, AlertAction::LocalOnly);
}

#[test]
fn alert_evaluator_emits_boolean_grammar_event() {
    let snapshot = snapshot_from_fixture(
        include_str!("../../../tests/fixtures/microstructure/thin_brittle_book.ndjson"),
        1_710_002_014_000,
    );
    let playbook = AlertPlaybook {
        schema_version: 1,
        id: "boolean-watch".to_owned(),
        description: "Local boolean alert.".to_owned(),
        rules: vec![AlertRule {
            id: "shock-or-wide-not-low-confidence".to_owned(),
            description: "Spread shock or wide spread while confidence is not low.".to_owned(),
            severity: AlertSeverity::Watch,
            condition: AlertCondition::All {
                conditions: vec![
                    AlertCondition::Any {
                        conditions: vec![
                            AlertCondition::FieldThreshold {
                                field: AlertField::SpreadShockBps,
                                op: AlertComparisonOp::Gte,
                                value: 250.0,
                            },
                            AlertCondition::FieldThreshold {
                                field: AlertField::SpreadBps,
                                op: AlertComparisonOp::Gte,
                                value: 300.0,
                            },
                        ],
                    },
                    AlertCondition::Not {
                        condition: Box::new(AlertCondition::FieldThreshold {
                            field: AlertField::ConfidenceScore,
                            op: AlertComparisonOp::Lt,
                            value: 50.0,
                        }),
                    },
                ],
            },
            cooldown_ms: 60_000,
            source_interval_ms: 30_000,
            action: AlertAction::LocalOnly,
        }],
    };

    let mut evaluator = AlertEvaluator::default();
    let evaluation = evaluator
        .evaluate(&playbook, &[snapshot], 1_710_002_014_000)
        .expect("evaluation succeeds");

    assert_eq!(evaluation.events.len(), 1);
    assert_eq!(
        evaluation.events[0].rule_id,
        "shock-or-wide-not-low-confidence"
    );
    assert!(evaluation.events[0].reason.contains("any("));
    assert!(evaluation.events[0].reason.contains("not("));
    assert_eq!(evaluation.events[0].action, AlertAction::LocalOnly);
}

fn snapshot_from_fixture(raw: &str, now_ms: i64) -> hls_core::market_state::FeatureSnapshot {
    let events = parse_ws_ndjson(raw).expect("fixture parses");
    let mut state = LiveMarketState::new(["@107".to_owned()]);
    for event in events {
        state.apply(event).expect("event applies");
    }

    FeatureEngine::default()
        .snapshots(&state, now_ms)
        .into_iter()
        .find(|snapshot| snapshot.symbol == "@107")
        .expect("snapshot exists")
}
