use hls_core::{
    health::{
        ConnectionHealth, ConnectionState, HealthInputs, HealthStatus, ReadOnlySafety,
        RecordingHealth, WriterHealth,
    },
    telemetry::{LatencySample, TelemetryWindow},
};

#[test]
fn health_snapshot_classifies_healthy_degraded_and_interrupted_states() {
    let healthy = HealthInputs {
        safety: ReadOnlySafety::read_only(),
        connection: ConnectionHealth::connected(0, 500),
        subscription_count: 24,
        last_message_age_ms: Some(500),
        lag_ms: Some(20),
        writer: WriterHealth {
            backlog: 4,
            warn_at: 100,
            rows_written: 250,
        },
        recording: RecordingHealth {
            enabled: true,
            clean_shutdown: None,
        },
        gap_count: 0,
    }
    .snapshot();

    assert_eq!(healthy.status, HealthStatus::Healthy);
    assert!(healthy.read_only);
    assert!(healthy.degraded_reasons.is_empty());

    let writer_lag = HealthInputs {
        writer: WriterHealth {
            backlog: 250,
            warn_at: 100,
            rows_written: 250,
        },
        ..HealthInputs::healthy_fixture()
    }
    .snapshot();
    assert_eq!(writer_lag.status, HealthStatus::Degraded);
    assert!(
        writer_lag
            .degraded_reasons
            .contains(&"writer backlog high".to_owned())
    );

    let interrupted = HealthInputs {
        safety: ReadOnlySafety {
            read_only: false,
            wallet_enabled: true,
            trading_enabled: false,
        },
        connection: ConnectionHealth {
            state: ConnectionState::Disconnected,
            reconnect_count: 2,
            last_reconnect_backoff_ms: Some(2_000),
            ..ConnectionHealth::connected(0, 500)
        },
        last_message_age_ms: Some(75_000),
        gap_count: 1,
        ..HealthInputs::healthy_fixture()
    }
    .snapshot();
    assert_eq!(interrupted.status, HealthStatus::Interrupted);
    assert!(!interrupted.read_only);
    assert!(
        interrupted
            .degraded_reasons
            .contains(&"read-only safety violation".to_owned())
    );
    assert!(
        interrupted
            .degraded_reasons
            .contains(&"connection disconnected".to_owned())
    );
}

#[test]
fn telemetry_window_measures_lag_percentiles() {
    let window = TelemetryWindow::from_samples(vec![
        LatencySample::new(1_000, 1_050, 1_070, 1_090),
        LatencySample::new(2_000, 2_100, 2_140, 2_180),
        LatencySample::new(3_000, 3_400, 3_430, 3_500),
    ]);

    assert_eq!(window.count(), 3);
    assert_eq!(window.data_lag_ms_p50(), Some(100));
    assert_eq!(window.data_lag_ms_p95(), Some(400));
    assert_eq!(window.feature_lag_ms_p95(), Some(430));
    assert_eq!(window.render_lag_ms_p95(), Some(500));
}
