use hls_core::{
    confidence::{ConfidenceLevel, ConfidenceReason},
    market_state::{
        AdverseSelectionProxy, FeatureSnapshot, LiquidityResilienceState, StalenessState,
        TradeabilityState,
    },
    metadata::{COHORT_FRESH_LIQUIDITY, COHORT_NEW_LISTING, COHORT_UNKNOWN_METADATA},
};
use hls_screen::{ScreenEngine, ScreenRequest, presets::find_preset};

use crate::theme::truncate_chars;

const WORKSTATION_COLS: [(&str, usize); 9] = [
    ("symbol", 12),
    ("price", 9),
    ("sprbp", 7),
    ("imb", 7),
    ("flow30", 8),
    ("rv5m", 7),
    ("amihud", 6),
    ("conf", 6),
    ("why now", 17),
];

const WORKSTATION_WIDTH: usize = 1
    + (WORKSTATION_COLS[0].1 + 3)
    + (WORKSTATION_COLS[1].1 + 3)
    + (WORKSTATION_COLS[2].1 + 3)
    + (WORKSTATION_COLS[3].1 + 3)
    + (WORKSTATION_COLS[4].1 + 3)
    + (WORKSTATION_COLS[5].1 + 3)
    + (WORKSTATION_COLS[6].1 + 3)
    + (WORKSTATION_COLS[7].1 + 3)
    + (WORKSTATION_COLS[8].1 + 3);

#[derive(Clone, Copy)]
enum Align {
    Left,
    Right,
}

const WORKSTATION_ALIGNS: [Align; 9] = [
    Align::Left,
    Align::Right,
    Align::Right,
    Align::Right,
    Align::Right,
    Align::Right,
    Align::Left,
    Align::Right,
    Align::Left,
];

pub fn render_main_table(rows: &[FeatureSnapshot]) -> String {
    render_table_with_title(rows, "READ-ONLY Hyperliquid spot live screen")
}

pub fn render_confidence_summary(rows: &[FeatureSnapshot]) -> String {
    let stats = TableStats::from_rows(rows);
    format!(
        "confidence_summary=high:{} medium:{} low:{} untrusted:{} min:{} reasons:{}",
        stats.confidence_high,
        stats.confidence_medium,
        stats.confidence_low,
        stats.confidence_untrusted,
        stats
            .min_confidence_score
            .map_or_else(|| "-".to_owned(), |score| score.to_string()),
        stats.confidence_reason_count
    )
}

pub fn render_screened_table(
    rows: &[FeatureSnapshot],
    title: &str,
    request: &ScreenRequest,
) -> hls_core::HlsResult<String> {
    let rows = ScreenEngine.apply(rows, request)?;
    Ok(render_workstation(&rows, title, Some(request)))
}

pub fn render_table_with_title(rows: &[FeatureSnapshot], title: &str) -> String {
    render_workstation(rows, title, None)
}

