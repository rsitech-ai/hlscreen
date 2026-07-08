use hls_core::market_state::{
    AdverseSelectionProxy, LiquidityResilienceState, TopOfBookEvent, TradeEvent, TradeSide,
};

use crate::formulas::spread_bps;

const BBO_PROXY_WINDOW_MS: i64 = 30_000;
const SPREAD_SHOCK_WINDOW_MS: i64 = 60_000;
const SHOCK_ABSOLUTE_THRESHOLD_BPS: f64 = 25.0;
const SHOCK_MULTIPLE_THRESHOLD: f64 = 2.0;
const RECOVERY_MULTIPLE: f64 = 1.5;
const RECOVERY_ABSOLUTE_BPS: f64 = 10.0;
const BRITTLE_TIMEOUT_MS: i64 = 10_000;

#[derive(Clone, Debug, PartialEq)]
pub struct LiquidityResilienceMetrics {
    pub spread_shock_bps: Option<f64>,
    pub spread_recovery_ms: Option<i64>,
    pub resilience_state: LiquidityResilienceState,
    pub signed_notional_flow_30s: Option<f64>,
    pub bbo_ofi_proxy_30s: Option<f64>,
    pub adverse_selection_proxy: AdverseSelectionProxy,
}

impl Default for LiquidityResilienceMetrics {
    fn default() -> Self {
        Self {
            spread_shock_bps: None,
            spread_recovery_ms: None,
            resilience_state: LiquidityResilienceState::Unknown,
            signed_notional_flow_30s: None,
            bbo_ofi_proxy_30s: None,
            adverse_selection_proxy: AdverseSelectionProxy::Unknown,
        }
    }
}

pub fn liquidity_resilience_metrics(
    bbo_events: &[TopOfBookEvent],
    trades: &[TradeEvent],
    now_ms: i64,
    tob_depth_usd: Option<f64>,
) -> LiquidityResilienceMetrics {
    let spread_metrics = spread_resilience(bbo_events, now_ms);
    let signed_flow = signed_notional_flow_30s(trades, now_ms);
    let bbo_ofi = bbo_ofi_proxy_30s(bbo_events, now_ms);
    let adverse_selection_proxy = adverse_selection_proxy(
        spread_metrics.resilience_state,
        signed_flow,
        bbo_ofi,
        tob_depth_usd,
    );

    LiquidityResilienceMetrics {
        signed_notional_flow_30s: signed_flow,
        bbo_ofi_proxy_30s: bbo_ofi,
        adverse_selection_proxy,
        ..spread_metrics
    }
}

fn spread_resilience(bbo_events: &[TopOfBookEvent], now_ms: i64) -> LiquidityResilienceMetrics {
    let quotes = quote_spreads_in_window(bbo_events, now_ms, SPREAD_SHOCK_WINDOW_MS);
    if quotes.len() < 2 {
        return LiquidityResilienceMetrics::default();
    }

    let (max_index, max_spread, shock_ts_ms) =
        quotes
            .iter()
            .enumerate()
            .fold((0, f64::MIN, quotes[0].ts_ms), |best, (index, quote)| {
                if quote.spread_bps > best.1 {
                    (index, quote.spread_bps, quote.ts_ms)
                } else {
                    best
                }
            });

    let baseline_values: Vec<f64> = quotes[..max_index]
        .iter()
        .map(|quote| quote.spread_bps)
        .collect();
    let Some(baseline) = median(baseline_values) else {
        return LiquidityResilienceMetrics {
            spread_shock_bps: Some(0.0),
            spread_recovery_ms: None,
            resilience_state: LiquidityResilienceState::Normal,
            ..LiquidityResilienceMetrics::default()
        };
    };

    let shock_bps = (max_spread - baseline).max(0.0);
    let shock_detected = shock_bps >= SHOCK_ABSOLUTE_THRESHOLD_BPS
        && max_spread >= baseline * SHOCK_MULTIPLE_THRESHOLD;
    if !shock_detected {
        return LiquidityResilienceMetrics {
            spread_shock_bps: Some(0.0),
            spread_recovery_ms: None,
            resilience_state: LiquidityResilienceState::Normal,
            ..LiquidityResilienceMetrics::default()
        };
    }

    let latest = quotes.last().expect("quotes is not empty");
    let recovery_threshold = (baseline * RECOVERY_MULTIPLE).max(baseline + RECOVERY_ABSOLUTE_BPS);
    let spread_recovery_ms = (latest.spread_bps <= recovery_threshold)
        .then(|| latest.ts_ms.saturating_sub(shock_ts_ms).max(0));
    let elapsed_since_shock = latest.ts_ms.saturating_sub(shock_ts_ms).max(0);
    let resilience_state = match spread_recovery_ms {
        Some(_) => LiquidityResilienceState::Normal,
        None if elapsed_since_shock > BRITTLE_TIMEOUT_MS => LiquidityResilienceState::Brittle,
        None if latest.spread_bps < max_spread => LiquidityResilienceState::Recovering,
        None => LiquidityResilienceState::Shock,
    };

    LiquidityResilienceMetrics {
        spread_shock_bps: Some(shock_bps),
        spread_recovery_ms,
        resilience_state,
        ..LiquidityResilienceMetrics::default()
    }
}

