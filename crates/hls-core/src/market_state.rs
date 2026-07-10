use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::confidence::DataConfidenceSnapshot;
use crate::metadata::MetadataEnrichment;
use crate::score::ScoreBreakdown;
use crate::{HlsError, HlsResult};

const MAX_BBO_EVENTS_PER_SYMBOL: usize = 256;
const MAX_CANDLE_EVENTS_PER_SYMBOL: usize = 512;
const MAX_TRADE_EVENTS_PER_SYMBOL: usize = 100_000;
const TRADE_RETENTION_MS: i64 = 60 * 60 * 1_000;

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
    trade_ids_by_symbol: HashMap<String, HashSet<String>>,
    latest_asset_context_recv_ns: HashMap<String, u64>,
    latest_all_mids_recv_ns: HashMap<String, u64>,
}

impl LiveMarketState {
    pub fn new(symbols: impl IntoIterator<Item = String>) -> Self {
        let symbols: HashSet<String> = symbols.into_iter().collect();
        let states = symbols
            .iter()
            .map(|symbol| (symbol.clone(), SymbolMarketState::new(symbol.clone())))
            .collect();

        Self {
            symbols,
            states,
            trade_ids_by_symbol: HashMap::new(),
            latest_asset_context_recv_ns: HashMap::new(),
            latest_all_mids_recv_ns: HashMap::new(),
        }
    }

