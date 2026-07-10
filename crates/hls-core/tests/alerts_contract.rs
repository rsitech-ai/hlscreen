use hls_core::alerts::{
    AlertAction, AlertComparisonOp, AlertCondition, AlertField, AlertPlaybook, AlertRule,
    AlertSeverity,
};

#[test]
fn alert_playbook_contract_is_local_only_and_serializable() {
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

    playbook.validate().expect("playbook is valid");
    let encoded = serde_json::to_string(&playbook).expect("playbook serializes");

    assert!(encoded.contains("spread-shock-watch"));
    assert!(encoded.contains("local_only"));
    assert!(!encoded.contains("order"));
    assert!(!encoded.contains("wallet"));
    assert!(!encoded.contains("exchange"));
}

#[test]
fn alert_playbook_rejects_empty_rules_and_non_local_actions() {
    let empty = AlertPlaybook {
        schema_version: 1,
        id: "empty".to_owned(),
        description: "Invalid.".to_owned(),
        rules: Vec::new(),
    };
    assert!(
        empty
            .validate()
            .expect_err("empty rules rejected")
            .to_string()
            .contains("at least one rule")
    );

    let mut playbook = AlertPlaybook {
        schema_version: 1,
        id: "unsafe".to_owned(),
        description: "Invalid.".to_owned(),
        rules: vec![AlertRule {
            id: "unsafe-action".to_owned(),
            description: "Invalid.".to_owned(),
            severity: AlertSeverity::Critical,
            condition: AlertCondition::SpreadShockAndLowConfidence {
                min_spread_shock_bps: 25.0,
                max_confidence_score: 80,
            },
            cooldown_ms: 1_000,
            source_interval_ms: 30_000,
            action: AlertAction::ExchangeAction,
        }],
    };

    assert!(
        playbook
            .validate()
            .expect_err("exchange actions rejected")
            .to_string()
            .contains("local-only")
    );

    playbook.rules[0].action = AlertAction::LocalOnly;
    playbook.rules[0].condition = AlertCondition::SpreadShockAndLowConfidence {
        min_spread_shock_bps: 0.0,
        max_confidence_score: 80,
    };
    assert!(
        playbook
            .validate()
            .expect_err("invalid threshold rejected")
            .to_string()
            .contains("positive")
    );
}

#[test]
fn alert_playbook_contract_supports_typed_threshold_grammar() {
    let playbook = AlertPlaybook {
        schema_version: 1,
        id: "threshold-watch".to_owned(),
        description: "Local alert for typed threshold stacks.".to_owned(),
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

    playbook.validate().expect("threshold grammar is valid");
    let encoded = serde_json::to_string(&playbook).expect("playbook serializes");

    assert!(encoded.contains("field_threshold"));
    assert!(encoded.contains("spread_shock_bps"));
    assert!(encoded.contains("confidence_score"));
    assert!(encoded.contains("local_only"));
    assert!(!encoded.contains("wallet"));
    assert!(!encoded.contains("exchange_action"));
}

#[test]
fn alert_playbook_contract_supports_boolean_grammar() {
    let playbook = AlertPlaybook {
        schema_version: 1,
        id: "boolean-watch".to_owned(),
        description: "Local alert for boolean condition stacks.".to_owned(),
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

    playbook.validate().expect("boolean grammar is valid");
    let encoded = serde_json::to_string(&playbook).expect("playbook serializes");

    assert!(encoded.contains("\"type\":\"any\""));
    assert!(encoded.contains("\"type\":\"not\""));
    assert!(encoded.contains("local_only"));
    assert!(!encoded.contains("wallet"));
    assert!(!encoded.contains("exchange_action"));
}

#[test]
fn alert_playbook_rejects_invalid_threshold_grammar() {
    let mut playbook = AlertPlaybook {
        schema_version: 1,
        id: "invalid-threshold-watch".to_owned(),
        description: "Invalid threshold grammar.".to_owned(),
        rules: vec![AlertRule {
            id: "empty-all".to_owned(),
            description: "Invalid.".to_owned(),
            severity: AlertSeverity::Watch,
            condition: AlertCondition::All { conditions: vec![] },
            cooldown_ms: 60_000,
            source_interval_ms: 30_000,
            action: AlertAction::LocalOnly,
        }],
    };

    assert!(
        playbook
            .validate()
            .expect_err("empty all rejected")
            .to_string()
            .contains("at least one child")
    );

    playbook.rules[0].condition = AlertCondition::FieldThreshold {
        field: AlertField::SpreadShockBps,
        op: AlertComparisonOp::Gte,
        value: f64::NAN,
    };
    assert!(
        playbook
            .validate()
            .expect_err("non-finite threshold rejected")
            .to_string()
            .contains("finite")
    );

    playbook.rules[0].condition = AlertCondition::SpreadShockAndLowConfidence {
        min_spread_shock_bps: f64::NAN,
        max_confidence_score: 70,
    };
    assert!(
        playbook
            .validate()
            .expect_err("non-finite spread shock rejected")
            .to_string()
            .contains("finite")
    );
}

#[test]
fn alert_playbook_rejects_invalid_boolean_grammar() {
    let playbook = AlertPlaybook {
        schema_version: 1,
        id: "invalid-boolean-watch".to_owned(),
        description: "Invalid boolean grammar.".to_owned(),
        rules: vec![AlertRule {
            id: "empty-any".to_owned(),
            description: "Invalid.".to_owned(),
            severity: AlertSeverity::Watch,
            condition: AlertCondition::Any { conditions: vec![] },
            cooldown_ms: 60_000,
            source_interval_ms: 30_000,
            action: AlertAction::LocalOnly,
        }],
    };

    assert!(
        playbook
            .validate()
            .expect_err("empty any rejected")
            .to_string()
            .contains("at least one child")
    );
}