fn render_workstation(
    rows: &[FeatureSnapshot],
    title: &str,
    request: Option<&ScreenRequest>,
) -> String {
    let stats = TableStats::from_rows(rows);
    let mut output = String::new();

    let stream_status = if title.to_ascii_lowercase().contains("replay") {
        "REPLAY ●"
    } else {
        "LIVE ●"
    };
    let recorder_status = if title.to_ascii_lowercase().contains("recording") {
        "REC ●"
    } else {
        "REC ready"
    };
    let status = format!(
        "{recorder_status}  {stream_status}  p95 local {}",
        format_age(stats.p95_age_ms)
    );

    output.push_str(&workstation_top_line(
        "Hyperliquid Spot Microstructure Workstation",
        &status,
    ));
    output.push_str(&workstation_full_line(
        &format!("filter: {}", filter_label(title, request)),
        &format!(
            "mode: {} | quality {}",
            mode_label(rows.len(), request),
            stats.quality_status().to_ascii_lowercase()
        ),
    ));
    output.push_str(&workstation_border("├", "┬", "┤"));
    output.push_str(&workstation_header_row());
    output.push_str(&workstation_border("├", "┼", "┤"));

    if rows.is_empty() {
        output.push_str(&workstation_full_line(
            "No rows matched the current read-only screen.",
            "wait for public frames or adjust filter",
        ));
        output.push_str(&workstation_border("└", "┴", "┘"));
        output.push_str(
            "\nNo wallet, no private streams, no order routes. Scores are screen heuristics, not orders or advice.\n",
        );
        return output;
    }

    for row in rows {
        output.push_str(&workstation_data_row(row));
    }

    output.push_str(&workstation_border("└", "┴", "┘"));

    let selected = &rows[0];
    output.push('\n');
    output.push_str(&format!("Selected: {}\n", selected.symbol));
    output.push_str(&format!(
        "Bid/Ask        {:<21} Micro-BBO      {:<12} Mark-Mid basis {}\n",
        format_bid_ask(selected),
        format_optional(selected.mid_px, 4),
        format_basis_bps(selected),
    ));
    output.push_str(&format!(
        "Top book       {:<21} OFI 30s        {:<12} Spread recovery {}\n",
        format_top_book(selected),
        format_signed_usd(selected.bbo_ofi_proxy_30s),
        format_recovery(selected.spread_recovery_ms),
    ));
    output.push_str(&format!(
        "Signed flow    5s:-  30s:{} 1m:-       RV 1m/5m/1h   {}\n",
        format_signed_usd(selected.signed_notional_flow_30s),
        format_volatility_compact_triplet(selected),
    ));
    output.push_str(&format!(
        "Confidence     {}\n",
        format_confidence_counters(selected)
    ));
    output.push_str(&format!(
        "Why ranked     {} | tradeability {} | resilience {}\n",
        format_why_ranked_tokens(selected),
        format_tradeability_state(selected.tradeability_state),
        format_resilience_state(selected.resilience_state),
    ));
    output.push_str(&format!(
        "Metadata       {}\n",
        format_metadata_summary(selected)
    ));

    output.push_str(
        "\nNo wallet, no private streams, no order routes. Scores are screen heuristics, not orders or advice.\n",
    );

    output
}

fn workstation_top_line(title: &str, status: &str) -> String {
    let left = format!(" {title} ");
    let right = format!(" {status} ");
    let inner_width = WORKSTATION_WIDTH - 2;
    let fill_width = inner_width.saturating_sub(char_count(&left) + char_count(&right));
    format!("┌{}{}{}┐\n", left, "─".repeat(fill_width), right)
}

fn workstation_full_line(left: &str, right: &str) -> String {
    let inner_width = WORKSTATION_WIDTH - 4;
    let right = if right.is_empty() {
        String::new()
    } else {
        format!(" {right}")
    };
    let right_width = char_count(&right);
    let left_width = inner_width.saturating_sub(right_width);
    let left_text = truncate_chars(left, left_width);
    let padding = left_width.saturating_sub(char_count(&left_text));
    format!("│ {left_text}{}{right} │\n", " ".repeat(padding))
}

fn workstation_border(left: &str, separator: &str, right: &str) -> String {
    let segments = WORKSTATION_COLS
        .iter()
        .map(|(_, width)| "─".repeat(width + 2))
        .collect::<Vec<_>>()
        .join(separator);
    format!("{left}{segments}{right}\n")
}

fn workstation_header_row() -> String {
    let cells = WORKSTATION_COLS
        .iter()
        .map(|(label, _)| (*label).to_owned())
        .collect::<Vec<_>>();
    workstation_row(&cells, true)
}

fn workstation_data_row(row: &FeatureSnapshot) -> String {
    let cells = vec![
        row.symbol.clone(),
        format_optional(row.price, 4),
        format_bps_value(row.spread_bps),
        format_imbalance_cell(row.tob_imbalance),
        format_signed_usd(row.signed_notional_flow_30s),
        format_volatility_compact(row.rv_5m),
        format_amihud_proxy(row),
        format_confidence_decimal(row),
        format_why_now(row),
    ];
    workstation_row(&cells, false)
}

fn workstation_row(cells: &[String], header: bool) -> String {
    let mut output = String::from("│");
    for (index, ((_, width), cell)) in WORKSTATION_COLS.iter().zip(cells.iter()).enumerate() {
        let align = if header {
            Align::Left
        } else {
            WORKSTATION_ALIGNS[index]
        };
        output.push(' ');
        output.push_str(&pad_cell(cell, *width, align));
        output.push(' ');
        output.push('│');
    }
    output.push('\n');
    output
}

fn pad_cell(value: &str, width: usize, align: Align) -> String {
    let value = truncate_chars(value, width);
    let padding = width.saturating_sub(char_count(&value));
    match align {
        Align::Left => format!("{value}{}", " ".repeat(padding)),
        Align::Right => format!("{}{value}", " ".repeat(padding)),
    }
}

