//! MFOP1 consumes committed typed observations, never raw frame acceptance.
use super::*;
use marketfeed_adapter_api::{
    HyperliquidLevelObservation, HyperliquidTradeSide, NativeObservation,
};
use marketfeed_engine::causal::{
    BootstrapDto, FactoryDto, HyperliquidMarketDto, InstrumentDto, InstrumentKindDto, ItemDto,
    NativeObservationPolicyDto, projection::*,
};
use marketfeed_model::Fixed;
use serde::Serialize;
use std::io::Read;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SharedInstrument {
    pub venue: u16,
    pub session: u64,
    pub instrument: u32,
    pub native_symbol: String,
    pub display_key: String,
    pub metadata: InstrumentDto,
}

enum Source {
    Legacy(Box<LegacySharedConsumer>),
    Projected(Box<Projected>),
}
pub struct SharedConsumer {
    source: Source,
}
impl SharedConsumer {
    pub fn open(directory: impl AsRef<Path>) -> HlsResult<Self> {
        let path = directory.as_ref();
        let mut magic = [0; 8];
        std::fs::File::open(path.join("events.mfsf"))
            .map_err(error)?
            .read_exact(&mut magic)
            .map_err(error)?;
        let source = if &magic == b"MFOP1\0\0\0" {
            Source::Projected(Box::new(Projected::open(path)?))
        } else {
            // Legacy open validates its own header before watermark access;
            // corruption or an unsupported profile never retries another reader.
            Source::Legacy(Box::new(LegacySharedConsumer::open(path)?))
        };
        Ok(Self { source })
    }
    pub fn sealed(&self) -> bool {
        match &self.source {
            Source::Legacy(r) => r.sealed(),
            Source::Projected(r) => r.sealed,
        }
    }
    pub fn caught_up(&self) -> bool {
        match &self.source {
            Source::Legacy(r) => r.caught_up(),
            Source::Projected(r) => !r.poisoned && r.caught_up,
        }
    }
    pub fn as_of_ms(&self) -> Option<i64> {
        match &self.source {
            Source::Legacy(r) => r.as_of_ms(),
            Source::Projected(r) => r.as_of_ns.map(|v| v / 1_000_000),
        }
    }
    pub fn cursor(&self) -> &FeedCursor {
        match &self.source {
            Source::Legacy(r) => r.cursor(),
            Source::Projected(r) => &r.cursor,
        }
    }
    pub fn symbols(&self) -> Vec<String> {
        match &self.source {
            Source::Legacy(r) => r.symbols(),
            Source::Projected(r) if r.poisoned => vec![],
            Source::Projected(r) => r.symbols.iter().cloned().collect(),
        }
    }
    pub fn instruments(&self) -> Vec<SharedInstrument> {
        match &self.source {
            Source::Legacy(_) => vec![],
            Source::Projected(r) if r.poisoned => vec![],
            Source::Projected(r) => r.instruments.values().cloned().collect(),
        }
    }
    pub fn source_cursor(&self) -> Option<&FeedCursor> {
        match &self.source {
            Source::Legacy(_) => None,
            Source::Projected(r) => r.source_cursor.as_ref(),
        }
    }
    pub fn metadata(&self, session: u64) -> HlsResult<Option<&BootstrapDto>> {
        match &self.source {
            Source::Legacy(_) => Ok(None),
            Source::Projected(r) if r.poisoned => Err(error("projected consumer poisoned")),
            Source::Projected(r) if !r.metadata_sessions.contains(&session) => Ok(None),
            Source::Projected(r) => r.reader.metadata(session).map_err(error),
        }
    }
    pub fn clock_policy(&self) -> &'static str {
        match &self.source {
            Source::Legacy(_) => "legacy_max_record_clock_v1",
            Source::Projected(_) => "max_checked_owner_approximation_physical_order_v1",
        }
    }
    /// Read and deliver only complete groups. The cursor covers returned
    /// updates; use consume to advance it only after feature application.
    pub fn poll(&mut self, budget: usize) -> HlsResult<Vec<SharedUpdate>> {
        self.consume(budget, Ok)
    }
    /// MFOP1 cursor/clock publication follows successful application. Any error
    /// poisons the consumer and preserves its last acknowledged cursor pair.
    /// The caller must discard partial external state on Err; no feature
    /// checkpoint or append/restart authority is created by this acknowledgment.
    /// Legacy archives retain their historical read-only reader behavior.
    pub fn consume<T>(
        &mut self,
        budget: usize,
        apply: impl FnOnce(Vec<SharedUpdate>) -> HlsResult<T>,
    ) -> HlsResult<T> {
        match &mut self.source {
            Source::Legacy(r) => r.poll(budget).and_then(apply),
            Source::Projected(r) => r.consume(budget, apply),
        }
    }
}

