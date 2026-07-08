use hls_core::score::{ScoreBreakdown, ScoreComponent, ScoreComponentKind};

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
    assert!(breakdown.component("liquidity").is_some());
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
