use hls_core::market_state::{FeatureSnapshot, LiveMarketState, StalenessState, SymbolMarketState};

use crate::{
    formulas::{bounded_score, spread_bps, tob_depth_usd, tob_imbalance},
    windows::{
        latest_candle_trade_count_z, latest_candle_volume_z, window_realized_volatility_since,
        window_return_since,
    },
};

const ONE_MINUTE_MS: u64 = 60_000;
const FIVE_MINUTES_MS: u64 = 5 * ONE_MINUTE_MS;
const ONE_HOUR_MS: u64 = 60 * ONE_MINUTE_MS;

#[derive(Clone, Debug)]
pub struct FeatureEngine {
    stale_after_ms: i64,
}

impl Default for FeatureEngine {
    fn default() -> Self {
        Self {
            stale_after_ms: 10_000,
        }
    }
}

impl FeatureEngine {
    pub fn snapshots(&self, state: &LiveMarketState, now_ms: i64) -> Vec<FeatureSnapshot> {
        let mut snapshots: Vec<_> = state
            .states()
            .map(|symbol_state| self.snapshot(symbol_state, now_ms))
            .collect();
        snapshots.sort_by(|left, right| left.symbol.cmp(&right.symbol));
        snapshots
    }

    pub fn snapshot(&self, state: &SymbolMarketState, now_ms: i64) -> FeatureSnapshot {
        let spread_bps = match (state.bid_px, state.ask_px) {
            (Some(bid), Some(ask)) => spread_bps(bid, ask),
            _ => None,
        };
        let tob_depth_usd = match (state.bid_px, state.bid_sz, state.ask_px, state.ask_sz) {
            (Some(bid_px), Some(bid_sz), Some(ask_px), Some(ask_sz)) => {
                Some(tob_depth_usd(bid_px, bid_sz, ask_px, ask_sz))
            }
            _ => None,
        };
        let tob_imbalance = match (state.bid_px, state.bid_sz, state.ask_px, state.ask_sz) {
            (Some(bid_px), Some(bid_sz), Some(ask_px), Some(ask_sz)) => {
                tob_imbalance(bid_px, bid_sz, ask_px, ask_sz)
            }
            _ => None,
        };
        let ret_1m = window_return_since(&state.trades, now_ms, ONE_MINUTE_MS);
        let ret_5m = window_return_since(&state.trades, now_ms, FIVE_MINUTES_MS);
        let ret_1h = window_return_since(&state.trades, now_ms, ONE_HOUR_MS);
        let rv_1m = window_realized_volatility_since(&state.trades, now_ms, ONE_MINUTE_MS);
        let rv_5m = window_realized_volatility_since(&state.trades, now_ms, FIVE_MINUTES_MS);
        let rv_1h = window_realized_volatility_since(&state.trades, now_ms, ONE_HOUR_MS);
        let volume_z_1h = latest_candle_volume_z(&state.candles);
        let trade_count_z_1h = latest_candle_trade_count_z(&state.candles);
        let updated_ms_ago = state.last_update_ms.map(|last| now_ms.saturating_sub(last));
        let staleness_state = match updated_ms_ago {
            Some(age) if age <= self.stale_after_ms => StalenessState::Fresh,
            Some(_) => StalenessState::Stale,
            None => StalenessState::Incomplete,
        };
        let incomplete_window_reason = if state.trades.len() < 2 {
            Some("need at least two trades for return windows".to_owned())
        } else {
            None
        };
        let liquidity_score = bounded_score(tob_depth_usd.unwrap_or_default() / 100.0);
        let score_return = ret_5m.or(ret_1m).or(ret_1h).unwrap_or_default();
        let momentum_score = bounded_score(50.0 + score_return * 100.0);
        let mean_reversion_score = bounded_score(50.0 - score_return * 100.0);

        FeatureSnapshot {
            symbol: state.hl_coin.clone(),
            price: state.last_trade_price.or(state.mid_px).or(state.mark_px),
            mid_px: state.mid_px,
            mark_px: state.mark_px,
            day_ntl_vlm: state.day_ntl_vlm,
            bid_px: state.bid_px,
            bid_sz: state.bid_sz,
            ask_px: state.ask_px,
            ask_sz: state.ask_sz,
            spread_bps,
            tob_depth_usd,
            tob_imbalance,
            ret_1m,
            ret_5m,
            ret_1h,
            rv_1m,
            rv_5m,
            rv_1h,
            volume_z_1h,
            trade_count_z_1h,
            liquidity_score,
            momentum_score,
            mean_reversion_score,
            updated_ms_ago,
            staleness_state,
            incomplete_window_reason,
        }
    }
}