fn filter_label(title: &str, request: Option<&ScreenRequest>) -> String {
    let Some(request) = request else {
        return title.to_owned();
    };
    match (&request.preset, &request.where_expr) {
        (Some(preset), Some(where_expr)) => format!("{where_expr}; preset {preset}"),
        (Some(preset), None) => preset.clone(),
        (None, Some(where_expr)) => where_expr.clone(),
        (None, None) => title.to_owned(),
    }
}

fn mode_label(row_count: usize, request: Option<&ScreenRequest>) -> String {
    let sort = request.and_then(|request| {
        request.sort.clone().or_else(|| {
            request
                .preset
                .as_deref()
                .and_then(find_preset)
                .map(|preset| preset.sort.to_owned())
        })
    });
    sort.map_or_else(
        || format!("top-{row_count} by screen rank"),
        |sort| format!("top-{row_count} by {}", sort.replace(':', " ")),
    )
}

fn format_bid_ask(row: &FeatureSnapshot) -> String {
    format!(
        "{} / {}",
        format_optional(row.bid_px, 4),
        format_optional(row.ask_px, 4)
    )
}

fn format_basis_bps(row: &FeatureSnapshot) -> String {
    match (row.mark_px, row.mid_px) {
        (Some(mark), Some(mid)) if mid != 0.0 => {
            format!("{:+.1} bps", ((mark / mid) - 1.0) * 10_000.0)
        }
        _ => "-".to_owned(),
    }
}

fn format_top_book(row: &FeatureSnapshot) -> String {
    format!(
        "{} / {}",
        format_usd(notional(row.bid_px, row.bid_sz)),
        format_usd(notional(row.ask_px, row.ask_sz))
    )
}

fn notional(px: Option<f64>, qty: Option<f64>) -> Option<f64> {
    match (px, qty) {
        (Some(px), Some(qty)) => Some(px * qty),
        _ => None,
    }
}

fn format_confidence_counters(row: &FeatureSnapshot) -> String {
    format!(
        "window:{} stale:{} sparse:{} reconnect:{} parser_drop:{}",
        row.confidence.incomplete_windows.len(),
        reason_count(row, ConfidenceReason::StaleQuote),
        reason_count(row, ConfidenceReason::SparseTrades),
        reason_count(row, ConfidenceReason::ReconnectGap),
        reason_count(row, ConfidenceReason::ParserDrops),
    )
}

fn reason_count(row: &FeatureSnapshot, reason: ConfidenceReason) -> usize {
    row.confidence
        .reasons
        .iter()
        .filter(|candidate| **candidate == reason)
        .count()
}

fn format_why_ranked_tokens(row: &FeatureSnapshot) -> String {
    let Some(breakdown) = &row.score_breakdown else {
        return format!("score {}", format_score_pair(row));
    };

    let mut tokens = breakdown
        .components
        .iter()
        .filter_map(|component| {
            if component.signed_contribution > 0.5 {
                Some(format!("+{}", component.name))
            } else if component.signed_contribution < -0.5 {
                Some(format!("-{}", component.name))
            } else {
                None
            }
        })
        .take(4)
        .collect::<Vec<_>>();

    if tokens.is_empty() {
        tokens.push(format!(
            "score {}",
            format_score(Some(breakdown.adjusted_total))
        ));
    }
    if !breakdown.unavailable_evidence.is_empty() {
        tokens.push(format!(
            "missing:{}",
            breakdown.unavailable_evidence.join(",")
        ));
    }

    tokens.join(" ")
}

fn format_metadata_summary(row: &FeatureSnapshot) -> String {
    format!(
        "metadata | {} | listing age {} | seeded {} | source {}",
        format_metadata_tags(row),
        format_listing_age(
            row.metadata
                .as_ref()
                .and_then(|metadata| metadata.listing_age_ms)
        ),
        format_usd(
            row.metadata
                .as_ref()
                .and_then(|metadata| metadata.seeded_usdc)
        ),
        row.metadata
            .as_ref()
            .map(|metadata| metadata.metadata_source.as_str())
            .unwrap_or("missing"),
    )
}

fn char_count(value: &str) -> usize {
    value.chars().count()
}

struct TableStats {
    stale: usize,
    incomplete: usize,
    median_spread_bps: Option<f64>,
    top_tob_depth_usd: Option<f64>,
    p95_age_ms: Option<i64>,
    confidence_high: usize,
    confidence_medium: usize,
    confidence_low: usize,
    confidence_untrusted: usize,
    min_confidence_score: Option<u8>,
    confidence_reason_count: usize,
}

