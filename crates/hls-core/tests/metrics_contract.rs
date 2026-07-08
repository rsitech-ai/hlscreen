use hls_core::metrics::{MetricDefinition, MetricKind, MetricsRegistry};

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
