use hls_core::health::HealthSnapshot;

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
    output.push_str(&panel_line(
        "INGEST",
        &format!(
            "last msg {} | lag {} | reconnects {} | gaps: {}",
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
        "STORAGE",
        &format!(
            "writer backlog: {} | rows {} | local metadata only",
            snapshot.writer_backlog, snapshot.rows_written
        ),
        if snapshot.degraded_reasons.is_empty() {
            "CLEAR"
        } else {
            "WATCH"
        },
    ));
    output.push_str(&bottom_border());

    if !snapshot.degraded_reasons.is_empty() {
        output.push_str("reasons requiring attention\n");
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
