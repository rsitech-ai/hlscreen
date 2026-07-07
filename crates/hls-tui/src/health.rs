use hls_core::health::HealthSnapshot;

pub fn render_health_pane(snapshot: &HealthSnapshot) -> String {
    let mut output = String::from("READ-ONLY health\n");
    output.push_str(&format!("status: {}\n", snapshot.status.as_str()));
    output.push_str(&format!("read-only: {}\n", snapshot.read_only));
    output.push_str(&format!("subscriptions: {}\n", snapshot.subscription_count));
    output.push_str(&format!(
        "last message age ms: {}\n",
        optional_u64(snapshot.last_message_age_ms)
    ));
    output.push_str(&format!("lag ms: {}\n", optional_u64(snapshot.lag_ms)));
    output.push_str(&format!("writer backlog: {}\n", snapshot.writer_backlog));
    output.push_str(&format!("rows written: {}\n", snapshot.rows_written));
    output.push_str(&format!("gaps: {}\n", snapshot.gap_count));

    if !snapshot.degraded_reasons.is_empty() {
        output.push_str("reasons:\n");
        for reason in &snapshot.degraded_reasons {
            output.push_str(&format!("- {reason}\n"));
        }
    }

    output
}

fn optional_u64(value: Option<u64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".to_owned())
}
