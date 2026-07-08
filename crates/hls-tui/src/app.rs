use hls_core::market_state::{FeatureSnapshot, StalenessState};
use hls_screen::{ScreenEngine, ScreenRequest};

use crate::theme::{bottom_border, divider, panel_line, section_rule, top_border};

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
        "MODE",
        &format!("{title} | PUBLIC WS/REST | local replay ready"),
        "SAFE",
    ));
    output.push_str(&panel_line(
        "UNIVERSE",
        &format!(
            "rows {} | fresh {} ({}) | stale {} | incomplete {}",
            rows.len(),
            stats.fresh,
            format_ratio(stats.fresh, rows.len()),
            stats.stale,
            stats.incomplete
        ),
        "LOCAL",
    ));
    output.push_str(&panel_line(
        "QUALITY",
        &format!(
            "median spread {} | top depth {} | total TOB {} | top score {}",
            format_bps(stats.median_spread_bps),
            format_usd(stats.top_tob_depth_usd),
            format_usd(stats.total_tob_depth_usd),
            format_score(stats.top_liquidity_score)
        ),
        stats.quality_status(),
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

    output.push_str(
        "#   SYMBOL        STATE          PRICE       SPREAD   TOB DEPTH    IMBAL    RET 1M     RV 1M    LIQ    MOM     AGE\n",
    );
    output.push_str(
        "──  ────────────  ─────────────  ───────────  ─────────  ──────────  ───────  ────────  ────────  ─────  ─────  ──────\n",
    );

    for (index, row) in rows.iter().enumerate() {
        output.push_str(&format!(
            "{:>02}  {:<12}  {:<13}  {:>11}  {:>9}  {:>10}  {:>7}  {:>8}  {:>8}  {:>5}  {:>5}  {:>6}\n",
            index + 1,
            row.symbol,
            format_state(&row.staleness_state),
            format_optional(row.price, 4),
            format_bps(row.spread_bps),
            format_usd(row.tob_depth_usd),
            format_imbalance(row.tob_imbalance),
            format_percent(row.ret_1m),
            format_volatility(row.rv_1m),
            format!("{:.1}", row.liquidity_score),
            format!("{:.1}", row.momentum_score),
            format_age(row.updated_ms_ago),
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
        StalenessState::Fresh => "● FRESH",
        StalenessState::Stale => "▲ STALE",
        StalenessState::Incomplete => "○ INCOMPLETE",
    }
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

fn max_value(values: impl Iterator<Item = f64>) -> Option<f64> {
    values
        .filter(|value| value.is_finite())
        .max_by(f64::total_cmp)
}
