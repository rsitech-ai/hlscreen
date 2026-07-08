use hls_core::health::{ConnectionState, HealthSnapshot};

use crate::theme::{bottom_border, divider, panel_line, top_border};

pub fn render_health_pane(snapshot: &HealthSnapshot) -> String {
    let mut output = String::new();
    output.push_str(&top_border());
    output.push_str(&panel_line(
        "HLSCREEN",
        "Operations Command Center",
        &snapshot.status.as_str().to_uppercase(),
    ));
    output.push_str(&divider());
    output.push_str(&panel_line(
        "SAFETY",
        &format!(
            "read-only {} | public market data | subscriptions {} | signed actions disabled",
            snapshot.read_only, snapshot.subscription_count
        ),
        if snapshot.read_only { "PASS" } else { "FAIL" },
    ));
    let connection = snapshot.connections.first();
    output.push_str(&panel_line(
        "CONNECTION",
        &format!(
            "state {} | last msg {} | lag {} | reconnects {} | gaps: {}",
            connection
                .map(|connection| format_connection_state(connection.state))
                .unwrap_or("unknown"),
            format_ms(snapshot.last_message_age_ms),
            format_ms(snapshot.lag_ms),
            snapshot.reconnect_count,
            snapshot.gap_count
        ),
        if snapshot.degraded_reasons.is_empty() {
            "CLEAR"
        } else {
            "WATCH"
        },
    ));
    output.push_str(&panel_line(
        "RECORDER",
        &format!(
            "enabled {} | clean {} | writer backlog {}/{} | rows {}",
            snapshot.recording.enabled,
            format_clean_shutdown(snapshot.recording.clean_shutdown),
            snapshot.writer_backlog,
            snapshot.writer_warn_at,
            snapshot.rows_written
        ),
        if snapshot.degraded_reasons.is_empty() {
            "CLEAR"
        } else {
            "WATCH"
        },
    ));
    output.push_str(&panel_line(
        "RUNBOOK",
        "fail closed on writer lag | reconnect gaps visible | local metadata only",
        "READY",
    ));
    output.push_str(&bottom_border());

    if !snapshot.degraded_reasons.is_empty() {
        output.push_str("attention queue\n");
        for reason in &snapshot.degraded_reasons {
            output.push_str(&format!("  • {reason}\n"));
        }
    } else {
        output.push_str("all monitored runtime checks are clear\n");
    }

    output
}

fn format_ms(value: Option<u64>) -> String {
    value
        .map(|value| format!("{value} ms"))
        .unwrap_or_else(|| "-".to_owned())
}

fn format_connection_state(state: ConnectionState) -> &'static str {
    match state {
        ConnectionState::Disconnected => "disconnected",
        ConnectionState::Connecting => "connecting",
        ConnectionState::Connected => "connected",
        ConnectionState::Stale => "stale",
        ConnectionState::PingSent => "ping sent",
        ConnectionState::Reconnecting => "reconnecting",
    }
}

fn format_clean_shutdown(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "yes",
        Some(false) => "no",
        None => "n/a",
    }
}