impl TableStats {
    fn from_rows(rows: &[FeatureSnapshot]) -> Self {
        let depths = finite_values(rows.iter().filter_map(|row| row.tob_depth_usd));
        let ages = rows
            .iter()
            .filter_map(|row| row.updated_ms_ago)
            .collect::<Vec<_>>();

        Self {
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
            p95_age_ms: percentile_i64(ages.iter().copied(), 0.95),
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
            return "CHECK";
        }
        if self.median_spread_bps.is_none() || self.top_tob_depth_usd.is_none() {
            return "PARTIAL";
        }

        let check_quality = self.median_spread_bps.is_some_and(|spread| spread >= 100.0)
            || self.top_tob_depth_usd.is_some_and(|depth| depth < 1_000.0);
        let watch_quality = self.median_spread_bps.is_some_and(|spread| spread >= 50.0)
            || self.top_tob_depth_usd.is_some_and(|depth| depth < 5_000.0)
            || self.stale > 0;

        if check_quality {
            "CHECK"
        } else if watch_quality {
            "WATCH"
        } else {
            "GOOD"
        }
    }
}

fn format_optional(value: Option<f64>, decimals: usize) -> String {
    value.map_or_else(|| "-".to_owned(), |value| format!("{value:.decimals$}"))
}

fn format_bps_value(value: Option<f64>) -> String {
    value.map_or_else(|| "-".to_owned(), |value| format!("{value:.1}"))
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

fn format_imbalance_cell(value: Option<f64>) -> String {
    value.map_or_else(|| "-".to_owned(), |value| format!("{value:+.2}"))
}

fn format_volatility_compact(value: Option<f64>) -> String {
    value.map_or_else(|| "-".to_owned(), |value| format!("{:.2}", value * 100.0))
}

fn format_volatility_compact_triplet(row: &FeatureSnapshot) -> String {
    format!(
        "{}/{}/{}",
        format_volatility_compact(row.rv_1m),
        format_volatility_compact(row.rv_5m),
        format_volatility_compact(row.rv_1h),
    )
}

fn format_amihud_proxy(row: &FeatureSnapshot) -> String {
    match (row.spread_bps, row.tob_depth_usd) {
        (Some(spread), Some(depth))
            if row.liquidity_score >= 70.0 && spread <= 20.0 && depth >= 10_000.0 =>
        {
            "low".to_owned()
        }
        (Some(spread), Some(depth))
            if row.liquidity_score >= 20.0 && spread <= 75.0 && depth >= 1_000.0 =>
        {
            "med".to_owned()
        }
        (Some(_), Some(_)) => "high".to_owned(),
        _ => "unknown".to_owned(),
    }
}

fn format_confidence_decimal(row: &FeatureSnapshot) -> String {
    format!("{:.2}", f64::from(row.confidence.score) / 100.0)
}

fn format_score(value: Option<f64>) -> String {
    value.map_or_else(|| "-".to_owned(), |value| format!("{value:.1}"))
}

fn format_score_pair(row: &FeatureSnapshot) -> String {
    row.score_breakdown.as_ref().map_or_else(
        || format!("{:.1}/{:.1}", row.liquidity_score, row.momentum_score),
        |breakdown| format!("{:.1}/{:.1}", breakdown.adjusted_total, breakdown.raw_total),
    )
}

fn format_metadata_tags(row: &FeatureSnapshot) -> String {
    match &row.metadata {
        Some(metadata) => format!("tags {}", metadata.cohort_label()),
        None => "tags unknown_metadata".to_owned(),
    }
}

fn format_listing_age(value: Option<i64>) -> String {
    value.map_or_else(
        || "-".to_owned(),
        |value| {
            let value = value.max(0);
            if value < 60 * 60 * 1_000 {
                format!("{:.0}m", value as f64 / (60.0 * 1_000.0))
            } else if value < 48 * 60 * 60 * 1_000 {
                format!("{:.1}h", value as f64 / (60.0 * 60.0 * 1_000.0))
            } else {
                format!("{:.1}d", value as f64 / (24.0 * 60.0 * 60.0 * 1_000.0))
            }
        },
    )
}

fn format_tradeability_state(state: TradeabilityState) -> &'static str {
    state.as_str()
}

fn format_resilience_state(state: LiquidityResilienceState) -> &'static str {
    state.as_str()
}

