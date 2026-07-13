use hls_core::metrics::{
    MetricDefinition, MetricKind, MetricSupport, MetricsRegistry, MicrostructureMetricDefinition,
    operations_metrics_snapshot,
};
use hls_core::telemetry::OperationsTelemetry;

#[test]
fn metrics_registry_accepts_low_cardinality_labels() {
    let registry = MetricsRegistry::new(vec![
        MetricDefinition::new(
            "hls_ws_messages_total",
            MetricKind::Counter,
            "Public WebSocket messages by channel.",
            ["channel"],
        ),
        MetricDefinition::new(
            "hls_feature_latency_us",
            MetricKind::Histogram,
            "Local feature computation latency.",
            std::iter::empty::<&str>(),
        ),
    ]);

    registry.validate().expect("metrics registry is valid");
}

#[test]
fn metrics_registry_rejects_high_cardinality_labels() {
    let registry = MetricsRegistry::new(vec![MetricDefinition::new(
        "hls_parse_latency_us",
        MetricKind::Histogram,
        "Parse latency with unsafe labels.",
        ["symbol", "run_id"],
    )]);

    let err = registry
        .validate()
        .expect_err("high-cardinality labels must be rejected");

    assert!(err.to_string().contains("high-cardinality label"));
}

#[test]
fn metrics_registry_requires_hls_prefixed_metric_names() {
    let registry = MetricsRegistry::new(vec![MetricDefinition::new(
        "parse_latency_us",
        MetricKind::Histogram,
        "Missing project prefix.",
        ["channel"],
    )]);

    let err = registry
        .validate()
        .expect_err("metric names must use the project prefix");

    assert!(err.to_string().contains("must start with hls_"));
}

#[test]
fn microstructure_metric_definitions_capture_canonical_proxy_and_unavailable_states() {
    let definitions = [
        MicrostructureMetricDefinition::canonical(
            "amihud_1m",
            "abs(return_1m) / dollar_volume_1m",
            "return_per_usd",
            ["trades"],
        ),
        MicrostructureMetricDefinition::proxy(
            "bbo_ofi_proxy_30s",
            "sum(best_level_queue_delta_notional)",
            "usd_notional",
            ["bbo"],
            "top-of-book proxy, not full depth OFI",
        ),
        MicrostructureMetricDefinition::proxy(
            "signed_flow_toxicity_proxy_30s",
            "abs(sum(signed_notional_30s)) / sum(abs(notional_30s))",
            "ratio",
            ["trades"],
            "public trade signed-flow concentration proxy, not canonical toxicity",
        ),
        MicrostructureMetricDefinition::unavailable(
            "l2_queue_position",
            "queue_position_at_price_level",
            "contracts",
            ["l2_book"],
            "public v1 feed does not record full order-book queue position",
        ),
    ];

    for definition in definitions {
        definition.validate().expect("definition is valid");
    }
}

#[test]
fn microstructure_metric_definitions_fail_closed_on_unsafe_or_vague_contracts() {
    let missing_caveat = MicrostructureMetricDefinition {
        name: "roll_effective_spread".to_owned(),
        formula: "2 * sqrt(max(0, -cov(delta_price_t, delta_price_t_minus_1)))".to_owned(),
        unit: "price".to_owned(),
        required_inputs: vec!["trades".to_owned()],
        support: MetricSupport::Proxy,
        caveat: None,
    };
    assert!(
        missing_caveat
            .validate()
            .expect_err("proxy metrics require caveats")
            .to_string()
            .contains("proxy metric")
    );

    let private_input = MicrostructureMetricDefinition::canonical(
        "private_fee_edge",
        "account_fee_tier_adjusted_edge",
        "bps",
        ["private_account"],
    );
    assert!(
        private_input
            .validate()
            .expect_err("private inputs are not allowed")
            .to_string()
            .contains("private")
    );

    let duplicate_input = MicrostructureMetricDefinition::canonical(
        "amihud_1m",
        "formula",
        "unit",
        ["trades", "trades"],
    );
    assert!(
        duplicate_input
            .validate()
            .expect_err("inputs must be unique")
            .to_string()
            .contains("repeats input")
    );
}

#[test]
fn operations_metrics_cover_recovery_and_data_quality_without_symbol_labels() {
    let snapshot = operations_metrics_snapshot(
        1_000,
        &OperationsTelemetry {
            reconnect_attempts: 4,
            parser_drops: 2,
            stale_duration_ms: 1_500,
            repair_latency_ms: Some(720),
            unrepaired_gap_duration_ms: 8_000,
        },
    )
    .expect("operations metrics are valid");

    let expected = [
        "hls_reconnect_attempts_total",
        "hls_parser_drops_total",
        "hls_stale_duration_ms_total",
        "hls_repair_latency_ms",
        "hls_unrepaired_gap_duration_ms",
    ];
    for name in expected {
        let sample = snapshot
            .samples
            .iter()
            .find(|sample| sample.name == name)
            .unwrap_or_else(|| panic!("missing {name}"));
        assert!(sample.labels.is_empty(), "{name} must not carry labels");
    }
    assert!(
        snapshot
            .prometheus_text
            .contains("hls_parser_drops_total 2")
    );
    assert!(
        snapshot
            .prometheus_text
            .contains("hls_repair_latency_ms 720")
    );
}

#[test]
fn operations_metrics_omit_unknown_repair_latency() {
    let snapshot = operations_metrics_snapshot(1_000, &OperationsTelemetry::default())
        .expect("operations metrics are valid");

    assert!(
        snapshot
            .samples
            .iter()
            .all(|sample| sample.name != "hls_repair_latency_ms")
    );
}
