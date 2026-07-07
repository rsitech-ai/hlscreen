use hls_core::market_state::{FeatureSnapshot, StalenessState};

pub fn render_main_table(rows: &[FeatureSnapshot]) -> String {
    let mut output = String::from("READ-ONLY Hyperliquid spot live screen\n");
    output.push_str("symbol        price      spread_bps  tob_depth_usd  ret_1m    liq_score  updated_ms  state\n");

    for row in rows {
        output.push_str(&format!(
            "{:<13} {:<10} {:<11} {:<14} {:<9} {:<10} {:<11} {}\n",
            row.symbol,
            format_optional(row.price, 4),
            format_optional(row.spread_bps, 2),
            format_optional(row.tob_depth_usd, 2),
            format_percent(row.ret_1m),
            format!("{:.2}", row.liquidity_score),
            row.updated_ms_ago
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_owned()),
            format_state(&row.staleness_state),
        ));
    }

    output
}

fn format_optional(value: Option<f64>, decimals: usize) -> String {
    value.map_or_else(|| "-".to_owned(), |value| format!("{value:.decimals$}"))
}

fn format_percent(value: Option<f64>) -> String {
    value.map_or_else(|| "-".to_owned(), |value| format!("{:.2}%", value * 100.0))
}

fn format_state(state: &StalenessState) -> &'static str {
    match state {
        StalenessState::Fresh => "fresh",
        StalenessState::Stale => "stale",
        StalenessState::Incomplete => "incomplete",
    }
}
