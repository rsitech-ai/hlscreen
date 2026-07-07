use hls_core::market_state::TradeEvent;

use crate::formulas::{percent_return, realized_volatility};

pub fn window_return(trades: &[TradeEvent]) -> Option<f64> {
    let first = trades.first()?;
    let last = trades.last()?;

    percent_return(first.price, last.price)
}

pub fn window_realized_volatility(trades: &[TradeEvent]) -> Option<f64> {
    if trades.len() < 3 {
        return Some(0.0);
    }

    let returns: Vec<f64> = trades
        .windows(2)
        .filter_map(|pair| percent_return(pair[0].price, pair[1].price))
        .collect();

    realized_volatility(&returns)
}