struct PendingGroup {
    group: ProjectionGroup,
    progress: ProjectionProgress,
    bytes: usize,
}
struct Projected {
    pending: Option<PendingGroup>,
    reader: ProjectionReader,
    cursor: FeedCursor,
    source_cursor: Option<FeedCursor>,
    instruments: BTreeMap<(u64, u32), SharedInstrument>,
    metadata_sessions: BTreeSet<u64>,
    symbols: BTreeSet<String>,
    as_of_ns: Option<i64>,
    caught_up: bool,
    sealed: bool,
    poisoned: bool,
}
impl Projected {
    fn open(path: &Path) -> HlsResult<Self> {
        let reader = ProjectionReader::open(
            path,
            ProjectionLimits {
                max_group_bytes: 8 << 20,
                max_items: 4096,
                max_owners: 10,
                max_bootstrap_bytes: 16 << 20,
                max_catalog_entries: 10_000,
                max_pending_inputs: 4096,
            },
        )
        .map_err(error)?;
        let cursor = reader.progress().map_err(error)?.target_cursor;
        Ok(Self {
            pending: None,
            reader,
            cursor,
            source_cursor: None,
            instruments: BTreeMap::new(),
            metadata_sessions: BTreeSet::new(),
            symbols: BTreeSet::new(),
            as_of_ns: None,
            caught_up: false,
            sealed: false,
            poisoned: false,
        })
    }
    fn session_symbols(&self, session: u64) -> Vec<String> {
        self.instruments
            .values()
            .filter(|i| i.session == session)
            .map(|i| i.display_key.clone())
            .collect()
    }
    fn bootstrap(&self, dto: &BootstrapDto) -> HlsResult<Vec<SharedInstrument>> {
        let FactoryDto::HyperliquidPublic { market } = dto.factory else {
            return Ok(vec![]);
        };
        if !matches!(
            dto.observation_policy().map_err(error)?,
            NativeObservationPolicyDto::HyperliquidV1 { .. }
        ) {
            return Err(error(
                "HLS requires admitted native observations for Hyperliquid",
            ));
        }
        let selected: BTreeSet<_> = dto
            .session
            .subscriptions
            .iter()
            .map(|s| s.instrument)
            .collect();
        let product = match market {
            HyperliquidMarketDto::Spot => "spot",
            HyperliquidMarketDto::Perps => "perps",
        };
        let mut additions = Vec::new();
        for i in &dto.catalog.instruments {
            if selected.contains(&i.id) {
                additions.push(SharedInstrument {
                    venue: dto.capture.venue,
                    session: dto.capture.session,
                    instrument: i.id,
                    native_symbol: i.key.native_symbol.clone(),
                    display_key: format!("{product}:{}", i.key.native_symbol),
                    metadata: i.clone(),
                });
            }
        }
        if additions.len() != selected.len()
            || self.instruments.len().saturating_add(additions.len()) > 1000
            || additions
                .iter()
                .map(|i| &i.display_key)
                .collect::<BTreeSet<_>>()
                .len()
                != additions.len()
        {
            return Err(error("incomplete/excessive projected instrument metadata"));
        }
        for incoming in &additions {
            let (venue, kind, name) = match market {
                HyperliquidMarketDto::Spot => (21, InstrumentKindDto::Spot, "hyperliquid-spot"),
                HyperliquidMarketDto::Perps => {
                    (22, InstrumentKindDto::PerpetualLinear, "hyperliquid-perps")
                }
            };
            if incoming.venue != venue
                || incoming.metadata.key.kind != kind
                || incoming.metadata.key.venue != name
            {
                return Err(error("contradictory product/venue instrument metadata"));
            }
            for existing in self
                .instruments
                .values()
                .filter(|i| i.display_key == incoming.display_key)
            {
                let mut normalized = existing.metadata.clone();
                normalized.id = incoming.instrument;
                if normalized != incoming.metadata {
                    return Err(error("conflicting metadata for one display instrument"));
                }
            }
        }
        Ok(additions)
    }
    fn convert(&self, group: &ProjectionGroup) -> HlsResult<ConvertedGroup> {
        let mut converted = ConvertedGroup::default();
        let mut events_in_group = 0usize;
        let mut levels_in_group = 0usize;
        match &group.source {
            ProjectionSource::Bootstrap(dto) => {
                let additions = self.bootstrap(dto)?;
                if !additions.is_empty() {
                    converted.push(SharedUpdate::Symbols(
                        additions.iter().map(|i| i.display_key.clone()).collect(),
                    ))?;
                }
                converted.additions = additions;
            }
            ProjectionSource::Seal { owners, .. } => {
                for owner in owners.iter().filter(|o| !o.uncertainties.is_empty()) {
                    let symbols = self.session_symbols(owner.identity.session);
                    if !symbols.is_empty() {
                        converted.push(SharedUpdate::Gap {
                            symbols,
                            reason: "sealed with unresolved source execution/handoff".into(),
                        })?;
                    }
                }
                converted.push(SharedUpdate::Sealed)?;
            }
            ProjectionSource::Owner { route, event, .. } => {
                let symbols = self.session_symbols(route.session);
                if !symbols.is_empty() {
                    let unsafe_event = matches!(
                        event,
                        ProjectionOwnerEvent::Ingress {
                            input: ProjectionInput::Disconnected { .. }
                                | ProjectionInput::TransportLost { .. }
                                | ProjectionInput::Close(_),
                            ..
                        } | ProjectionOwnerEvent::ExecutionCompleted {
                            unsafe_state: true,
                            ..
                        } | ProjectionOwnerEvent::ExecutionIndeterminate { .. }
                            | ProjectionOwnerEvent::ExecutionNotInvoked { .. }
                            | ProjectionOwnerEvent::NotExecuted { .. }
                            | ProjectionOwnerEvent::Revoked { .. }
                            | ProjectionOwnerEvent::Withheld { .. }
                    );
                    if unsafe_event {
                        converted.push(SharedUpdate::Gap {
                            symbols,
                            reason: "committed source safety/continuity transition".into(),
                        })?;
                    }
                }
            }
        }
        if let Some(owner) = &group.owner {
            let symbols = self.session_symbols(owner.session);
            if !symbols.is_empty() {
                for item in &group.items {
                    match item {
                        ItemDto::ObservedMarket(leaf) => {
                            let batch = leaf.batch().map_err(error)?;
                            events_in_group = events_in_group
                                .checked_add(batch.native().rows.len())
                                .filter(|n| *n <= 4096)
                                .ok_or_else(|| error("projected feature event budget"))?;
                            let received = u64::try_from(batch.source().received.receive_ts.0)
                                .map_err(error)?;
                            let mut events = Vec::new();
                            for row in &batch.native().rows {
                                if let NativeObservation::HyperliquidBook(book) = &row.observation {
                                    levels_in_group = levels_in_group
                                        .checked_add(book.sides[0].len())
                                        .and_then(|n| n.checked_add(book.sides[1].len()))
                                        .filter(|n| *n <= 40_000)
                                        .ok_or_else(|| {
                                            error("projected feature book-level budget")
                                        })?;
                                }
                                let mut event = project_native(&row.observation, received)?;
                                let coin = event
                                    .hl_coin()
                                    .ok_or_else(|| error("unadmitted native map scope"))?;
                                let identity = self
                                    .instruments
                                    .values()
                                    .find(|i| i.session == owner.session && i.native_symbol == coin)
                                    .ok_or_else(|| error("missing native display identity"))?;
                                qualify(&mut event, &identity.display_key);
                                events.push(event);
                            }
                            if !events.is_empty() {
                                converted.push(SharedUpdate::Events(events))?;
                            }
                        }
                        ItemDto::System(leaf) => {
                            let FeedRecord::System { event, .. } = leaf.record().map_err(error)?
                            else {
                                return Err(error("projected system kind"));
                            };
                            if matches!(
                                event,
                                marketfeed_model::SystemEvent::ParseError { .. }
                                    | marketfeed_model::SystemEvent::EventsDropped { .. }
                                    | marketfeed_model::SystemEvent::SequenceGap { .. }
                                    | marketfeed_model::SystemEvent::QuoteInvalidated { .. }
                                    | marketfeed_model::SystemEvent::BookInvalidated { .. }
                            ) {
                                converted.push(SharedUpdate::Gap {
                                    symbols: symbols.clone(),
                                    reason: format!("{event:?}"),
                                })?;
                            }
                        }
                        // Canonical companion batches (e.g. VenueStatus) are
                        // not a second copy of native market observations.
                        ItemDto::Market(_) => {}
                        _ => return Err(error("outbound intent in market items")),
                    }
                }
            }
        }
        Ok(converted)
    }
    fn consume<T>(
        &mut self,
        budget: usize,
        apply: impl FnOnce(Vec<SharedUpdate>) -> HlsResult<T>,
    ) -> HlsResult<T> {
        if budget == 0 || budget > 4096 {
            return Err(error("shared consumer poll budget must be 1..4096"));
        }
        if self.poisoned {
            return Err(error("projected consumer poisoned; rebuild from header"));
        }
        // A caught callback/conversion panic must not make skipped input
        // reusable. Publish success explicitly only after application.
        self.poisoned = true;
        let result = (|| {
            let mut updates = Vec::new();
            let mut cursor = self.cursor.clone();
            let mut source_cursor = self.source_cursor.clone();
            let mut as_of_ns = self.as_of_ns;
            let mut sealed = self.sealed;
            let mut caught_up = false;
            let mut bytes = 0usize;
            let mut output_bytes = 0usize;
            let mut events = 0usize;
            let mut levels = 0usize;
            for _ in 0..budget {
                let prepared = match self.pending.take() {
                    Some(p) => p,
                    None => {
                        let Some(group) = self.reader.read_next().map_err(error)? else {
                            caught_up = true;
                            break;
                        };
                        let progress = self.reader.progress().map_err(error)?;
                        let mut counter = GroupSize(0);
                        serde_json::to_writer(&mut counter, &group).map_err(error)?;
                        PendingGroup {
                            group,
                            progress,
                            bytes: counter.0,
                        }
                    }
                };
                if bytes
                    .checked_add(prepared.bytes)
                    .is_none_or(|n| n > 8 << 20)
                    && cursor != self.cursor
                {
                    self.pending = Some(prepared);
                    break;
                }
                if matches!(
                    prepared.group.clock,
                    ProjectionClock::Owner {
                        approximate_unix_ns: None,
                        ..
                    }
                ) {
                    return Err(error("owner archive clock approximation unavailable"));
                }
                let converted = self.convert(&prepared.group)?;
                let mut next_events = 0usize;
                let mut next_levels = 0usize;
                for update in &converted.updates {
                    if let SharedUpdate::Events(rows) = update {
                        next_events += rows.len();
                        for row in rows {
                            if let MarketEvent::OrderBook(book) = row {
                                next_levels += book.bids.len() + book.asks.len();
                            }
                        }
                    }
                }
                if (events + next_events > 4096
                    || levels + next_levels > 40_000
                    || output_bytes + converted.bytes > 8 << 20)
                    && cursor != self.cursor
                {
                    // Conversion stages metadata without publishing it. Keep
                    // one whole group; never rerun native source execution.
                    self.pending = Some(prepared);
                    break;
                }
                bytes += prepared.bytes;
                output_bytes += converted.bytes;
                events += next_events;
                levels += next_levels;
                if let ProjectionClock::Owner {
                    approximate_unix_ns: Some(ns),
                    ..
                } = prepared.group.clock
                {
                    as_of_ns = Some(as_of_ns.map_or(ns, |old| old.max(ns)));
                }
                cursor = prepared.progress.target_cursor;
                source_cursor = prepared.progress.source_cursor;
                sealed = prepared.progress.sealed;
                if let ProjectionSource::Bootstrap(dto) = &prepared.group.source {
                    self.metadata_sessions.insert(dto.capture.session);
                }
                for instrument in converted.additions {
                    self.symbols.insert(instrument.display_key.clone());
                    self.instruments
                        .insert((instrument.session, instrument.instrument), instrument);
                }
                updates.extend(converted.updates);
            }
            let applied = apply(updates)?;
            self.cursor = cursor;
            self.source_cursor = source_cursor;
            self.as_of_ns = as_of_ns;
            self.sealed = sealed;
            self.caught_up = caught_up;
            self.poisoned = false;
            Ok(applied)
        })();
        if result.is_err() {
            self.poisoned = true;
            self.pending = None;
            self.instruments.clear();
            self.metadata_sessions.clear();
            self.symbols.clear();
            self.caught_up = false;
        }
        result
    }
}

