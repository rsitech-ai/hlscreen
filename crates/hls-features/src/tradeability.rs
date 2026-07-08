use hls_core::{
    confidence::ConfidenceLevel,
    market_state::{LiquidityResilienceState, StalenessState, TradeabilityState},
};

const TRADEABLE_MAX_SPREAD_BPS: f64 = 25.0;
const COSTLY_SPREAD_BPS: f64 = 50.0;
const MIN_TRADEABLE_DEPTH_USD: f64 = 5_000.0;
const THIN_DEPTH_USD: f64 = 1_000.0;

#[derive(Clone, Debug)]
pub struct TradeabilityInput {
    pub spread_bps: Option<f64>,
    pub tob_depth_usd: Option<f64>,
    pub confidence_level: ConfidenceLevel,
    pub staleness_state: StalenessState,
    pub resilience_state: LiquidityResilienceState,
}

pub fn classify_tradeability(input: TradeabilityInput) -> TradeabilityState {
    let (Some(spread_bps), Some(tob_depth_usd)) = (input.spread_bps, input.tob_depth_usd) else {
        return TradeabilityState::Unknown;
    };

    if !spread_bps.is_finite() || !tob_depth_usd.is_finite() {
        return TradeabilityState::Unknown;
    }
    if matches!(input.resilience_state, LiquidityResilienceState::Unknown) {
        return TradeabilityState::Unknown;
    }
    if !matches!(input.staleness_state, StalenessState::Fresh)
        || matches!(input.confidence_level, ConfidenceLevel::Untrusted)
    {
        return TradeabilityState::Stale;
    }
    if tob_depth_usd < THIN_DEPTH_USD {
        return TradeabilityState::Thin;
    }
    if spread_bps >= COSTLY_SPREAD_BPS
        || matches!(
            input.resilience_state,
            LiquidityResilienceState::Shock
                | LiquidityResilienceState::Recovering
                | LiquidityResilienceState::Brittle
        )
    {
        return TradeabilityState::Costly;
    }
    if spread_bps <= TRADEABLE_MAX_SPREAD_BPS
        && tob_depth_usd >= MIN_TRADEABLE_DEPTH_USD
        && matches!(
            input.confidence_level,
            ConfidenceLevel::High | ConfidenceLevel::Medium
        )
    {
        return TradeabilityState::Tradeable;
    }

    TradeabilityState::Costly
}
