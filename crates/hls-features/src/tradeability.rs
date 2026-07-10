use hls_core::{
    confidence::ConfidenceLevel,
    fees::FeeProfile,
    market_state::{
        FeeAwareTradeabilitySnapshot, LiquidityResilienceState, StalenessState, TradeabilityState,
    },
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

#[derive(Clone, Debug)]
pub struct FeeAwareTradeabilityInput<'a> {
    pub spread_bps: Option<f64>,
    pub base_state: TradeabilityState,
    pub profile: &'a FeeProfile,
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

pub fn classify_fee_aware_tradeability(
    input: FeeAwareTradeabilityInput<'_>,
) -> Option<FeeAwareTradeabilitySnapshot> {
    let spread_bps = input.spread_bps?;
    if !spread_bps.is_finite() {
        return None;
    }

    let expected_round_trip_cost_bps = spread_bps + input.profile.round_trip_blended_cost_bps();
    let (state, reason) = if !matches!(input.base_state, TradeabilityState::Tradeable) {
        (
            input.base_state,
            format!("base_tradeability_{}", input.base_state.as_str()),
        )
    } else if expected_round_trip_cost_bps <= input.profile.max_tradeable_round_trip_bps() {
        (
            TradeabilityState::Tradeable,
            "within_tradeable_fee_threshold".to_owned(),
        )
    } else {
        (
            TradeabilityState::Costly,
            "fee_cost_exceeds_tradeable_threshold".to_owned(),
        )
    };

    Some(FeeAwareTradeabilitySnapshot {
        profile_name: input.profile.name.clone(),
        state,
        expected_round_trip_cost_bps,
        maker_fee_bps: input.profile.maker_fee_bps(),
        taker_fee_bps: input.profile.taker_fee_bps(),
        taker_fill_ratio: input.profile.taker_fill_ratio(),
        slippage_buffer_bps: input.profile.slippage_buffer_bps(),
        max_tradeable_round_trip_bps: input.profile.max_tradeable_round_trip_bps(),
        reason,
    })
}
