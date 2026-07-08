use hls_core::{HlsError, HlsResult};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Field {
    Symbol,
    Price,
    MidPx,
    MarkPx,
    DayNtlVlm,
    BidPx,
    AskPx,
    SpreadBps,
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
