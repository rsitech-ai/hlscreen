use hls_core::{
    market_state::{AdverseSelectionProxy, SymbolMarketState, TradeEvent, TradeSide},
    metrics::MicrostructureMetricSnapshot,
};

use crate::{formulas::percent_return, resilience::bbo_ofi_proxy_30s};

const ONE_MINUTE_MS: i64 = 60_000;
const FIVE_MINUTES_MS: i64 = 5 * ONE_MINUTE_MS;

pub fn microstructure_metric_snapshots(
    state: &SymbolMarketState,
    now_ms: i64,
    bbo_ofi: Option<f64>,
    adverse_selection_proxy: AdverseSelectionProxy,
) -> Vec<MicrostructureMetricSnapshot> {
    vec![
        amihud_1m(state, now_ms),
        roll_effective_spread(state, now_ms),
        bipower_variation_5m(state, now_ms),
        bbo_ofi_metric(state, now_ms, bbo_ofi),
        signed_flow_toxicity_proxy_30s(state, now_ms),
        adverse_selection_metric(adverse_selection_proxy),
    ]
}

fn amihud_1m(state: &SymbolMarketState, now_ms: i64) -> MicrostructureMetricSnapshot {
    let trades = trades_in_window(&state.trades, now_ms, ONE_MINUTE_MS);
    if trades.len() < 2 {
        return MicrostructureMetricSnapshot::unavailable(
            "amihud_1m",
            "return_per_usd",
            "need at least two trades in the 1m public trade window",
        );
    }

    let Some(return_1m) = percent_return(
        trades.first().expect("nonempty").price,
        trades.last().expect("nonempty").price,
    ) else {
        return MicrostructureMetricSnapshot::unavailable(
            "amihud_1m",
            "return_per_usd",
            "trade prices must be finite and positive",
        );
    };
    let dollar_volume = trades.iter().map(|trade| trade.notional).sum::<f64>();
    if dollar_volume <= 0.0 || !dollar_volume.is_finite() {
        return MicrostructureMetricSnapshot::unavailable(
            "amihud_1m",
            "return_per_usd",
            "1m public trade notional must be positive",
        );
    }

    MicrostructureMetricSnapshot::proxy(
        "amihud_1m",
        return_1m.abs() / dollar_volume,
        "return_per_usd",
        "bounded public-trade Amihud-style proxy; not a canonical production estimate",
    )
}

fn roll_effective_spread(state: &SymbolMarketState, now_ms: i64) -> MicrostructureMetricSnapshot {
    let trades = trades_in_window(&state.trades, now_ms, FIVE_MINUTES_MS);
    if trades.len() < 4 {
        return MicrostructureMetricSnapshot::unavailable(
            "roll_effective_spread",
            "price",
            "need at least four trades for adjacent price-change covariance",
        );
    }

    let price_changes: Vec<f64> = trades
        .windows(2)
        .map(|pair| pair[1].price - pair[0].price)
        .collect();
    let adjacent_products: Vec<f64> = price_changes
        .windows(2)
        .map(|pair| pair[0] * pair[1])
        .collect();
    let serial_covariance = adjacent_products.iter().sum::<f64>() / adjacent_products.len() as f64;
    if serial_covariance >= 0.0 || !serial_covariance.is_finite() {
        return MicrostructureMetricSnapshot::unavailable(
            "roll_effective_spread",
            "price",
            "non-negative adjacent price-change covariance does not support Roll spread estimate",
        );
    }

    MicrostructureMetricSnapshot::proxy(
        "roll_effective_spread",
        2.0 * (-serial_covariance).sqrt(),
        "price",
        "bounded public-trade Roll-style proxy without production sampling validation",
    )
}

fn bipower_variation_5m(state: &SymbolMarketState, now_ms: i64) -> MicrostructureMetricSnapshot {
    let trades = trades_in_window(&state.trades, now_ms, FIVE_MINUTES_MS);
    if trades.len() < 3 {
        return MicrostructureMetricSnapshot::unavailable(
            "bipower_variation_5m",
            "decimal_variance",
            "need at least three trades for adjacent absolute return products",
        );
    }

    let returns: Vec<f64> = trades
        .windows(2)
        .filter_map(|pair| percent_return(pair[0].price, pair[1].price))
        .collect();
    if returns.len() < 2 {
        return MicrostructureMetricSnapshot::unavailable(
            "bipower_variation_5m",
            "decimal_variance",
            "trade prices must be finite and positive",
        );
    }

    let bipower = std::f64::consts::FRAC_PI_2
        * returns
            .windows(2)
            .map(|pair| pair[0].abs() * pair[1].abs())
            .sum::<f64>();

    MicrostructureMetricSnapshot::proxy(
        "bipower_variation_5m",
        bipower,
        "decimal_variance",
        "bounded trade-to-trade bipower-style proxy without canonical time-bar sampling",
    )
}