pub fn signed_notional_flow_30s(trades: &[TradeEvent], now_ms: i64) -> Option<f64> {
    let start_ms = now_ms.saturating_sub(BBO_PROXY_WINDOW_MS);
    let mut saw_trade = false;
    let flow = trades
        .iter()
        .filter(|trade| trade.exchange_ts_ms >= start_ms && trade.exchange_ts_ms <= now_ms)
        .map(|trade| {
            saw_trade = true;
            match trade.side {
                TradeSide::Buy => trade.notional,
                TradeSide::Sell => -trade.notional,
            }
        })
        .sum::<f64>();

    saw_trade.then_some(flow)
}

pub fn bbo_ofi_proxy_30s(bbo_events: &[TopOfBookEvent], now_ms: i64) -> Option<f64> {
    let quotes = quotes_in_window(bbo_events, now_ms, BBO_PROXY_WINDOW_MS);
    if quotes.len() < 2 {
        return None;
    }

    Some(
        quotes
            .windows(2)
            .map(|pair| quote_pair_ofi_proxy(pair[0], pair[1]))
            .sum(),
    )
}

fn adverse_selection_proxy(
    resilience_state: LiquidityResilienceState,
    signed_flow: Option<f64>,
    bbo_ofi: Option<f64>,
    tob_depth_usd: Option<f64>,
) -> AdverseSelectionProxy {
    let (Some(signed_flow), Some(bbo_ofi), Some(depth)) = (signed_flow, bbo_ofi, tob_depth_usd)
    else {
        return AdverseSelectionProxy::Unknown;
    };

    let depth = depth.max(1.0);
    let flow_pressure = signed_flow.abs() / depth;
    let diverges = signed_flow.signum() != 0.0
        && bbo_ofi.signum() != 0.0
        && signed_flow.signum() != bbo_ofi.signum();

    if matches!(
        resilience_state,
        LiquidityResilienceState::Brittle | LiquidityResilienceState::Shock
    ) && flow_pressure >= 1.0
    {
        AdverseSelectionProxy::Brittle
    } else if (diverges && flow_pressure >= 0.1) || flow_pressure >= 0.5 {
        AdverseSelectionProxy::Watch
    } else {
        AdverseSelectionProxy::Normal
    }
}

#[derive(Clone, Copy)]
struct QuoteSpread {
    ts_ms: i64,
    spread_bps: f64,
}

fn quote_spreads_in_window(
    bbo_events: &[TopOfBookEvent],
    now_ms: i64,
    window_ms: i64,
) -> Vec<QuoteSpread> {
    quotes_in_window(bbo_events, now_ms, window_ms)
        .into_iter()
        .filter_map(|event| {
            spread_bps(event.bid_price?, event.ask_price?).map(|spread_bps| QuoteSpread {
                ts_ms: event.exchange_ts_ms,
                spread_bps,
            })
        })
        .collect()
}

fn quotes_in_window(
    bbo_events: &[TopOfBookEvent],
    now_ms: i64,
    window_ms: i64,
) -> Vec<&TopOfBookEvent> {
    let start_ms = now_ms.saturating_sub(window_ms);
    bbo_events
        .iter()
        .filter(|event| event.exchange_ts_ms >= start_ms && event.exchange_ts_ms <= now_ms)
        .collect()
}

fn quote_pair_ofi_proxy(previous: &TopOfBookEvent, current: &TopOfBookEvent) -> f64 {
    let bid = match (
        previous.bid_price,
        previous.bid_size,
        current.bid_price,
        current.bid_size,
    ) {
        (Some(prev_px), Some(_), Some(cur_px), Some(cur_sz)) if cur_px > prev_px => cur_px * cur_sz,
        (Some(prev_px), Some(prev_sz), Some(cur_px), Some(cur_sz)) if cur_px == prev_px => {
            cur_px * (cur_sz - prev_sz)
        }
        (Some(prev_px), Some(prev_sz), Some(cur_px), Some(_)) if cur_px < prev_px => {
            -(prev_px * prev_sz)
        }
        _ => 0.0,
    };

    let ask = match (
        previous.ask_price,
        previous.ask_size,
        current.ask_price,
        current.ask_size,
    ) {
        (Some(prev_px), Some(_), Some(cur_px), Some(cur_sz)) if cur_px < prev_px => {
            -(cur_px * cur_sz)
        }
        (Some(prev_px), Some(prev_sz), Some(cur_px), Some(cur_sz)) if cur_px == prev_px => {
            -(cur_px * (cur_sz - prev_sz))
        }
        (Some(prev_px), Some(prev_sz), Some(cur_px), Some(_)) if cur_px > prev_px => {
            prev_px * prev_sz
        }
        _ => 0.0,
    };

    bid + ask
}

fn median(mut values: Vec<f64>) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(f64::total_cmp);
    let mid = values.len() / 2;
    if values.len().is_multiple_of(2) {
        Some((values[mid - 1] + values[mid]) / 2.0)
    } else {
        Some(values[mid])
    }
}
