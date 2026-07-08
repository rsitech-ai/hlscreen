use hls_core::{
    confidence::{ConfidenceLevel, ConfidenceReason},
    market_state::{FeatureSnapshot, StalenessState},
};
use hls_screen::{ScreenEngine, ScreenRequest};

use crate::theme::{bottom_border, divider, panel_line, section_rule, top_border, truncate_chars};

pub fn render_main_table(rows: &[FeatureSnapshot]) -> String {
    render_table_with_title(rows, "READ-ONLY Hyperliquid spot live screen")
}

pub fn render_screened_table(
    rows: &[FeatureSnapshot],
    title: &str,
    request: &ScreenRequest,
) -> hls_core::HlsResult<String> {
    let rows = ScreenEngine.apply(rows, request)?;
    Ok(render_table_with_title(&rows, title))
}

pub fn render_table_with_title(rows: &[FeatureSnapshot], title: &str) -> String {
    let stats = TableStats::from_rows(rows);
    let mut output = String::new();
    output.push_str(&top_border());
    output.push_str(&panel_line(
        "HLSCREEN",
        "Hyperliquid Microstructure Workstation",
        "READ-ONLY",
    ));
    output.push_str(&divider());
    output.push_str(&panel_line(
        "SESSION",
        &format!("{title} | PUBLIC WS/REST | local replay ready"),
        "SAFE",
    ));
    output.push_str(&panel_line(
        "UNIVERSE",
        &format!(
            "rows {} | fresh {}/{} | stale {} | incomplete {} | coverage {}",
            rows.len(),
            stats.fresh,
            rows.len(),
            stats.stale,
            stats.incomplete,
            format_ratio(stats.fresh, rows.len()),
        ),
        "LOCAL",
    ));
    output.push_str(&panel_line(
        "QUALITY",
        &format!(
            "spread med {} | depth top {} | depth total {} | top liq {}",
            format_bps(stats.median_spread_bps),
            format_usd(stats.top_tob_depth_usd),
            format_usd(stats.total_tob_depth_usd),
            format_score(stats.top_liquidity_score)
        ),
        stats.quality_status(),
    ));
    output.push_str(&panel_line(
        "LATENCY",
        &format!(
            "age med {} | age max {} | freshness-only quality | local render",
            format_age(stats.median_age_ms),
            format_age(stats.max_age_ms),
        ),
        stats.latency_status(),
    ));
    output.push_str(&panel_line(
        "CONFIDENCE",
        &format!(
            "high {} | medium {} | low {} | untrusted {} | min {} | reasons {}",
            stats.confidence_high,
            stats.confidence_medium,
            stats.confidence_low,
            stats.confidence_untrusted,
            stats
                .min_confidence_score
                .map_or_else(|| "-".to_owned(), |score| score.to_string()),
            stats.confidence_reason_count,
        ),
        stats.confidence_status(),
    ));
    output.push_str(&bottom_border());
    output.push_str(&section_rule("MARKET BOARD"));

    if rows.is_empty() {
        output.push_str("No rows matched the current screen. Data is unchanged; adjust the read-only filter or wait for fresh public frames.\n");
        output.push_str(
            "\nNo wallet, no private streams, no order routes. Scores are screen heuristics, not orders or advice.\n",
        );
        return output;
    }

    output.push_str("#  SYMBOL        STATE      CONF    PRICE       SPRD      DEPTH     IMB     RET1M    RV1M     SCORE    AGE  OBSERVATION\n");
    output.push_str("── ────────────  ─────────  ──────  ──────────  ────────  ────────  ───────  ───────  ─────  ─────────  ───── ────────────────────────\n");

    for (index, row) in rows.iter().enumerate() {
        output.push_str(&format!(
            "{:>02} {:<12}  {:<9}  {:<6}  {:>10}  {:>8}  {:>8}  {:>7}  {:>7}  {:>5}  {:>9}  {:>5} {}\n",
            index + 1,
            row.symbol,
            format_state(&row.staleness_state),
            format_confidence_chip(row),
            format_optional(row.price, 4),
            format_bps(row.spread_bps),
            format_usd(row.tob_depth_usd),
            format_imbalance(row.tob_imbalance),
            format_percent(row.ret_1m),
            format_volatility(row.rv_1m),
            format_score_pair(row),
            format_age(row.updated_ms_ago),
            truncate_chars(&format_row_observation(row), 28),
        ));
    }

    if let Some(selected) = rows.first() {
        output.push_str(&section_rule("SELECTED SYMBOL"));
        output.push_str(&format!(
            "{} | {} | {} | mid {} | mark {}\n",
            selected.symbol,
            format_px_qty("bid", selected.bid_px, selected.bid_sz),
            format_px_qty("ask", selected.ask_px, selected.ask_sz),
            format_optional(selected.mid_px, 4),
            format_optional(selected.mark_px, 4),
        ));
        output.push_str(&format!(
            "microstructure | spread {} | imbalance {} | top depth {} | ret 1m {} | rv 1m {}\n",
            format_bps(selected.spread_bps),
            format_imbalance(selected.tob_imbalance),
            format_usd(selected.tob_depth_usd),
            format_percent(selected.ret_1m),
            format_volatility(selected.rv_1m),
        ));
        output.push_str(&format!(
            "state | {} | age {} | incomplete {} | observation {}\n",
            format_state(&selected.staleness_state),
            format_age(selected.updated_ms_ago),
            selected
                .incomplete_window_reason
                .as_deref()
                .unwrap_or("none"),
            format_observation(selected),
        ));
        output.push_str(&format!(
            "confidence | {} {} | reasons {} | incomplete windows {}\n",
            format_confidence_level(selected.confidence.level),
            selected.confidence.score,
            format_confidence_reasons(&selected.confidence.reasons),
            format_confidence_windows(&selected.confidence.incomplete_windows),
        ));
    }

    output.push_str(
        "\nNo wallet, no private streams, no order routes. Scores are screen heuristics, not orders or advice.\n",
    );

    output
}

