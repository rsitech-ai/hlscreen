pub fn spread_bps(bid_px: f64, ask_px: f64) -> Option<f64> {
    if bid_px <= 0.0 || ask_px <= 0.0 || bid_px > ask_px {
        return None;
    }

    let mid = (bid_px + ask_px) / 2.0;
    Some((ask_px - bid_px) / mid * 10_000.0)
}

pub fn tob_depth_usd(bid_px: f64, bid_sz: f64, ask_px: f64, ask_sz: f64) -> f64 {
    bid_px * bid_sz + ask_px * ask_sz
}

pub fn tob_imbalance(bid_px: f64, bid_sz: f64, ask_px: f64, ask_sz: f64) -> Option<f64> {
    let bid_notional = bid_px * bid_sz;
    let ask_notional = ask_px * ask_sz;
    let total = bid_notional + ask_notional;

    if total <= 0.0 {
        return None;
    }

    Some((bid_notional - ask_notional) / total)
}

pub fn percent_return(start: f64, end: f64) -> Option<f64> {
    if start <= 0.0 || end <= 0.0 {
        return None;
    }

    Some((end - start) / start)
}

pub fn z_score(value: f64, mean: f64, std_dev: f64) -> Option<f64> {
    if std_dev <= 0.0 {
        return None;
    }

    Some((value - mean) / std_dev)
}

pub fn realized_volatility(returns: &[f64]) -> Option<f64> {
    if returns.is_empty() {
        return None;
    }

    let mean = returns.iter().sum::<f64>() / returns.len() as f64;
    let variance = returns
        .iter()
        .map(|value| {
            let diff = value - mean;
            diff * diff
        })
        .sum::<f64>()
        / returns.len() as f64;

    Some(variance.sqrt())
}

pub fn bounded_score(value: f64) -> f64 {
    value.clamp(0.0, 100.0)
}
