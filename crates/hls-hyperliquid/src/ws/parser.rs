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
        "activeAssetCtx" | "activeSpotAssetCtx" => {
            parse_active_asset_ctx(envelope.data).map(|event| vec![event])
        }
        "candle" => parse_candles(envelope.data),
        "subscriptionResponse" | "pong" => Ok(Vec::new()),
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
            if trade.time < 0 {
                return Err(HlsError::Parse(
                    "trade.time must be non-negative".to_owned(),
                ));
            }
            let side = match trade.side.as_str() {
                "B" | "buy" | "Buy" => TradeSide::Buy,
                "A" | "sell" | "Sell" => TradeSide::Sell,
                other => return Err(HlsError::Parse(format!("unsupported trade side '{other}'"))),
            };
            let price = parse_positive(&trade.px, "trade.px")?;
            let size = parse_positive(&trade.sz, "trade.sz")?;
            let notional = price * size;
            if !notional.is_finite() {
                return Err(HlsError::Parse("trade notional must be finite".to_owned()));
            }
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
    if bbo.time < 0 {
        return Err(HlsError::Parse("bbo.time must be non-negative".to_owned()));
    }
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
        day_ntl_vlm: parse_optional_non_negative(ctx.ctx.day_ntl_vlm, "assetCtx.dayNtlVlm")?,
        prev_day_px: parse_optional_positive(ctx.ctx.prev_day_px, "assetCtx.prevDayPx")?,
        mark_px: parse_optional_positive(ctx.ctx.mark_px, "assetCtx.markPx")?,
        mid_px: parse_optional_positive(ctx.ctx.mid_px, "assetCtx.midPx")?,
        circulating_supply: parse_optional_non_negative(
            ctx.ctx.circulating_supply,
            "assetCtx.circulatingSupply",
        )?,
    }))
}

fn parse_candles(data: serde_json::Value) -> HlsResult<Vec<MarketEvent>> {
    let candles = if data.is_array() {
        parse_json::<Vec<WsCandle>>(data, "candle")?
    } else {
        vec![parse_json::<WsCandle>(data, "candle")?]
    };

    candles
        .into_iter()
        .map(|candle| {
            let open_ts_ms = parse_required_i64(candle.t, "candle.t")?;
            let close_ts_ms = parse_required_i64(candle.close_ms, "candle.T")?;
            let open = parse_required_f64(candle.o, "candle.o")?;
            let close = parse_required_f64(candle.c, "candle.c")?;
            let high = parse_required_f64(candle.h, "candle.h")?;
            let low = parse_required_f64(candle.l, "candle.l")?;
            let volume_base = parse_required_f64(candle.v, "candle.v")?;
            let trade_count = parse_required_u64(candle.n, "candle.n")?;

            if open_ts_ms > close_ts_ms {
                return Err(HlsError::Parse(
                    "candle open time must be <= close time".to_owned(),
                ));
            }
            if open_ts_ms < 0 || close_ts_ms < 0 {
                return Err(HlsError::Parse(
                    "candle timestamps must be non-negative".to_owned(),
                ));
            }
            if open <= 0.0 || close <= 0.0 || high <= 0.0 || low <= 0.0 {
                return Err(HlsError::Parse(
                    "candle OHLC values must be positive".to_owned(),
                ));
            }
            if high < low || high < open || high < close || low > open || low > close {
                return Err(HlsError::Parse(
                    "candle OHLC values are internally inconsistent".to_owned(),
                ));
            }
            if volume_base < 0.0 {
                return Err(HlsError::Parse("candle.v must be non-negative".to_owned()));
            }

            Ok(MarketEvent::Candle(CandleEvent {
                recv_ts_ns: 0,
                open_ts_ms,
                close_ts_ms,
                hl_coin: candle.s,
                interval: candle.i,
                open,
                high,
                low,
                close,
                volume_base,
                trade_count,
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

    if !parsed.is_finite() {
        return Err(HlsError::Parse(format!("{field} must be finite")));
    }
    if parsed <= 0.0 {
        return Err(HlsError::Parse(format!("{field} must be positive")));
    }

    Ok(parsed)
}

fn parse_optional_number(raw: Option<serde_json::Value>, field: &str) -> HlsResult<Option<f64>> {
    let Some(raw) = raw else {
        return Ok(None);
    };

    let parsed = match raw {
        serde_json::Value::Null => Ok(None),
        serde_json::Value::Number(number) => number
            .as_f64()
            .ok_or_else(|| HlsError::Parse(format!("{field} is not representable as f64")))
            .map(Some),
        serde_json::Value::String(text) if text.trim().is_empty() => Ok(None),
        serde_json::Value::String(text) => text
            .parse::<f64>()
            .map(Some)
            .map_err(|err| HlsError::Parse(format!("{field} must be numeric: {err}"))),
        other => Err(HlsError::Parse(format!(
            "{field} must be numeric string, number, or null; got {other}"
        ))),
    }?;

    match parsed {
        Some(value) if !value.is_finite() => {
            Err(HlsError::Parse(format!("{field} must be finite")))
        }
        _ => Ok(parsed),
    }
}

fn parse_optional_positive(raw: Option<serde_json::Value>, field: &str) -> HlsResult<Option<f64>> {
    let parsed = parse_optional_number(raw, field)?;
    if parsed.is_some_and(|value| value < 0.0) {
        return Err(HlsError::Parse(format!(
            "{field} must be positive or a zero missing-value sentinel"
        )));
    }
    Ok(parsed.filter(|value| *value > 0.0))
}

fn parse_optional_non_negative(
    raw: Option<serde_json::Value>,
    field: &str,
) -> HlsResult<Option<f64>> {
    let parsed = parse_optional_number(raw, field)?;
    if parsed.is_some_and(|value| value < 0.0) {
        return Err(HlsError::Parse(format!("{field} must be non-negative")));
    }
    Ok(parsed)
}

fn parse_required_f64(raw: serde_json::Value, field: &str) -> HlsResult<f64> {
    parse_optional_number(Some(raw), field)?
        .ok_or_else(|| HlsError::Parse(format!("{field} must not be null")))
}

fn parse_required_i64(raw: serde_json::Value, field: &str) -> HlsResult<i64> {
    match raw {
        serde_json::Value::Number(number) => number
            .as_i64()
            .ok_or_else(|| HlsError::Parse(format!("{field} is not representable as i64"))),
        serde_json::Value::String(text) => text
            .parse::<i64>()
            .map_err(|err| HlsError::Parse(format!("{field} must be an integer: {err}"))),
        other => Err(HlsError::Parse(format!(
            "{field} must be integer string or number; got {other}"
        ))),
    }
}

fn parse_required_u64(raw: serde_json::Value, field: &str) -> HlsResult<u64> {
    match raw {
        serde_json::Value::Number(number) => number
            .as_u64()
            .ok_or_else(|| HlsError::Parse(format!("{field} is not representable as u64"))),
        serde_json::Value::String(text) => text
            .parse::<u64>()
            .map_err(|err| HlsError::Parse(format!("{field} must be an unsigned integer: {err}"))),
        other => Err(HlsError::Parse(format!(
            "{field} must be unsigned integer string or number; got {other}"
        ))),
    }
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
