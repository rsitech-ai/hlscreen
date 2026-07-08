use hls_core::market_state::{FeatureSnapshot, StalenessState};
use hls_screen::{ScreenEngine, ScreenRequest};

use crate::theme::{bottom_border, divider, panel_line, top_border};

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
    output.push_str(&panel_line("HLSCREEN", title, "READ-ONLY"));
    output.push_str(&divider());
    output.push_str(&panel_line(
        "DATA",
        &format!(
            "public spot market data only | rows {} | fresh {} | stale {} | incomplete {}",
            rows.len(),
            stats.fresh,
            stats.stale,
            stats.incomplete
        ),
        "LOCAL",
    ));
    output.push_str(&bottom_border());
    output.push_str(
        "SYMBOL        STATE         PRICE         SPREAD     TOB DEPTH       IMBAL     RET 1M    SCORE      AGE\n",
    );
    output.push_str(
        "────────────  ────────────  ────────────  ─────────  ────────────  ─────────  ─────────  ───────  ───────\n",
    );

    for row in rows {
        output.push_str(&format!(
            "{:<12}  {:<12}  {:>12}  {:>9}  {:>12}  {:>9}  {:>9}  {:>7}  {:>7}\n",
            row.symbol,
            format_state(&row.staleness_state),
            format_optional(row.price, 4),
            format_bps(row.spread_bps),
            format_usd(row.tob_depth_usd),
            format_imbalance(row.tob_imbalance),
            format_percent(row.ret_1m),
            format!("{:.2}", row.liquidity_score),
            format_age(row.updated_ms_ago),
        ));
    }

    output.push_str(
        "\nRead-only screen: public spot market data only. Scores are heuristics, not trading signals.\n",
    );

    output
}

struct TableStats {
    fresh: usize,
    stale: usize,
    incomplete: usize,
}

impl TableStats {
    fn from_rows(rows: &[FeatureSnapshot]) -> Self {
        let fresh = rows
            .iter()
            .filter(|row| row.staleness_state == StalenessState::Fresh)
            .count();

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
        StalenessState::Incomplete => "○ incomplete",
    }
}