    pub fn apply(&mut self, event: MarketEvent) -> HlsResult<()> {
        match event {
            MarketEvent::AllMids(event) => {
                let recv_ms = i64::try_from(event.recv_ts_ns / 1_000_000).unwrap_or(i64::MAX);
                for (hl_coin, mid) in event.mids_by_hl_coin {
                    if !self.states.contains_key(&hl_coin) {
                        continue;
                    }
                    let latest_recv_ns = self
                        .latest_all_mids_recv_ns
                        .entry(hl_coin.clone())
                        .or_default();
                    if event.recv_ts_ns >= *latest_recv_ns
                        && let Some(state) = self.states.get_mut(&hl_coin)
                    {
                        *latest_recv_ns = event.recv_ts_ns;
                        state.mid_px = Some(mid);
                        if recv_ms > 0 {
                            state.last_update_ms =
                                Some(state.last_update_ms.unwrap_or(0).max(recv_ms));
                        }
                    }
                }
            }
            MarketEvent::Trade(event) => {
                let hl_coin = event.hl_coin.clone();
                self.state_mut(&hl_coin)?;
                let state = self.states.get_mut(&hl_coin).ok_or_else(|| {
                    HlsError::Config(format!("state for symbol '{hl_coin}' was not initialized"))
                })?;
                let trade_ids = self.trade_ids_by_symbol.entry(hl_coin).or_default();
                state.apply_trade(event, trade_ids);
            }
            MarketEvent::TopOfBook(event) => {
                self.state_mut(&event.hl_coin)?.apply_top_of_book(event);
            }
            MarketEvent::AssetContext(event) => {
                let hl_coin = event.hl_coin.clone();
                self.state_mut(&hl_coin)?;
                let latest_recv_ns = self
                    .latest_asset_context_recv_ns
                    .entry(hl_coin.clone())
                    .or_default();
                if event.recv_ts_ns >= *latest_recv_ns {
                    *latest_recv_ns = event.recv_ts_ns;
                    self.states
                        .get_mut(&hl_coin)
                        .ok_or_else(|| {
                            HlsError::Config(format!(
                                "state for symbol '{hl_coin}' was not initialized"
                            ))
                        })?
                        .apply_asset_context(event);
                }
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

    fn apply_trade(&mut self, event: TradeEvent, trade_ids: &mut HashSet<String>) {
        self.rebuild_trade_ids_if_needed(trade_ids);
        let latest_ts_ms = self
            .last_trade_ts_ms
            .unwrap_or(event.exchange_ts_ms)
            .max(event.exchange_ts_ms);
        let cutoff_ms = latest_ts_ms.saturating_sub(TRADE_RETENTION_MS);
        self.prune_trades_before(cutoff_ms, trade_ids);

        if event.exchange_ts_ms < cutoff_ms {
            return;
        }
        if !trade_ids.insert(event.unique_trade_id.clone()) {
            self.duplicate_trade_count = self.duplicate_trade_count.saturating_add(1);
            return;
        }

        self.last_update_ms = Some(
            self.last_update_ms
                .unwrap_or(event.exchange_ts_ms)
                .max(event.exchange_ts_ms),
        );
        if self
            .last_trade_ts_ms
            .is_none_or(|last_trade_ts_ms| event.exchange_ts_ms >= last_trade_ts_ms)
        {
            self.last_trade_ts_ms = Some(event.exchange_ts_ms);
            self.last_trade_price = Some(event.price);
        }

        let insert_at = self
            .trades
            .partition_point(|trade| trade.exchange_ts_ms <= event.exchange_ts_ms);
        self.trades.insert(insert_at, event);
        self.enforce_trade_count_limit(trade_ids);
    }

    fn rebuild_trade_ids_if_needed(&self, trade_ids: &mut HashSet<String>) {
        if trade_ids.len() != self.trades.len() {
            trade_ids.clear();
            trade_ids.extend(
                self.trades
                    .iter()
                    .map(|trade| trade.unique_trade_id.clone()),
            );
        }
    }

    fn prune_trades_before(&mut self, cutoff_ms: i64, trade_ids: &mut HashSet<String>) {
        let stale_count = self
            .trades
            .partition_point(|trade| trade.exchange_ts_ms < cutoff_ms);
        for trade in self.trades.drain(..stale_count) {
            trade_ids.remove(&trade.unique_trade_id);
        }
    }

    fn enforce_trade_count_limit(&mut self, trade_ids: &mut HashSet<String>) {
        let overflow = self
            .trades
            .len()
            .saturating_sub(MAX_TRADE_EVENTS_PER_SYMBOL);
        for trade in self.trades.drain(..overflow) {
            trade_ids.remove(&trade.unique_trade_id);
        }
    }

    fn apply_top_of_book(&mut self, event: TopOfBookEvent) {
        let is_current = self
            .bbo_events
            .last()
            .is_none_or(|latest| event.exchange_ts_ms >= latest.exchange_ts_ms);
        self.last_update_ms = Some(
            self.last_update_ms
                .unwrap_or(event.exchange_ts_ms)
                .max(event.exchange_ts_ms),
        );
        if is_current {
            self.bid_px = event.bid_price;
            self.bid_sz = event.bid_size;
            self.ask_px = event.ask_price;
            self.ask_sz = event.ask_size;
            if let (Some(bid), Some(ask)) = (event.bid_price, event.ask_price) {
                self.mid_px = Some((bid + ask) / 2.0);
            }
        }
        let insert_at = self
            .bbo_events
            .partition_point(|quote| quote.exchange_ts_ms <= event.exchange_ts_ms);
        self.bbo_events.insert(insert_at, event);
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
        self.last_update_ms = Some(
            self.last_update_ms
                .unwrap_or(event.close_ts_ms)
                .max(event.close_ts_ms),
        );
        if let Some(existing) = self.candles.iter_mut().find(|candle| {
            candle.interval == event.interval && candle.open_ts_ms == event.open_ts_ms
        }) {
            *existing = event;
            return;
        }

        self.candles.push(event);
        self.candles.sort_by_key(|candle| candle.open_ts_ms);
        if self.candles.len() > MAX_CANDLE_EVENTS_PER_SYMBOL {
            let overflow = self.candles.len() - MAX_CANDLE_EVENTS_PER_SYMBOL;
            self.candles.drain(0..overflow);
        }
    }
}

#[cfg(test)]
mod internal_tests {
    use super::*;

    #[test]
    fn all_mids_does_not_track_symbols_outside_the_selected_universe() {
        let mut state = LiveMarketState::new(["@107".to_owned()]);
        let mids_by_hl_coin = (0..1_000)
            .map(|index| (format!("unknown-{index}"), 1.0))
            .collect();

        state
            .apply(MarketEvent::AllMids(AllMidsEvent {
                recv_ts_ns: 1,
                mids_by_hl_coin,
            }))
            .expect("unknown all-mids symbols are ignored");

        assert!(state.latest_all_mids_recv_ns.is_empty());
    }
}
