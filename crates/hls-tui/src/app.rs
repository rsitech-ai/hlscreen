use hls_core::market_state::{FeatureSnapshot, StalenessState};
use hls_screen::{ScreenEngine, ScreenRequest};

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
    let mut output = format!("{title}\n");
    let fresh = rows
        .iter()
        .filter(|row| row.staleness_state == StalenessState::Fresh)
        .count();
    let stale = rows
        .iter()
        .filter(|row| row.staleness_state == StalenessState::Stale)
        .count();
    let incomplete = rows
        .iter()
        .filter(|row| row.staleness_state == StalenessState::Incomplete)
        .count();
    output.push_str(&format!(
        "scope: public spot market data only | rows={} fresh={} stale={} incomplete={}\n",
        rows.len(),
        fresh,
        stale,
        incomplete
    ));
    output.push_str(
        "symbol        price       spread   TOB depth      ret 1m   score   age ms   state\n",
    );
    output.push_str(
        "------------  ----------  -------  -------------  -------  ------  -------  ----------\n",
    );

    for row in rows {
        output.push_str(&format!(
            "{:<12}  {:>10}  {:>7}  {:>13}  {:>7}  {:>6}  {:>7}  {}\n",
            row.symbol,
            format_optional(row.price, 4),
            format_optional(row.spread_bps, 1),
            format_optional(row.tob_depth_usd, 0),
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
