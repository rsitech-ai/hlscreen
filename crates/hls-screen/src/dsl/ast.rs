use hls_core::{HlsError, HlsResult};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Field {
    Symbol,
    Price,
    MidPx,
    MarkPx,
    DayNtlVlm,
    BidPx,
    AskPx,
    SpreadBps,
    ConfidenceScore,
    ConfidenceState,
    SpreadShockBps,
    SpreadRecoveryMs,
    ResilienceState,
    TradeabilityState,
    AdverseSelectionProxy,
    SignedNotionalFlow30s,
    BboOfiProxy30s,
    TobDepthUsd,
    TobImbalance,
    Ret1m,
    Ret5m,
    Ret1h,
    Rv1m,
    Rv5m,
    Rv1h,
    VolumeZ1h,
    TradeCountZ1h,
    LiquidityScore,
    MomentumScore,
    MeanReversionScore,
    ScoreTotal,
    ScoreRawTotal,
    ScoreConfidencePenalty,
    ScoreComponent(String),
    MetadataState,
    MetadataSource,
    MetadataFetchedAtMs,
    ListingAgeMs,
    Deployer,
    DeployTimeMs,
    SeededUsdc,
    MaxSupply,
    CirculatingSupply,
    CohortTag,
    UpdatedMsAgo,
}

impl Field {
    pub fn parse(input: &str) -> HlsResult<Self> {
        match input {
            "symbol" => Ok(Self::Symbol),
            "price" => Ok(Self::Price),
            "mid_px" => Ok(Self::MidPx),
            "mark_px" => Ok(Self::MarkPx),
            "day_ntl_vlm" => Ok(Self::DayNtlVlm),
            "bid_px" => Ok(Self::BidPx),
            "ask_px" => Ok(Self::AskPx),
            "spread_bps" => Ok(Self::SpreadBps),
            "confidence_score" => Ok(Self::ConfidenceScore),
            "confidence_state" => Ok(Self::ConfidenceState),
            "spread_shock_bps" => Ok(Self::SpreadShockBps),
            "spread_recovery_ms" => Ok(Self::SpreadRecoveryMs),
            "resilience_state" => Ok(Self::ResilienceState),
            "tradeability_state" => Ok(Self::TradeabilityState),
            "adverse_selection_proxy" => Ok(Self::AdverseSelectionProxy),
            "signed_notional_flow_30s" => Ok(Self::SignedNotionalFlow30s),
            "bbo_ofi_proxy_30s" => Ok(Self::BboOfiProxy30s),
            "tob_depth_usd" => Ok(Self::TobDepthUsd),
            "tob_imbalance" => Ok(Self::TobImbalance),
            "ret_1m" => Ok(Self::Ret1m),
            "ret_5m" => Ok(Self::Ret5m),
            "ret_1h" => Ok(Self::Ret1h),
            "rv_1m" => Ok(Self::Rv1m),
            "rv_5m" => Ok(Self::Rv5m),
            "rv_1h" => Ok(Self::Rv1h),
            "volume_z_1h" => Ok(Self::VolumeZ1h),
            "trade_count_z_1h" => Ok(Self::TradeCountZ1h),
            "liquidity_score" => Ok(Self::LiquidityScore),
            "momentum_score" => Ok(Self::MomentumScore),
            "mean_reversion_score" => Ok(Self::MeanReversionScore),
            "score_total" => Ok(Self::ScoreTotal),
            "score_raw_total" => Ok(Self::ScoreRawTotal),
            "score_confidence_penalty" => Ok(Self::ScoreConfidencePenalty),
            component if component.starts_with("score_component.") => {
                let name = component.trim_start_matches("score_component.");
                if name.trim().is_empty() {
                    return Err(HlsError::Config(
                        "score component field requires a component name".to_owned(),
                    ));
                }
                Ok(Self::ScoreComponent(name.to_owned()))
            }
            "metadata_state" => Ok(Self::MetadataState),
            "metadata_source" => Ok(Self::MetadataSource),
            "metadata_fetched_at_ms" => Ok(Self::MetadataFetchedAtMs),
            "listing_age_ms" => Ok(Self::ListingAgeMs),
            "deployer" => Ok(Self::Deployer),
            "deploy_time_ms" => Ok(Self::DeployTimeMs),
            "seeded_usdc" => Ok(Self::SeededUsdc),
            "max_supply" => Ok(Self::MaxSupply),
            "circulating_supply" => Ok(Self::CirculatingSupply),
            "cohort_tag" => Ok(Self::CohortTag),
            "updated_ms_ago" => Ok(Self::UpdatedMsAgo),
            other => Err(HlsError::Config(format!("unknown field '{other}'"))),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CmpOp {
    Gt,
    Gte,
    Lt,
    Lte,
    Eq,
    Ne,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ValueExpr {
    Field(Field),
    Number(f64),
    String(String),
    Bool(bool),
    Abs(Field),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Compare {
        left: ValueExpr,
        op: CmpOp,
        right: ValueExpr,
    },
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SortDirection {
    Asc,
    Desc,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SortField {
    pub value: ValueExpr,
    pub direction: SortDirection,
}
