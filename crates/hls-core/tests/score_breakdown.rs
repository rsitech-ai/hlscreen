use hls_core::score::{ScoreBreakdown, ScoreComponent, ScoreComponentKind, ScoreDirection};

#[test]
fn score_breakdown_sums_named_components_and_applies_confidence() {
    let breakdown = ScoreBreakdown::from_components(
        "@107",
        80,
        vec![
            ScoreComponent::new("liquidity", ScoreComponentKind::Liquidity, 45.0),
            ScoreComponent::new("momentum", ScoreComponentKind::Momentum, 20.0),
            ScoreComponent::new("spread_cost", ScoreComponentKind::SpreadCost, -5.0),
        ],
    );

    assert_eq!(breakdown.symbol, "@107");
    assert_eq!(breakdown.raw_total, 60.0);
    assert_eq!(breakdown.adjusted_total, 48.0);
    assert_eq!(breakdown.confidence_score, 80);
    assert_eq!(breakdown.confidence_penalty(), -12.0);
    let liquidity = breakdown.component("liquidity").expect("liquidity");
    assert_eq!(liquidity.signed_contribution, 45.0);
    assert_eq!(liquidity.direction, ScoreDirection::Positive);
}

#[test]
fn score_breakdown_clamps_totals_to_screen_score_bounds() {
    let high = ScoreBreakdown::from_components(
        "@107",
        100,
        vec![ScoreComponent::new(
            "oversized",
            ScoreComponentKind::Custom,
            150.0,
        )],
    );
    let low = ScoreBreakdown::from_components(
        "@107",
        100,
        vec![ScoreComponent::new(
            "negative",
            ScoreComponentKind::Custom,
            -150.0,
        )],
    );

    assert_eq!(high.raw_total, 100.0);
    assert_eq!(high.adjusted_total, 100.0);
    assert_eq!(low.raw_total, 0.0);
    assert_eq!(low.adjusted_total, 0.0);
}

#[test]
fn duplicate_component_names_are_rejected() {
    let err = ScoreBreakdown::try_from_components(
        "@107",
        100,
        vec![
            ScoreComponent::new("flow", ScoreComponentKind::SignedFlow, 10.0),
            ScoreComponent::new("flow", ScoreComponentKind::SignedFlow, 5.0),
        ],
    )
    .expect_err("duplicate component names should fail");

    assert!(err.to_string().contains("duplicate score component"));
}

#[test]
fn weighted_components_explain_direction_and_unavailable_evidence() {
    let breakdown = ScoreBreakdown::from_components(
        "@107",
        50,
        vec![
            ScoreComponent::weighted(
                "liquidity_resilience",
                ScoreComponentKind::Resilience,
                5_000.0,
                60.0,
                0.40,
                "top_of_book",
            ),
            ScoreComponent::weighted(
                "spread_cost",
                ScoreComponentKind::SpreadCost,
                75.0,
                -12.0,
                1.0,
                "bbo_latest",
            ),
        ],
    )
    .with_unavailable_evidence(vec![
        "metadata.cohort_tag".to_owned(),
        "metadata.cohort_tag".to_owned(),
    ]);

    assert_eq!(breakdown.version, "score_breakdown.v1");
    assert_eq!(breakdown.raw_total, 12.0);
    assert_eq!(breakdown.adjusted_total, 6.0);
    assert_eq!(breakdown.confidence_penalty(), -6.0);
    assert_eq!(
        breakdown
            .component("spread_cost")
            .expect("spread cost")
            .direction,
        ScoreDirection::Negative
    );
    assert_eq!(
        breakdown.unavailable_evidence,
        vec!["metadata.cohort_tag".to_owned()]
    );
}

#[test]
fn score_breakdown_fixture_matches_serialized_contract() {
    let breakdown: ScoreBreakdown = serde_json::from_str(include_str!(
        "../../../tests/fixtures/microstructure/explainable_scores.json"
    ))
    .expect("score fixture parses");

    assert_eq!(breakdown.symbol, "@107");
    assert_eq!(breakdown.raw_total, 45.0);
    assert_eq!(breakdown.adjusted_total, 36.0);
    assert_eq!(breakdown.confidence_score, 80);
    assert_eq!(breakdown.components.len(), 3);
    assert_eq!(
        breakdown
            .component("spread_cost")
            .expect("spread")
            .direction,
        ScoreDirection::Negative
    );
    assert_eq!(
        breakdown.unavailable_evidence,
        vec!["metadata.cohort_tag".to_owned()]
    );
}
