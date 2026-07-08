use hls_core::{
    confidence::ConfidenceReason, market_state::FeatureSnapshot, score::ScoreComponent,
};

use crate::theme::{bottom_border, divider, panel_line, section_rule, top_border, truncate_chars};

pub fn render_why_ranked_pane(row: &FeatureSnapshot) -> String {
    let mut output = String::new();
    output.push_str(&top_border());
    output.push_str(&panel_line(
        "WHY RANKED",
        &format!(
            "{} score explanation | read-only screen heuristic",
            row.symbol
        ),
        "DETAIL",
    ));
    output.push_str(&divider());

    let Some(breakdown) = &row.score_breakdown else {
        output.push_str(&panel_line(
            "SCORE",
            "unavailable | row has no generated score breakdown",
            "CHECK",
        ));
        output.push_str(&bottom_border());
        output.push_str(
            "\nNo score explanation is available for this row. This is still read-only market data, not advice.\n",
        );
        return output;
    };

    output.push_str(&panel_line(
        "TOTAL",
        &format!(
            "adjusted {} | raw {} | confidence {} | penalty {}",
            format_score(breakdown.adjusted_total),
            format_score(breakdown.raw_total),
            breakdown.confidence_score,
            format_signed_score(breakdown.confidence_penalty()),
        ),
        score_status(breakdown.adjusted_total),
    ));
    output.push_str(&panel_line(
        "QUALITY",
        &format!(
            "confidence {} {} | reasons {} | incomplete {}",
            row.confidence.level.as_str(),
            row.confidence.score,
            format_confidence_reasons(&row.confidence.reasons),
            format_windows(&row.confidence.incomplete_windows),
        ),
        confidence_status(row.confidence.score),
    ));
    output.push_str(&bottom_border());
    output.push_str(&section_rule("COMPONENTS"));
    output.push_str("NAME                      KIND             DIR       RAW        NORM     WEIGHT   CONTRIB   WINDOW\n");
    output.push_str("────────────────────────  ───────────────  ────────  ─────────  ───────  ───────  ────────  ───────────────\n");
    for component in &breakdown.components {
        output.push_str(&format_component(component));
    }

    output.push_str(&section_rule("EVIDENCE"));
    if breakdown.unavailable_evidence.is_empty() {
        output.push_str("unavailable evidence | none\n");
    } else {
        output.push_str(&format!(
            "unavailable evidence | {}\n",
            breakdown.unavailable_evidence.join(", ")
        ));
    }
    output.push_str(&format!(
        "public evidence | spread {} | top depth {} | signed flow 30s {} | BBO OFI 30s {}\n",
        format_bps(row.spread_bps),
        format_usd(row.tob_depth_usd),
        format_signed_usd(row.signed_notional_flow_30s),
        format_signed_usd(row.bbo_ofi_proxy_30s),
    ));
    output.push_str(&format!(
        "caveat | BBO/top-of-book proxy only | no fill model | screen heuristic, not advice | version {}\n",
        breakdown.version,
    ));

    output
}

fn format_component(component: &ScoreComponent) -> String {
    format!(
        "{:<24}  {:<15}  {:<8}  {:>9}  {:>7}  {:>7}  {:>8}  {}\n",
        truncate_chars(&component.name, 24),
        component.kind.as_str(),
        component.direction.as_str(),
        format_raw(component.raw_value),
        format_score(component.normalized_value),
        format!("{:.2}x", component.weight),
        format_signed_score(component.signed_contribution),
        component.evidence_window.as_deref().unwrap_or("-"),
    )
}

fn format_raw(value: f64) -> String {
    let abs = value.abs();
    if abs >= 1_000_000.0 {
        format!("{:.1}M", value / 1_000_000.0)
    } else if abs >= 1_000.0 {
        format!("{:.1}K", value / 1_000.0)
    } else if abs < 1.0 && value != 0.0 {
        format!("{value:.4}")
    } else {
        format!("{value:.1}")
    }
}

fn format_score(value: f64) -> String {
    if value.is_finite() {
        format!("{value:.1}")
    } else {
        "-".to_owned()
    }
}

fn format_signed_score(value: f64) -> String {
    if !value.is_finite() {
        return "-".to_owned();
    }
    if value >= 0.0 {
        format!("+{value:.1}")
    } else {
        format!("{value:.1}")
    }
}

fn format_bps(value: Option<f64>) -> String {
    value.map_or_else(|| "-".to_owned(), |value| format!("{value:.1} bps"))
}

fn format_usd(value: Option<f64>) -> String {
    value.map_or_else(
        || "-".to_owned(),
        |value| {
            let abs = value.abs();
            if abs >= 1_000_000_000.0 {
                format!("${:.1}B", value / 1_000_000_000.0)
            } else if abs >= 1_000_000.0 {
                format!("${:.1}M", value / 1_000_000.0)
            } else if abs >= 1_000.0 {
                format!("${:.1}K", value / 1_000.0)
            } else {
                format!("${value:.0}")
            }
        },
    )
}

fn format_signed_usd(value: Option<f64>) -> String {
    value.map_or_else(
        || "-".to_owned(),
        |value| {
            let sign = if value >= 0.0 { "+" } else { "-" };
            let formatted = format_usd(Some(value.abs()));
            format!("{sign}{formatted}")
        },
    )
}

fn format_confidence_reasons(reasons: &[ConfidenceReason]) -> String {
    if reasons.is_empty() {
        return "none".to_owned();
    }

    reasons
        .iter()
        .map(|reason| match reason {
            ConfidenceReason::ReconnectGap => "reconnect_gap",
            ConfidenceReason::StaleQuote => "stale_quote",
            ConfidenceReason::SparseTrades => "sparse_trades",
            ConfidenceReason::DuplicateEvents => "duplicate_events",
            ConfidenceReason::ParserDrops => "parser_drops",
            ConfidenceReason::WriterBacklog => "writer_backlog",
            ConfidenceReason::IncompleteWindow => "incomplete_window",
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn format_windows(windows: &[String]) -> String {
    if windows.is_empty() {
        "none".to_owned()
    } else {
        windows.join(",")
    }
}

fn score_status(score: f64) -> &'static str {
    if score >= 70.0 {
        "STRONG"
    } else if score >= 40.0 {
        "WATCH"
    } else {
        "LOW"
    }
}

fn confidence_status(score: u8) -> &'static str {
    if score >= 90 {
        "HIGH"
    } else if score >= 70 {
        "MED"
    } else if score >= 40 {
        "LOW"
    } else {
        "BLOCK"
    }
}
