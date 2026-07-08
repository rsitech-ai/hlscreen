use hls_core::market_state::FeatureSnapshot;

use crate::dsl::ast::Field;

#[derive(Clone, Debug, PartialEq)]
pub enum FieldValue {
    Number(f64),
    String(String),
    Bool(bool),
    Missing,
}

impl FieldValue {
    pub fn as_sort_number(&self) -> Option<f64> {
        match self {
            Self::Number(value) if value.is_finite() => Some(*value),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ScreenRow<'a> {
    snapshot: &'a FeatureSnapshot,
}

impl<'a> ScreenRow<'a> {
    pub fn new(snapshot: &'a FeatureSnapshot) -> Self {
        Self { snapshot }
    }

    pub fn value(&self, field: Field) -> FieldValue {
        match field {
            Field::Symbol => FieldValue::String(self.snapshot.symbol.clone()),
            Field::Price => optional_number(self.snapshot.price),
            Field::MidPx => optional_number(self.snapshot.mid_px),
            Field::MarkPx => optional_number(self.snapshot.mark_px),
            Field::DayNtlVlm => optional_number(self.snapshot.day_ntl_vlm),
            Field::BidPx => optional_number(self.snapshot.bid_px),
            Field::AskPx => optional_number(self.snapshot.ask_px),
            Field::SpreadBps => optional_number(self.snapshot.spread_bps),
            Field::ConfidenceScore => FieldValue::Number(f64::from(self.snapshot.confidence.score)),
            Field::ConfidenceState => {
                FieldValue::String(self.snapshot.confidence.level.as_str().to_owned())
            }
            Field::SpreadShockBps => optional_number(self.snapshot.spread_shock_bps),
            Field::SpreadRecoveryMs => self
                .snapshot
                .spread_recovery_ms
                .map(|value| FieldValue::Number(value as f64))
                .unwrap_or(FieldValue::Missing),
            Field::ResilienceState => {
                FieldValue::String(self.snapshot.resilience_state.as_str().to_owned())
            }
            Field::TradeabilityState => {
                FieldValue::String(self.snapshot.tradeability_state.as_str().to_owned())
            }
            Field::AdverseSelectionProxy => {
                FieldValue::String(self.snapshot.adverse_selection_proxy.as_str().to_owned())
            }
            Field::SignedNotionalFlow30s => optional_number(self.snapshot.signed_notional_flow_30s),
            Field::BboOfiProxy30s => optional_number(self.snapshot.bbo_ofi_proxy_30s),
            Field::TobDepthUsd => optional_number(self.snapshot.tob_depth_usd),
            Field::TobImbalance => optional_number(self.snapshot.tob_imbalance),
            Field::Ret1m => optional_number(self.snapshot.ret_1m),
            Field::Ret5m => optional_number(self.snapshot.ret_5m),
            Field::Ret1h => optional_number(self.snapshot.ret_1h),
            Field::Rv1m => optional_number(self.snapshot.rv_1m),
            Field::Rv5m => optional_number(self.snapshot.rv_5m),
            Field::Rv1h => optional_number(self.snapshot.rv_1h),
            Field::VolumeZ1h => optional_number(self.snapshot.volume_z_1h),
            Field::TradeCountZ1h => optional_number(self.snapshot.trade_count_z_1h),
            Field::LiquidityScore => FieldValue::Number(self.snapshot.liquidity_score),
            Field::MomentumScore => FieldValue::Number(self.snapshot.momentum_score),
            Field::MeanReversionScore => FieldValue::Number(self.snapshot.mean_reversion_score),
            Field::UpdatedMsAgo => self
                .snapshot
                .updated_ms_ago
                .map(|value| FieldValue::Number(value as f64))
                .unwrap_or(FieldValue::Missing),
        }
    }
}

fn optional_number(value: Option<f64>) -> FieldValue {
    match value {
        Some(value) if value.is_finite() => FieldValue::Number(value),
        _ => FieldValue::Missing,
    }
}