struct TableStats {
    fresh: usize,
    stale: usize,
    incomplete: usize,
    median_spread_bps: Option<f64>,
    top_tob_depth_usd: Option<f64>,
    total_tob_depth_usd: Option<f64>,
    top_liquidity_score: Option<f64>,
    median_age_ms: Option<i64>,
    max_age_ms: Option<i64>,
    confidence_high: usize,
    confidence_medium: usize,
    confidence_low: usize,
    confidence_untrusted: usize,
    min_confidence_score: Option<u8>,
    confidence_reason_count: usize,
}

impl TableStats {
    fn from_rows(rows: &[FeatureSnapshot]) -> Self {
        let fresh = rows
            .iter()
            .filter(|row| row.staleness_state == StalenessState::Fresh)
            .count();

        let depths = finite_values(rows.iter().filter_map(|row| row.tob_depth_usd));

        Self {
            fresh,
            stale: rows
                .iter()
                .filter(|row| row.staleness_state == StalenessState::Stale)
                .count(),
            incomplete: rows
                .iter()
                .filter(|row| row.staleness_state == StalenessState::Incomplete)
                .count(),
            median_spread_bps: median(finite_values(rows.iter().filter_map(|row| row.spread_bps))),
            top_tob_depth_usd: max_value(depths.iter().copied()),
            total_tob_depth_usd: (!depths.is_empty()).then(|| depths.iter().sum()),
            top_liquidity_score: max_value(rows.iter().map(|row| row.liquidity_score)),
            median_age_ms: median_i64(rows.iter().filter_map(|row| row.updated_ms_ago)),
            max_age_ms: rows.iter().filter_map(|row| row.updated_ms_ago).max(),
            confidence_high: rows
                .iter()
                .filter(|row| row.confidence.level == ConfidenceLevel::High)
                .count(),
            confidence_medium: rows
                .iter()
                .filter(|row| row.confidence.level == ConfidenceLevel::Medium)
                .count(),
            confidence_low: rows
                .iter()
                .filter(|row| row.confidence.level == ConfidenceLevel::Low)
                .count(),
            confidence_untrusted: rows
                .iter()
                .filter(|row| row.confidence.level == ConfidenceLevel::Untrusted)
                .count(),
            min_confidence_score: rows.iter().map(|row| row.confidence.score).min(),
            confidence_reason_count: rows.iter().map(|row| row.confidence.reasons.len()).sum(),
        }
    }

