use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::confidence::DataConfidenceSnapshot;
use crate::metadata::MetadataEnrichment;
use crate::score::ScoreBreakdown;
use crate::{HlsError, HlsResult};

const MAX_BBO_EVENTS_PER_SYMBOL: usize = 256;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum TradeSide {
    Buy,
    Sell,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TradeEvent {
    pub recv_ts_ns: u64,
    pub exchange_ts_ms: i64,
    pub hl_coin: String,
    pub side: TradeSide,
    pub price: f64,
    pub size: f64,
    pub notional: f64,
    pub hash: String,
    pub tid: u64,
    pub unique_trade_id: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TopOfBookEvent {
    pub recv_ts_ns: u64,
    pub exchange_ts_ms: i64,
    pub hl_coin: String,
    pub bid_price: Option<f64>,
    pub bid_size: Option<f64>,
    pub bid_order_count: Option<u64>,
    pub ask_price: Option<f64>,
    pub ask_size: Option<f64>,
    pub ask_order_count: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AssetContextEvent {
    pub recv_ts_ns: u64,
    pub hl_coin: String,
    pub day_ntl_vlm: Option<f64>,
    pub prev_day_px: Option<f64>,
    pub mark_px: Option<f64>,
    pub mid_px: Option<f64>,
    pub circulating_supply: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AllMidsEvent {
    pub recv_ts_ns: u64,
    pub mids_by_hl_coin: HashMap<String, f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CandleEvent {
    pub recv_ts_ns: u64,
    pub open_ts_ms: i64,
    pub close_ts_ms: i64,
    pub hl_coin: String,
    pub interval: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume_base: f64,
    pub trade_count: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum MarketEvent {
    Trade(TradeEvent),
    TopOfBook(TopOfBookEvent),
    AssetContext(AssetContextEvent),
    AllMids(AllMidsEvent),
    Candle(CandleEvent),
}

impl MarketEvent {
    pub fn hl_coin(&self) -> Option<&str> {
        match self {
            Self::Trade(event) => Some(&event.hl_coin),
            Self::TopOfBook(event) => Some(&event.hl_coin),
            Self::AssetContext(event) => Some(&event.hl_coin),
            Self::AllMids(_) => None,
            Self::Candle(event) => Some(&event.hl_coin),
        }
    }

    pub fn with_recv_ts_ns(mut self, recv_ts_ns: u64) -> Self {
        match &mut self {
            Self::Trade(event) => event.recv_ts_ns = recv_ts_ns,
            Self::TopOfBook(event) => event.recv_ts_ns = recv_ts_ns,
            Self::AssetContext(event) => event.recv_ts_ns = recv_ts_ns,
            Self::AllMids(event) => event.recv_ts_ns = recv_ts_ns,
            Self::Candle(event) => event.recv_ts_ns = recv_ts_ns,
        }
        self
    }

    pub fn recv_ts_ns(&self) -> u64 {
        match self {
            Self::Trade(event) => event.recv_ts_ns,
            Self::TopOfBook(event) => event.recv_ts_ns,
            Self::AssetContext(event) => event.recv_ts_ns,
            Self::AllMids(event) => event.recv_ts_ns,
            Self::Candle(event) => event.recv_ts_ns,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum StalenessState {
    Fresh,
    Stale,
    Incomplete,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LiquidityResilienceState {
    Unknown,
    Normal,
    Shock,
    Recovering,
    Brittle,
}

impl LiquidityResilienceState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Normal => "normal",
            Self::Shock => "shock",
            Self::Recovering => "recovering",
            Self::Brittle => "brittle",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TradeabilityState {
    Unknown,
    Tradeable,
    Costly,
    Thin,
    Stale,
}

impl TradeabilityState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Tradeable => "tradeable",
            Self::Costly => "costly",
            Self::Thin => "thin",
            Self::Stale => "stale",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdverseSelectionProxy {
    Unknown,
    Normal,
    Watch,
    Brittle,
}

impl AdverseSelectionProxy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Normal => "normal",
            Self::Watch => "watch",
            Self::Brittle => "brittle",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FeatureSnapshot {
    pub symbol: String,
    pub confidence: DataConfidenceSnapshot,
    pub price: Option<f64>,
    pub mid_px: Option<f64>,
    pub mark_px: Option<f64>,
    pub day_ntl_vlm: Option<f64>,
    pub bid_px: Option<f64>,
    pub bid_sz: Option<f64>,
    pub ask_px: Option<f64>,
    pub ask_sz: Option<f64>,
    pub spread_bps: Option<f64>,
    pub spread_shock_bps: Option<f64>,
    pub spread_recovery_ms: Option<i64>,
    pub resilience_state: LiquidityResilienceState,
    pub tradeability_state: TradeabilityState,
    pub adverse_selection_proxy: AdverseSelectionProxy,
    pub signed_notional_flow_30s: Option<f64>,
    pub bbo_ofi_proxy_30s: Option<f64>,
    pub tob_depth_usd: Option<f64>,
    pub tob_imbalance: Option<f64>,
    pub ret_1m: Option<f64>,
    pub ret_5m: Option<f64>,
    pub ret_1h: Option<f64>,
    pub rv_1m: Option<f64>,
    pub rv_5m: Option<f64>,
    pub rv_1h: Option<f64>,
    pub volume_z_1h: Option<f64>,
    pub trade_count_z_1h: Option<f64>,
    pub liquidity_score: f64,
    pub momentum_score: f64,
    pub mean_reversion_score: f64,
    pub score_breakdown: Option<ScoreBreakdown>,
    pub metadata: Option<MetadataEnrichment>,
    pub updated_ms_ago: Option<i64>,
    pub staleness_state: StalenessState,
    pub incomplete_window_reason: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct LiveMarketState {
    symbols: HashSet<String>,
    states: HashMap<String, SymbolMarketState>,
}

impl LiveMarketState {
    pub fn new(symbols: impl IntoIterator<Item = String>) -> Self {
        let symbols: HashSet<String> = symbols.into_iter().collect();
        let states = symbols
            .iter()
            .map(|symbol| (symbol.clone(), SymbolMarketState::new(symbol.clone())))
            .collect();

        Self { symbols, states }
    }

    pub fn apply(&mut self, event: MarketEvent) -> HlsResult<()> {
        match event {
            MarketEvent::AllMids(event) => {
                let recv_ms = i64::try_from(event.recv_ts_ns / 1_000_000).unwrap_or(i64::MAX);
                for (hl_coin, mid) in event.mids_by_hl_coin {
                    if let Some(state) = self.states.get_mut(&hl_coin) {
                        state.mid_px = Some(mid);
                        if recv_ms > 0 {
                            state.last_update_ms =
                                Some(state.last_update_ms.unwrap_or(0).max(recv_ms));
                        }
                    }
                }
            }
            MarketEvent::Trade(event) => {
                self.state_mut(&event.hl_coin)?.apply_trade(event);
            }
            MarketEvent::TopOfBook(event) => {
                self.state_mut(&event.hl_coin)?.apply_top_of_book(event);
            }
            MarketEvent::AssetContext(event) => {
                self.state_mut(&event.hl_coin)?.apply_asset_context(event);
            }
            MarketEvent::Candle(event) => {
                self.state_mut(&event.hl_coin)?.apply_candle(event);
            }
        }

        Ok(())
    }

    pub fn symbol_state(&self, hl_coin: &str) -> Option<&SymbolMarketState> {
        self.states.get(hl_coin)
    }

    pub fn states(&self) -> impl Iterator<Item = &SymbolMarketState> {
        self.states.values()
    }

    fn state_mut(&mut self, hl_coin: &str) -> HlsResult<&mut SymbolMarketState> {
        if !self.symbols.contains(hl_coin) {
            self.symbols.insert(hl_coin.to_owned());
            self.states.insert(
                hl_coin.to_owned(),
                SymbolMarketState::new(hl_coin.to_owned()),
            );
        }

        self.states.get_mut(hl_coin).ok_or_else(|| {
            HlsError::Config(format!("state for symbol '{hl_coin}' was not initialized"))
        })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SymbolMarketState {
    pub hl_coin: String,
    pub last_trade_price: Option<f64>,
    pub last_trade_ts_ms: Option<i64>,
    pub bid_px: Option<f64>,
    pub bid_sz: Option<f64>,
    pub ask_px: Option<f64>,
    pub ask_sz: Option<f64>,
    pub mid_px: Option<f64>,
    pub mark_px: Option<f64>,
    pub day_ntl_vlm: Option<f64>,
    pub prev_day_px: Option<f64>,
    pub candles: Vec<CandleEvent>,
    pub trades: Vec<TradeEvent>,
    pub bbo_events: Vec<TopOfBookEvent>,
    pub duplicate_trade_count: u64,
    pub last_update_ms: Option<i64>,
}

impl SymbolMarketState {
    fn new(hl_coin: String) -> Self {
        Self {
            hl_coin,
            last_trade_price: None,
            last_trade_ts_ms: None,
            bid_px: None,
            bid_sz: None,
            ask_px: None,
            ask_sz: None,
            mid_px: None,
            mark_px: None,
            day_ntl_vlm: None,
            prev_day_px: None,
            candles: Vec::new(),
            trades: Vec::new(),
            bbo_events: Vec::new(),
            duplicate_trade_count: 0,
            last_update_ms: None,
        }
    }

    fn apply_trade(&mut self, event: TradeEvent) {
        if self
            .trades
            .iter()
            .any(|trade| trade.unique_trade_id == event.unique_trade_id)
        {
            self.duplicate_trade_count = self.duplicate_trade_count.saturating_add(1);
            return;
        }

        self.last_update_ms = Some(event.exchange_ts_ms);
        self.last_trade_ts_ms = Some(event.exchange_ts_ms);
        self.last_trade_price = Some(event.price);
        self.trades.push(event);
    }

    fn apply_top_of_book(&mut self, event: TopOfBookEvent) {
        self.last_update_ms = Some(event.exchange_ts_ms);
        self.bid_px = event.bid_price;
        self.bid_sz = event.bid_size;
        self.ask_px = event.ask_price;
        self.ask_sz = event.ask_size;
        if let (Some(bid), Some(ask)) = (event.bid_price, event.ask_price) {
            self.mid_px = Some((bid + ask) / 2.0);
        }
        self.bbo_events.push(event);
        if self.bbo_events.len() > MAX_BBO_EVENTS_PER_SYMBOL {
            let overflow = self.bbo_events.len() - MAX_BBO_EVENTS_PER_SYMBOL;
            self.bbo_events.drain(0..overflow);
        }
    }

    fn apply_asset_context(&mut self, event: AssetContextEvent) {
        let recv_ms = i64::try_from(event.recv_ts_ns / 1_000_000).unwrap_or(i64::MAX);
        self.day_ntl_vlm = event.day_ntl_vlm;
        self.prev_day_px = event.prev_day_px;
        self.mark_px = event.mark_px;
        self.mid_px = self.mid_px.or(event.mid_px);
        if recv_ms > 0 {
            self.last_update_ms = Some(self.last_update_ms.unwrap_or(0).max(recv_ms));
        }
    }

    fn apply_candle(&mut self, event: CandleEvent) {
        self.last_update_ms = Some(event.close_ts_ms);
        self.candles.push(event);
    }
}
