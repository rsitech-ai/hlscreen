//! Shared-feed consumer. The canonical archive stays exact; this module is the
//! explicit lossy float boundary for hlscreen feature calculations and display.
//! No exchange connection, ticker inference or second protocol decoder lives here.
use hls_core::{HlsError, HlsResult, market_state::*};
use marketfeed_adapter_hyperliquid::protocol::{self, Decimal, Message};
pub use marketfeed_recording::shared_feed::FeedCursor;
use marketfeed_recording::{
    MetadataRecord,
    shared_feed::{FeedReader, FeedRecord},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};
fn error(s: impl ToString) -> HlsError {
    HlsError::Parse(s.to_string())
}
fn float(d: Decimal) -> HlsResult<f64> {
    // This conversion is deliberately confined to the feature projection. It
    // must never feed back into canonical recording, hashing or trade identity.
    let value = d.0.coefficient as f64 / 10_f64.powi(i32::from(d.0.scale));
    if !value.is_finite() || (d.0.coefficient != 0 && value == 0.0) {
        return Err(error("decimal outside finite feature range"));
    }
    Ok(value)
}
fn option(d: Option<Decimal>) -> HlsResult<Option<f64>> {
    d.map(float).transpose()
}
fn level(l: protocol::Level) -> HlsResult<OrderBookLevel> {
    Ok(OrderBookLevel {
        price: float(l.px)?,
        size: float(l.sz)?,
        order_count: l.n,
    })
}
pub fn project(message: Message, recv_ts_ns: u64) -> HlsResult<Vec<MarketEvent>> {
    let mut out = Vec::new();
    match message {
        Message::Trades(rows) => {
            for t in rows {
                let id = t.unique_id();
                let price = float(t.px)?;
                let size = float(t.sz)?;
                let notional = price * size;
                if !notional.is_finite() {
                    return Err(error("feature notional overflow"));
                }
                out.push(MarketEvent::Trade(TradeEvent {
                    recv_ts_ns,
                    exchange_ts_ms: t.time,
                    hl_coin: t.coin,
                    side: if t.side == "B" {
                        TradeSide::Buy
                    } else {
                        TradeSide::Sell
                    },
                    price,
                    size,
                    notional,
                    hash: t.hash,
                    tid: t.tid,
                    unique_trade_id: id,
                }));
            }
        }
        Message::Bbo(b) => {
            let [bid, ask] = b.bbo;
            out.push(MarketEvent::TopOfBook(TopOfBookEvent {
                recv_ts_ns,
                exchange_ts_ms: b.time,
                hl_coin: b.coin,
                bid_price: option(bid.as_ref().map(|l| l.px))?,
                bid_size: option(bid.as_ref().map(|l| l.sz))?,
                bid_order_count: bid.map(|l| l.n),
                ask_price: option(ask.as_ref().map(|l| l.px))?,
                ask_size: option(ask.as_ref().map(|l| l.sz))?,
                ask_order_count: ask.map(|l| l.n),
            }));
        }
        Message::Book(b) => {
            let [bids, asks] = b.levels;
            out.push(MarketEvent::OrderBook(OrderBookEvent {
                recv_ts_ns,
                exchange_ts_ms: b.time,
                hl_coin: b.coin,
                bids: bids.into_iter().map(level).collect::<HlsResult<_>>()?,
                asks: asks.into_iter().map(level).collect::<HlsResult<_>>()?,
            }));
        }
        Message::Candle(c) => out.push(MarketEvent::Candle(CandleEvent {
            recv_ts_ns,
            open_ts_ms: c.t,
            close_ts_ms: c.end,
            hl_coin: c.s,
            interval: c.i,
            open: float(c.o)?,
            high: float(c.h)?,
            low: float(c.l)?,
            close: float(c.c)?,
            volume_base: float(c.v)?,
            trade_count: c.n,
            provenance: CandleProvenance::WebSocket,
            completion: CandleCompletion::Open,
        })),
        Message::Context(c) => out.push(MarketEvent::AssetContext(AssetContextEvent {
            recv_ts_ns,
            hl_coin: c.coin,
            day_ntl_vlm: Some(float(c.ctx.day_ntl_vlm)?),
            prev_day_px: Some(float(c.ctx.prev_day_px)?),
            mark_px: Some(float(c.ctx.mark_px)?),
            mid_px: option(c.ctx.mid_px)?,
            circulating_supply: option(c.ctx.circulating_supply)?,
        })),
        Message::AllMids(m) => out.push(MarketEvent::AllMids(AllMidsEvent {
            recv_ts_ns,
            mids_by_hl_coin: m
                .into_iter()
                .map(|(s, p)| Ok((s, float(p)?)))
                .collect::<HlsResult<_>>()?,
        })),
        Message::Subscription(_) | Message::Pong => {}
    }
    Ok(out)
}
/// Live standalone mode uses this same decoder. Legacy recordings can retain
/// their original format reader without owning a second live protocol path.
pub fn parse_shared_ws_message(raw: &str) -> HlsResult<Vec<MarketEvent>> {
    project(protocol::decode(raw.as_bytes()).map_err(error)?, 0)
}
#[derive(Debug)]
pub enum SharedUpdate {
    Symbols(Vec<String>),
    Stopped,
    Sealed,
    Events(Vec<MarketEvent>),
    Gap {
        symbols: Vec<String>,
        reason: String,
    },
}
struct Pending {
    seq: u64,
    recv: u64,
    payload: Vec<u8>,
}
pub struct SharedConsumer {
    reader: FeedReader,
    sessions: BTreeMap<u64, Vec<String>>,
    pending: BTreeMap<u64, Pending>,
    symbols: BTreeSet<String>,
    caught_up: bool,
    sealed: bool,
    as_of_ns: Option<i64>,
}
impl SharedConsumer {
    /// Rebuilds from the committed prefix on restart; feature state is replayed
    /// alongside metadata rather than resuming a cursor with empty windows.
    pub fn open(directory: impl AsRef<Path>) -> HlsResult<Self> {
        Ok(Self {
            reader: FeedReader::open(directory, None).map_err(error)?,
            sessions: BTreeMap::new(),
            pending: BTreeMap::new(),
            symbols: BTreeSet::new(),
            caught_up: false,
            sealed: false,
            as_of_ns: None,
        })
    }
    pub fn sealed(&self) -> bool {
        self.sealed
    }
    pub fn caught_up(&self) -> bool {
        self.caught_up
    }
    /// Clock recorded at the consumed cursor; deterministic for archive replay.
    pub fn as_of_ms(&self) -> Option<i64> {
        self.as_of_ns.map(|ns| ns / 1_000_000)
    }
    pub fn cursor(&self) -> &FeedCursor {
        self.reader.cursor()
    }
    pub fn symbols(&self) -> Vec<String> {
        self.symbols.iter().cloned().collect()
    }
    /// At most `budget` disk records per poll so replay cannot starve the UI.
    pub fn poll(&mut self, budget: usize) -> HlsResult<Vec<SharedUpdate>> {
        if budget == 0 || budget > 4096 {
            return Err(error("shared consumer poll budget must be 1..4096"));
        }
        let mut updates = Vec::new();
        self.caught_up = false;
        for _ in 0..budget {
            let Some(record) = self.reader.read_next().map_err(error)? else {
                self.caught_up = true;
                break;
            };
            match &record {
                FeedRecord::Seal { now_ns, .. }
                | FeedRecord::Timer { now_ns, .. }
                | FeedRecord::Connection { now_ns, .. }
                | FeedRecord::Lifecycle { now_ns, .. } => {
                    self.as_of_ns = Some(self.as_of_ns.map_or(*now_ns, |old| old.max(*now_ns)))
                }
                FeedRecord::Frame {
                    receive_ts_ns,
                    direction: 0,
                    ..
                } => {
                    self.as_of_ns = Some(
                        self.as_of_ns
                            .map_or(*receive_ts_ns, |old| old.max(*receive_ts_ns)),
                    )
                }
                _ => {}
            }
            match record {
                FeedRecord::Seal {
                    schema_version: 1, ..
                } => {
                    self.sealed = true;
                    updates.push(SharedUpdate::Sealed);
                }

                FeedRecord::Metadata(MetadataRecord::Session(m))
                    if matches!(m.venue_id, 21 | 22) =>
                {
                    if m.schema_version != 1 {
                        return Err(error("unsupported shared session metadata"));
                    }
                    if self.sessions.len() >= 10 && !self.sessions.contains_key(&m.session_id) {
                        return Err(error("shared session budget exceeded"));
                    }
                    let ids: BTreeSet<_> = m
                        .initial_subscriptions
                        .iter()
                        .map(|s| s.instrument_id)
                        .collect();
                    let symbols: Vec<_> = m
                        .catalog
                        .iter()
                        .filter(|i| ids.contains(&i.instrument_id))
                        .map(|i| i.native_symbol.clone())
                        .collect();
                    if symbols.is_empty() || symbols.len() != ids.len() {
                        return Err(error("incomplete shared instrument mapping"));
                    }
                    if self.sessions.contains_key(&m.session_id) {
                        return Err(error("duplicate shared session metadata"));
                    }
                    for s in &symbols {
                        self.symbols.insert(s.clone());
                    }
                    if self.symbols.len() > 1000 {
                        return Err(error("shared symbol budget exceeded"));
                    }
                    self.sessions.insert(m.session_id, symbols.clone());
                    updates.push(SharedUpdate::Symbols(symbols));
                }
                FeedRecord::Frame {
                    session,
                    frame_seq,
                    receive_ts_ns,
                    direction: 0,
                    opcode: 1,
                    payload_base64,
                    ..
                } if self.sessions.contains_key(&session.0) => {
                    let frame = FeedRecord::Frame {
                        session,
                        frame_seq,
                        receive_ts_ns,
                        monotonic_ns: 0,
                        direction: 0,
                        opcode: 1,
                        payload_base64,
                    };
                    let payload = frame
                        .payload_bytes()
                        .map_err(error)?
                        .ok_or_else(|| error("missing original frame"))?;
                    if payload.len() > 4 * 1024 * 1024 {
                        return Err(error("Hyperliquid shared frame too large"));
                    }
                    let recv = u64::try_from(receive_ts_ns).map_err(error)?;
                    if self
                        .pending
                        .insert(
                            session.0,
                            Pending {
                                seq: frame_seq,
                                recv,
                                payload,
                            },
                        )
                        .is_some()
                    {
                        updates.push(SharedUpdate::Gap {
                            symbols: self.sessions[&session.0].clone(),
                            reason: "previous source frame was never accepted".into(),
                        });
                    }
                }
                FeedRecord::FrameAccepted { session, frame_seq }
                    if self.sessions.contains_key(&session.0) =>
                {
                    let pending = self
                        .pending
                        .remove(&session.0)
                        .ok_or_else(|| error("acceptance without original shared frame"))?;
                    if pending.seq != frame_seq {
                        return Err(error("shared original/accepted frame mismatch"));
                    }
                    let message = protocol::decode(&pending.payload).map_err(error)?;
                    let events = project(message, pending.recv)?;
                    for e in &events {
                        if let Some(coin) = e.hl_coin() {
                            if !self.sessions[&session.0].iter().any(|s| s == coin) {
                                return Err(error("event outside shared session catalog"));
                            }
                        }
                    }
                    if !events.is_empty() {
                        updates.push(SharedUpdate::Events(events));
                    }
                }
                FeedRecord::Connection {
                    session,
                    connected: false,
                    reason,
                    ..
                } if self.sessions.contains_key(&session.0) => {
                    let unaccepted = self.pending.remove(&session.0).is_some();
                    let reason =
                        reason.unwrap_or_else(|| "disconnected;continuity_unverified".into());
                    if reason.starts_with("LocalStop") && !unaccepted {
                        updates.push(SharedUpdate::Stopped);
                    } else {
                        updates.push(SharedUpdate::Gap {
                            symbols: self.sessions[&session.0].clone(),
                            reason,
                        });
                    }
                }
                FeedRecord::Lifecycle { session, state, .. }
                    if self.sessions.contains_key(&session.0)
                        && state.starts_with("disconnected") =>
                {
                    let unaccepted = self.pending.remove(&session.0).is_some();
                    if state.starts_with("disconnected:LocalStop") && !unaccepted {
                        updates.push(SharedUpdate::Stopped);
                    } else {
                        updates.push(SharedUpdate::Gap {
                            symbols: self.sessions[&session.0].clone(),
                            reason: state,
                        });
                    }
                }
                FeedRecord::System { session, event } if self.sessions.contains_key(&session.0) => {
                    let text = format!("{event:?}");
                    if text.starts_with("ParseError")
                        || text.starts_with("EventsDropped")
                        || text.starts_with("SequenceGap")
                    {
                        updates.push(SharedUpdate::Gap {
                            symbols: self.sessions[&session.0].clone(),
                            reason: text,
                        });
                    }
                }
                _ => {}
            }
        }
        Ok(updates)
    }
}