    fn quality_status(&self) -> &'static str {
        if self.incomplete > 0 {
            "CHECK"
        } else if self.stale > 0 {
            "WATCH"
        } else {
            "GOOD"
        }
    }

    fn latency_status(&self) -> &'static str {
        match self.max_age_ms {
            Some(age) if age > 10_000 => "WATCH",
            Some(_) => "FAST",
            None => "CHECK",
        }
    }

    fn confidence_status(&self) -> &'static str {
        if self.confidence_untrusted > 0 {
            "BLOCK"
        } else if self.confidence_low > 0 {
            "CHECK"
        } else if self.confidence_medium > 0 || self.confidence_reason_count > 0 {
            "WATCH"
        } else {
            "GOOD"
        }
    }
}

fn format_optional(value: Option<f64>, decimals: usize) -> String {
    value.map_or_else(|| "-".to_owned(), |value| format!("{value:.decimals$}"))
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

fn format_imbalance(value: Option<f64>) -> String {
    value.map_or_else(|| "-".to_owned(), |value| format!("{:+.0}%", value * 100.0))
}

fn format_percent(value: Option<f64>) -> String {
    value.map_or_else(|| "-".to_owned(), |value| format!("{:+.2}%", value * 100.0))
}

fn format_volatility(value: Option<f64>) -> String {
    value.map_or_else(|| "-".to_owned(), |value| format!("{:.2}%", value * 100.0))
}

fn format_score(value: Option<f64>) -> String {
    value.map_or_else(|| "-".to_owned(), |value| format!("{value:.1}"))
}

fn format_score_pair(row: &FeatureSnapshot) -> String {
    format!("{:.1}/{:.1}", row.liquidity_score, row.momentum_score)
}

fn format_confidence_chip(row: &FeatureSnapshot) -> String {
    let prefix = match row.confidence.level {
        ConfidenceLevel::High => "H",
        ConfidenceLevel::Medium => "M",
        ConfidenceLevel::Low => "L",
        ConfidenceLevel::Untrusted => "U",
    };
    format!("{prefix}{:03}", row.confidence.score)
}

fn format_confidence_level(level: ConfidenceLevel) -> &'static str {
    match level {
        ConfidenceLevel::High => "high",
        ConfidenceLevel::Medium => "medium",
        ConfidenceLevel::Low => "low",
        ConfidenceLevel::Untrusted => "untrusted",
    }
}

fn format_confidence_reason(reason: ConfidenceReason) -> &'static str {
    match reason {
        ConfidenceReason::ReconnectGap => "reconnect_gap",
        ConfidenceReason::StaleQuote => "stale_quote",
        ConfidenceReason::SparseTrades => "sparse_trades",
        ConfidenceReason::DuplicateEvents => "duplicate_events",
        ConfidenceReason::ParserDrops => "parser_drops",
        ConfidenceReason::WriterBacklog => "writer_backlog",
        ConfidenceReason::IncompleteWindow => "incomplete_window",
    }
}

fn format_confidence_reasons(reasons: &[ConfidenceReason]) -> String {
    if reasons.is_empty() {
        return "none".to_owned();
    }

    reasons
        .iter()
        .map(|reason| format_confidence_reason(*reason))
        .collect::<Vec<_>>()
        .join(",")
}

