use hls_core::market_state::{CandleEvent, TradeEvent};

use crate::formulas::{percent_return, realized_volatility, z_score};

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

pub fn window_return_since(trades: &[TradeEvent], now_ms: i64, window_ms: u64) -> Option<f64> {
    let window_trades = trades_in_window(trades, now_ms, window_ms);
    if window_trades.len() < 2 {
        return None;
    }

    percent_return(window_trades.first()?.price, window_trades.last()?.price)
}

pub fn window_realized_volatility_since(
    trades: &[TradeEvent],
    now_ms: i64,
    window_ms: u64,
) -> Option<f64> {
    let window_trades = trades_in_window(trades, now_ms, window_ms);
    if window_trades.len() < 3 {
        return Some(0.0);
    }

    let returns: Vec<f64> = window_trades
        .windows(2)
        .filter_map(|pair| percent_return(pair[0].price, pair[1].price))
        .collect();

    realized_volatility(&returns)
}

pub fn latest_candle_volume_z(candles: &[CandleEvent]) -> Option<f64> {
    latest_candle_z(candles, |candle| candle.volume_base)
}

pub fn latest_candle_trade_count_z(candles: &[CandleEvent]) -> Option<f64> {
    latest_candle_z(candles, |candle| candle.trade_count as f64)
}

fn trades_in_window(trades: &[TradeEvent], now_ms: i64, window_ms: u64) -> Vec<&TradeEvent> {
    let window_ms = i64::try_from(window_ms).unwrap_or(i64::MAX);
    let start_ms = now_ms.saturating_sub(window_ms);

    trades
        .iter()
        .filter(|trade| trade.exchange_ts_ms >= start_ms && trade.exchange_ts_ms <= now_ms)
        .collect()
}

fn latest_candle_z(candles: &[CandleEvent], value: impl Fn(&CandleEvent) -> f64) -> Option<f64> {
    let latest = candles.last()?;
    let baseline = &candles[..candles.len() - 1];
    if baseline.len() < 2 {
        return Some(0.0);
    }

    let baseline_values: Vec<f64> = baseline.iter().map(&value).collect();
    let mean = baseline_values.iter().sum::<f64>() / baseline_values.len() as f64;
    let variance = baseline_values
        .iter()
        .map(|sample| {
            let diff = sample - mean;
            diff * diff
        })
        .sum::<f64>()
        / baseline_values.len() as f64;

    z_score(value(latest), mean, variance.sqrt()).or(Some(0.0))
}