#[derive(Default)]
struct ConvertedGroup {
    updates: Vec<SharedUpdate>,
    additions: Vec<SharedInstrument>,
    bytes: usize,
}
impl ConvertedGroup {
    fn push(&mut self, update: SharedUpdate) -> HlsResult<()> {
        // Count encoded output, including every cloned gap symbol and reason,
        // before retaining another update. One oversized group fails closed.
        let mut counter = GroupSize(self.bytes);
        serde_json::to_writer(&mut counter, &update).map_err(error)?;
        self.bytes = counter.0;
        self.updates.push(update);
        Ok(())
    }
}

struct GroupSize(usize);
impl std::io::Write for GroupSize {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0 = self
            .0
            .checked_add(bytes.len())
            .filter(|n| *n <= 8 << 20)
            .ok_or_else(|| std::io::Error::other("shared input/output byte budget"))?;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
fn decimal(v: Fixed) -> HlsResult<f64> {
    super::float(Decimal(v))
}
fn optional(v: Option<Fixed>) -> HlsResult<Option<f64>> {
    v.map(decimal).transpose()
}
fn native_level(v: &HyperliquidLevelObservation) -> HlsResult<OrderBookLevel> {
    Ok(OrderBookLevel {
        price: decimal(v.price)?,
        size: decimal(v.size)?,
        order_count: v.orders,
    })
}
fn qualify(event: &mut MarketEvent, key: &str) {
    match event {
        MarketEvent::Trade(v) => v.hl_coin = key.into(),
        MarketEvent::TopOfBook(v) => v.hl_coin = key.into(),
        MarketEvent::OrderBook(v) => v.hl_coin = key.into(),
        MarketEvent::AssetContext(v) => v.hl_coin = key.into(),
        MarketEvent::Candle(v) => v.hl_coin = key.into(),
        MarketEvent::AllMids(_) => {}
    }
}
pub(super) fn project_native(row: &NativeObservation, recv_ts_ns: u64) -> HlsResult<MarketEvent> {
    Ok(match row {
        NativeObservation::HyperliquidTrade(v) => {
            let price = decimal(v.price)?;
            let size = decimal(v.size)?;
            let notional = price * size;
            if !notional.is_finite() {
                return Err(error("feature notional overflow"));
            }
            MarketEvent::Trade(TradeEvent {
                recv_ts_ns,
                exchange_ts_ms: v.time_ms,
                hl_coin: v.coin.clone(),
                side: match v.side {
                    HyperliquidTradeSide::Buy => TradeSide::Buy,
                    HyperliquidTradeSide::Sell => TradeSide::Sell,
                },
                price,
                size,
                notional,
                hash: v.hash.clone(),
                tid: v.tid,
                unique_trade_id: v.unique_id(),
            })
        }
        NativeObservation::HyperliquidBbo(v) => MarketEvent::TopOfBook(TopOfBookEvent {
            recv_ts_ns,
            exchange_ts_ms: v.time_ms,
            hl_coin: v.coin.clone(),
            bid_price: optional(v.sides[0].as_ref().map(|l| l.price))?,
            bid_size: optional(v.sides[0].as_ref().map(|l| l.size))?,
            bid_order_count: v.sides[0].as_ref().map(|l| l.orders),
            ask_price: optional(v.sides[1].as_ref().map(|l| l.price))?,
            ask_size: optional(v.sides[1].as_ref().map(|l| l.size))?,
            ask_order_count: v.sides[1].as_ref().map(|l| l.orders),
        }),
        NativeObservation::HyperliquidBook(v) => MarketEvent::OrderBook(OrderBookEvent {
            recv_ts_ns,
            exchange_ts_ms: v.time_ms,
            hl_coin: v.coin.clone(),
            bids: v.sides[0]
                .iter()
                .map(native_level)
                .collect::<HlsResult<_>>()?,
            asks: v.sides[1]
                .iter()
                .map(native_level)
                .collect::<HlsResult<_>>()?,
        }),
        NativeObservation::HyperliquidCandle(v) => MarketEvent::Candle(CandleEvent {
            recv_ts_ns,
            open_ts_ms: v.open_ms,
            close_ts_ms: v.end_ms,
            hl_coin: v.coin.clone(),
            interval: v.interval.clone(),
            open: decimal(v.open)?,
            high: decimal(v.high)?,
            low: decimal(v.low)?,
            close: decimal(v.close)?,
            volume_base: decimal(v.volume)?,
            trade_count: v.trades,
            provenance: CandleProvenance::WebSocket,
            completion: CandleCompletion::Open,
        }),
        NativeObservation::HyperliquidContext(v) => MarketEvent::AssetContext(AssetContextEvent {
            recv_ts_ns,
            hl_coin: v.coin.clone(),
            day_ntl_vlm: Some(decimal(v.day_notional_volume)?),
            prev_day_px: Some(decimal(v.previous_day_price)?),
            mark_px: Some(decimal(v.mark_price)?),
            mid_px: optional(v.mid_price)?,
            circulating_supply: optional(v.circulating_supply)?,
            native_context: Some(NativeContextFields {
                mid_price: optional(v.mid_price)?,
                circulating_supply: optional(v.circulating_supply)?,
                funding: optional(v.funding)?,
                open_interest: optional(v.open_interest)?,
                oracle_price: optional(v.oracle_price)?,
                day_base_volume: optional(v.day_base_volume)?,
                premium: optional(v.premium)?,
                impact_prices: v
                    .impact_prices
                    .map(|[a, b]| Ok::<_, HlsError>([decimal(a)?, decimal(b)?]))
                    .transpose()?,
            }),
        }),
        NativeObservation::HyperliquidAllMids(_) => {
            return Err(error(
                "all-mids actual producer admission is not implemented",
            ));
        }
    })
}
