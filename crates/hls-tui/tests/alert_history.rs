use hls_core::{
    alerts::{AlertAction, AlertCooldownStatus, AlertEvent, AlertSeverity},
    confidence::ConfidenceLevel,
    market_state::{FeatureSnapshot, LiveMarketState},
};
use hls_features::engine::FeatureEngine;
use hls_hyperliquid::ws::parser::parse_ws_ndjson;
use hls_screen::ScreenRequest;
use hls_tui::{
    alerts::{BoundedAlertHistory, MAX_TUI_ALERT_BYTES, MAX_TUI_ALERT_ROWS},
    interaction::{WorkstationAction, WorkstationPane, WorkstationUiState},
    ratatui_app::{
        RatatuiColorMode, RatatuiFrameModel, RatatuiViewport, render_ratatui_snapshot_for_test,
    },
};

#[test]
fn alert_history_is_newest_first_and_bounded_by_rows_and_bytes() {
    let mut history = BoundedAlertHistory::default();
    for index in 0..100 {
        history.push(event(
            index,
            if index == 99 {
                AlertSeverity::Critical
            } else {
                AlertSeverity::Watch
            },
            &"reason ".repeat(100),
        ));
    }

    assert!(history.len() <= MAX_TUI_ALERT_ROWS);
    assert!(history.bytes() <= MAX_TUI_ALERT_BYTES);
    assert_eq!(history.records()[0].triggered_at_ms, 99);
    assert_eq!(history.records()[0].severity, AlertSeverity::Critical);
    assert!(
        history
            .records()
            .iter()
            .all(|record| record.action == AlertAction::LocalOnly)
    );
}

#[test]
fn focused_status_pane_renders_bounded_alert_fields_and_keyboard_cursor() {
    let mut history = BoundedAlertHistory::default();
    history.push(event(
        1_710_000_000_000,
        AlertSeverity::Watch,
        "spread shock 31.0 bps",
    ));
    history.push(event(
        1_710_000_001_000,
        AlertSeverity::Critical,
        "confidence fell below 40",
    ));

    let mut state = WorkstationUiState::default();
    state.apply(WorkstationAction::FocusPane(WorkstationPane::Status), 1);
    state.apply(WorkstationAction::Down, 1);
    assert_eq!(state.status_alert_index(), 1);

    let model = RatatuiFrameModel::new(
        vec![snapshot()],
        "READ-ONLY Hyperliquid spot live screen",
        ScreenRequest::default(),
        state,
    )
    .with_alerts(history.records().iter().cloned().collect());
    let rendered = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 180,
            height: 42,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("alert status renders");

    assert!(rendered.contains("LOCAL ALERTS 2"));
    assert!(rendered.contains("critical"));
    assert!(rendered.contains("watch"));
    assert!(rendered.contains("shock-rule"));
    assert!(rendered.contains("HYPE/USDC"));
    assert!(rendered.contains("confidence fell below 40"));
    assert!(rendered.contains("spread shock 31.0 bps"));
    assert!(rendered.contains("j/k alert history"));
    assert!(rendered.contains("> watch"));
}

fn event(triggered_at_ms: i64, severity: AlertSeverity, reason: &str) -> AlertEvent {
    AlertEvent {
        playbook_id: "local-risk".to_owned(),
        rule_id: "shock-rule".to_owned(),
        symbol: "HYPE/USDC".to_owned(),
        severity,
        triggered_at_ms,
        reason: reason.to_owned(),
        confidence_level: ConfidenceLevel::Low,
        confidence_score: 35,
        source_interval_ms: 1_000,
        cooldown_status: AlertCooldownStatus::Emitted,
        action: AlertAction::LocalOnly,
    }
}

fn snapshot() -> FeatureSnapshot {
    let events = parse_ws_ndjson(include_str!(
        "../../../tests/fixtures/hyperliquid/ws_mock_live.ndjson"
    ))
    .expect("fixture parses");
    let mut state = LiveMarketState::new(["@107".to_owned()]);
    for event in events {
        state.apply(event).expect("event applies");
    }
    FeatureEngine::default()
        .snapshots(&state, 1_710_000_066_000)
        .into_iter()
        .next()
        .expect("snapshot exists")
}
