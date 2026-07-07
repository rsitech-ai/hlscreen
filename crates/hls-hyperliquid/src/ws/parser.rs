use hls_core::{
    HlsError, HlsResult,
    market_state::{
        AllMidsEvent, AssetContextEvent, CandleEvent, MarketEvent, TopOfBookEvent, TradeEvent,
        TradeSide,
    },
};
use serde::de::DeserializeOwned;

use super::types::{WsActiveSpotAssetCtx, WsAllMids, WsBbo, WsCandle, WsEnvelope, WsTrade};

pub fn parse_ws_ndjson(raw: &str) -> HlsResult<Vec<MarketEvent>> {
    let mut events = Vec::new();

    for (line_number, line) in raw.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let parsed = parse_ws_message(line)
            .map_err(|err| HlsError::Parse(format!("line {}: {err}", line_number + 1)))?;
        events.extend(parsed);
    }

    Ok(events)
}

pub fn parse_ws_message(raw: &str) -> HlsResult<Vec<MarketEvent>> {
    let envelope: WsEnvelope = serde_json::from_str(raw)
        .map_err(|err| HlsError::Parse(format!("invalid WS envelope: {err}")))?;

    match envelope.channel.as_str() {
        "trades" => parse_trades(envelope.data),
        "bbo" => parse_bbo(envelope.data).map(|event| vec![event]),
        "allMids" => parse_all_mids(envelope.data).map(|event| vec![event]),
        "activeAssetCtx" => parse_active_asset_ctx(envelope.data).map(|event| vec![event]),
        "candle" => parse_candles(envelope.data),
        "subscriptionResponse" => Ok(Vec::new()),
        "userFills" | "userEvents" | "orderUpdates" | "openOrders" | "notification" => {
            Err(HlsError::Parse(format!(
                "unsupported private or trading channel '{}'",
                envelope.channel
            )))
        }
        other => Err(HlsError::Parse(format!(
            "unsupported public channel '{other}'"
        ))),
    }
}

fn parse_trades(data: serde_json::Value) -> HlsResult<Vec<MarketEvent>> {
    parse_json::<Vec<WsTrade>>(data, "trades")?
        .into_iter()
        .map(|trade| {
            let side = match trade.side.as_str() {
                "B" | "buy" | "Buy" => TradeSide::Buy,
                "A" | "sell" | "Sell" => TradeSide::Sell,
                other => return Err(HlsError::Parse(format!("unsupported trade side '{other}'"))),
            };
            let price = parse_positive(&trade.px, "trade.px")?;
            let size = parse_positive(&trade.sz, "trade.sz")?;
            let notional = price * size;
            let unique_trade_id = format!("{}:{}:{}", trade.coin, trade.time, trade.tid);

            Ok(MarketEvent::Trade(TradeEvent {
                recv_ts_ns: 0,
                exchange_ts_ms: trade.time,
                hl_coin: trade.coin,
                side,
                price,
                size,
                notional,
                hash: trade.hash,
                tid: trade.tid,
                unique_trade_id,
            }))
        })
        .collect()
}

fn parse_bbo(data: serde_json::Value) -> HlsResult<MarketEvent> {
    let bbo = parse_json::<WsBbo>(data, "bbo")?;
    let [bid, ask] = bbo.bbo;
    let (bid_price, bid_size, bid_order_count) = level_parts(bid, "bid")?;
    let (ask_price, ask_size, ask_order_count) = level_parts(ask, "ask")?;

    if let (Some(bid_price), Some(ask_price)) = (bid_price, ask_price)
        && bid_price > ask_price
    {
        return Err(HlsError::Parse(
            "bbo bid price must be <= ask price".to_owned(),
        ));
    }

    Ok(MarketEvent::TopOfBook(TopOfBookEvent {
        recv_ts_ns: 0,
        exchange_ts_ms: bbo.time,
        hl_coin: bbo.coin,
        bid_price,
        bid_size,
        bid_order_count,
        ask_price,
        ask_size,
        ask_order_count,
    }))
}

fn parse_all_mids(data: serde_json::Value) -> HlsResult<MarketEvent> {
    let all_mids = parse_json::<WsAllMids>(data, "allMids")?;
    let mids_by_hl_coin = all_mids
        .mids
        .into_iter()
        .map(|(coin, mid)| parse_positive(&mid, "allMids.mid").map(|mid| (coin, mid)))
        .collect::<HlsResult<_>>()?;

    Ok(MarketEvent::AllMids(AllMidsEvent {
        recv_ts_ns: 0,
        mids_by_hl_coin,
    }))
}

fn parse_active_asset_ctx(data: serde_json::Value) -> HlsResult<MarketEvent> {
    let ctx = parse_json::<WsActiveSpotAssetCtx>(data, "activeAssetCtx")?;

    Ok(MarketEvent::AssetContext(AssetContextEvent {
        recv_ts_ns: 0,
        hl_coin: ctx.coin,
        day_ntl_vlm: ctx.ctx.day_ntl_vlm,
        prev_day_px: ctx.ctx.prev_day_px,
        mark_px: ctx.ctx.mark_px,
        mid_px: ctx.ctx.mid_px,
        circulating_supply: ctx.ctx.circulating_supply,
    }))
}

fn parse_candles(data: serde_json::Value) -> HlsResult<Vec<MarketEvent>> {
    parse_json::<Vec<WsCandle>>(data, "candle")?
        .into_iter()
        .map(|candle| {
            if candle.t > candle.close_ms {
                return Err(HlsError::Parse(
                    "candle open time must be <= close time".to_owned(),
                ));
            }
            if candle.o <= 0.0 || candle.c <= 0.0 || candle.h <= 0.0 || candle.l <= 0.0 {
                return Err(HlsError::Parse(
                    "candle OHLC values must be positive".to_owned(),
                ));
            }
            if candle.h < candle.l
                || candle.h < candle.o
                || candle.h < candle.c
                || candle.l > candle.o
                || candle.l > candle.c
            {
                return Err(HlsError::Parse(
                    "candle OHLC values are internally inconsistent".to_owned(),
                ));
            }

            Ok(MarketEvent::Candle(CandleEvent {
                recv_ts_ns: 0,
                open_ts_ms: candle.t,
                close_ts_ms: candle.close_ms,
                hl_coin: candle.s,
                interval: candle.i,
                open: candle.o,
                high: candle.h,
                low: candle.l,
                close: candle.c,
                volume_base: candle.v,
                trade_count: candle.n,
            }))
        })
        .collect()
}

fn parse_json<T: DeserializeOwned>(value: serde_json::Value, label: &str) -> HlsResult<T> {
    serde_json::from_value(value)
        .map_err(|err| HlsError::Parse(format!("invalid {label} payload: {err}")))
}

fn parse_positive(raw: &str, field: &str) -> HlsResult<f64> {
    let parsed = raw
        .parse::<f64>()
        .map_err(|err| HlsError::Parse(format!("{field} must be numeric: {err}")))?;

    if parsed <= 0.0 {
        return Err(HlsError::Parse(format!("{field} must be positive")));
    }

    Ok(parsed)
}

fn level_parts(
    level: Option<super::types::WsLevel>,
    label: &str,
) -> HlsResult<(Option<f64>, Option<f64>, Option<u64>)> {
    let Some(level) = level else {
        return Ok((None, None, None));
    };

    Ok((
        Some(parse_positive(&level.px, &format!("{label}.px"))?),
        Some(parse_positive(&level.sz, &format!("{label}.sz"))?),
        Some(level.n),
    ))
}