fn format_recovery(value: Option<i64>) -> String {
    value.map_or_else(
        || "-".to_owned(),
        |value| {
            if value < 1_000 {
                format!("{value}ms")
            } else {
                format!("{:.1}s", value as f64 / 1_000.0)
            }
        },
    )
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

fn format_why_now(row: &FeatureSnapshot) -> String {
    let parts = observation_parts(row);
    if parts.is_empty() {
        "steady".to_owned()
    } else {
        parts
            .into_iter()
            .map(|part| match part.as_str() {
                "thin book" => "thin".to_owned(),
                "wide spread" => "wide".to_owned(),
                "tight spread" => "tight".to_owned(),
                "move active" => "move".to_owned(),
                "imbalanced" => "imbalance".to_owned(),
                "spread shock" => "shock".to_owned(),
                "recovering book" => "recovering".to_owned(),
                "brittle book" => "brittle".to_owned(),
                "fresh liquidity" => "fresh liq".to_owned(),
                "new listing" => "new".to_owned(),
                other => other.to_owned(),
            })
            .take(2)
            .collect::<Vec<_>>()
            .join(" + ")
    }
}

fn observation_parts(row: &FeatureSnapshot) -> Vec<String> {
    let mut parts = Vec::new();

    if matches!(row.staleness_state, StalenessState::Stale) {
        parts.push("stale feed".to_owned());
    } else if matches!(row.staleness_state, StalenessState::Incomplete) {
        parts.push("partial data".to_owned());
    }

    match row.confidence.level {
        ConfidenceLevel::Low => parts.push("low confidence".to_owned()),
        ConfidenceLevel::Untrusted => parts.push("untrusted data".to_owned()),
        ConfidenceLevel::High | ConfidenceLevel::Medium => {}
    }

    match row.tradeability_state {
        TradeabilityState::Thin => parts.push("thin tradeability".to_owned()),
        TradeabilityState::Costly => parts.push("costly tradeability".to_owned()),
        TradeabilityState::Stale => parts.push("stale tradeability".to_owned()),
        TradeabilityState::Unknown | TradeabilityState::Tradeable => {}
    }

    match row.resilience_state {
        LiquidityResilienceState::Shock => parts.push("spread shock".to_owned()),
        LiquidityResilienceState::Recovering => parts.push("recovering book".to_owned()),
        LiquidityResilienceState::Brittle => parts.push("brittle book".to_owned()),
        LiquidityResilienceState::Unknown | LiquidityResilienceState::Normal => {}
    }

    match row.adverse_selection_proxy {
        AdverseSelectionProxy::Watch => parts.push("flow watch".to_owned()),
        AdverseSelectionProxy::Brittle => parts.push("adverse proxy".to_owned()),
        AdverseSelectionProxy::Unknown | AdverseSelectionProxy::Normal => {}
    }

    if row.tob_depth_usd.is_some_and(|depth| depth < 1_000.0) {
        parts.push("thin book".to_owned());
    }
    if row.spread_bps.is_some_and(|spread| spread >= 50.0) {
        parts.push("wide spread".to_owned());
    } else if row.spread_bps.is_some_and(|spread| spread <= 10.0) {
        parts.push("tight spread".to_owned());
    }
    if row.ret_1m.is_some_and(|ret| ret.abs() >= 0.005) {
        parts.push("move active".to_owned());
    }
    if row
        .tob_imbalance
        .is_some_and(|imbalance| imbalance.abs() >= 0.4)
    {
        parts.push("imbalanced".to_owned());
    }
    if let Some(metadata) = &row.metadata {
        if metadata.has_tag(COHORT_NEW_LISTING) {
            parts.push("new listing".to_owned());
        }
        if metadata.has_tag(COHORT_FRESH_LIQUIDITY) {
            parts.push("fresh liquidity".to_owned());
        }
        if metadata.has_tag(COHORT_UNKNOWN_METADATA) {
            parts.push("metadata partial".to_owned());
        }
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

fn percentile_i64(values: impl Iterator<Item = i64>, percentile: f64) -> Option<i64> {
    let mut values: Vec<_> = values.collect();
    if values.is_empty() {
        return None;
    }
    values.sort_unstable();
    let percentile = percentile.clamp(0.0, 1.0);
    let index = ((values.len() - 1) as f64 * percentile).ceil() as usize;
    values.get(index).copied()
}

fn max_value(values: impl Iterator<Item = f64>) -> Option<f64> {
    values
        .filter(|value| value.is_finite())
        .max_by(f64::total_cmp)
}