fn format_confidence_windows(windows: &[String]) -> String {
    if windows.is_empty() {
        "none".to_owned()
    } else {
        windows.join(",")
    }
}

fn format_px_qty(label: &str, px: Option<f64>, qty: Option<f64>) -> String {
    match (px, qty) {
        (Some(px), Some(qty)) => format!("{label} {px:.4} x {qty:.4}"),
        (Some(px), None) => format!("{label} {px:.4} x -"),
        _ => format!("{label} -"),
    }
}

fn format_ratio(numerator: usize, denominator: usize) -> String {
    if denominator == 0 {
        return "0%".to_owned();
    }

    format!("{:.0}%", (numerator as f64 / denominator as f64) * 100.0)
}

fn format_age(value: Option<i64>) -> String {
    value.map_or_else(
        || "-".to_owned(),
        |value| {
            let value = value.max(0);
            if value < 1_000 {
                format!("{value}ms")
            } else {
                format!("{:.1}s", value as f64 / 1_000.0)
            }
        },
    )
}

fn format_state(state: &StalenessState) -> &'static str {
    match state {
        StalenessState::Fresh => "● fresh",
        StalenessState::Stale => "▲ stale",
        StalenessState::Incomplete => "○ partial",
    }
}

fn format_observation(row: &FeatureSnapshot) -> String {
    let parts = observation_parts(row);
    if parts.is_empty() {
        "steady".to_owned()
    } else {
        parts.join(" · ")
    }
}

fn format_row_observation(row: &FeatureSnapshot) -> String {
    let parts = observation_parts(row);
    if parts.is_empty() {
        "steady".to_owned()
    } else {
        parts.into_iter().take(2).collect::<Vec<_>>().join(" · ")
    }
}

fn observation_parts(row: &FeatureSnapshot) -> Vec<&'static str> {
    let mut parts = Vec::new();

    if matches!(row.staleness_state, StalenessState::Stale) {
        parts.push("stale feed");
    } else if matches!(row.staleness_state, StalenessState::Incomplete) {
        parts.push("partial data");
    }

    match row.confidence.level {
        ConfidenceLevel::Low => parts.push("low confidence"),
        ConfidenceLevel::Untrusted => parts.push("untrusted data"),
        ConfidenceLevel::High | ConfidenceLevel::Medium => {}
    }

    if row.tob_depth_usd.is_some_and(|depth| depth < 1_000.0) {
        parts.push("thin book");
    }
    if row.spread_bps.is_some_and(|spread| spread >= 50.0) {
        parts.push("wide spread");
    } else if row.spread_bps.is_some_and(|spread| spread <= 10.0) {
        parts.push("tight spread");
    }
    if row.ret_1m.is_some_and(|ret| ret.abs() >= 0.005) {
        parts.push("move active");
    }
    if row
        .tob_imbalance
        .is_some_and(|imbalance| imbalance.abs() >= 0.4)
    {
        parts.push("imbalanced");
    }

    parts
}

fn finite_values(values: impl Iterator<Item = f64>) -> Vec<f64> {
    values.filter(|value| value.is_finite()).collect()
}

fn median(mut values: Vec<f64>) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(f64::total_cmp);
    let mid = values.len() / 2;
    if values.len() % 2 == 0 {
        Some((values[mid - 1] + values[mid]) / 2.0)
    } else {
        Some(values[mid])
    }
}

fn median_i64(values: impl Iterator<Item = i64>) -> Option<i64> {
    let mut values: Vec<_> = values.collect();
    if values.is_empty() {
        return None;
    }
    values.sort_unstable();
    let mid = values.len() / 2;
    if values.len() % 2 == 0 {
        Some((values[mid - 1] + values[mid]) / 2)
    } else {
        Some(values[mid])
    }
}

fn max_value(values: impl Iterator<Item = f64>) -> Option<f64> {
    values
        .filter(|value| value.is_finite())
        .max_by(f64::total_cmp)
}