fn bbo_ofi_metric(
    state: &SymbolMarketState,
    now_ms: i64,
    bbo_ofi: Option<f64>,
) -> MicrostructureMetricSnapshot {
    let value = bbo_ofi.or_else(|| bbo_ofi_proxy_30s(&state.bbo_events, now_ms));
    match value {
        Some(value) if value.is_finite() => MicrostructureMetricSnapshot::proxy(
            "bbo_ofi_proxy_30s",
            value,
            "usd_notional",
            "top-of-book proxy, not full-depth order-flow imbalance",
        ),
        _ => MicrostructureMetricSnapshot::unavailable(
            "bbo_ofi_proxy_30s",
            "usd_notional",
            "need at least two public BBO updates in the 30s window",
        ),
    }
}

fn signed_flow_toxicity_proxy_30s(
    state: &SymbolMarketState,
    now_ms: i64,
) -> MicrostructureMetricSnapshot {
    let trades = trades_in_window(&state.trades, now_ms, 30_000);
    if trades.len() < 2 {
        return MicrostructureMetricSnapshot::unavailable(
            "signed_flow_toxicity_proxy_30s",
            "ratio",
            "need at least two trades in the 30s public trade window",
        );
    }

    let mut signed_notional = 0.0;
    let mut absolute_notional = 0.0;
    for trade in trades {
        if !trade.notional.is_finite() || trade.notional <= 0.0 {
            continue;
        }
        absolute_notional += trade.notional.abs();
        signed_notional += match trade.side {
            TradeSide::Buy => trade.notional,
            TradeSide::Sell => -trade.notional,
        };
    }

    if absolute_notional <= 0.0 || !absolute_notional.is_finite() {
        return MicrostructureMetricSnapshot::unavailable(
            "signed_flow_toxicity_proxy_30s",
            "ratio",
            "30s public trade notional must be positive",
        );
    }

    MicrostructureMetricSnapshot::proxy(
        "signed_flow_toxicity_proxy_30s",
        (signed_notional.abs() / absolute_notional).clamp(0.0, 1.0),
        "ratio",
        "public trade signed-flow concentration proxy, not canonical toxicity or fill quality",
    )
}

fn adverse_selection_metric(
    adverse_selection_proxy: AdverseSelectionProxy,
) -> MicrostructureMetricSnapshot {
    match adverse_selection_proxy {
        AdverseSelectionProxy::Unknown => MicrostructureMetricSnapshot::unavailable(
            "adverse_selection_toxicity_proxy",
            "ordinal",
            "need signed flow, BBO OFI proxy, and top-of-book depth",
        ),
        AdverseSelectionProxy::Normal => MicrostructureMetricSnapshot::proxy(
            "adverse_selection_toxicity_proxy",
            0.0,
            "ordinal",
            "ordinal proxy from top-of-book resilience, signed flow, and BBO OFI",
        ),
        AdverseSelectionProxy::Watch => MicrostructureMetricSnapshot::proxy(
            "adverse_selection_toxicity_proxy",
            1.0,
            "ordinal",
            "ordinal proxy from top-of-book resilience, signed flow, and BBO OFI",
        ),
        AdverseSelectionProxy::Brittle => MicrostructureMetricSnapshot::proxy(
            "adverse_selection_toxicity_proxy",
            2.0,
            "ordinal",
            "ordinal proxy from top-of-book resilience, signed flow, and BBO OFI",
        ),
    }
}

fn trades_in_window(trades: &[TradeEvent], now_ms: i64, window_ms: i64) -> Vec<&TradeEvent> {
    let start_ms = now_ms.saturating_sub(window_ms);
    trades
        .iter()
        .filter(|trade| trade.exchange_ts_ms >= start_ms && trade.exchange_ts_ms <= now_ms)
        .collect()
}
