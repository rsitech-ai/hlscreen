use std::collections::{BTreeMap, HashMap, HashSet};

use hls_core::{
    HlsError, HlsResult,
    market_state::{CandleEvent, CompositeCandle, CompositeCoverageState, CompositeVolumeSource},
};

const INITIAL_COMPOSITE_VALUE: f64 = 100.0;
const MAX_CONSTITUENT_WEIGHT: f64 = 0.10;
const HEALTHY_COVERAGE: f64 = 0.80;
const PARTIAL_COVERAGE: f64 = 0.50;

pub fn build_market_composite(
    candles: &[CandleEvent],
    liquidity_by_symbol: &HashMap<String, f64>,
    requested_symbols: usize,
) -> HlsResult<Vec<CompositeCandle>> {
    build_market_composite_with_exact_volume(
        candles,
        liquidity_by_symbol,
        &HashMap::new(),
        requested_symbols,
    )
}

pub fn build_market_composite_with_exact_volume(
    candles: &[CandleEvent],
    liquidity_by_symbol: &HashMap<String, f64>,
    exact_quote_volume_by_open_ts: &HashMap<i64, f64>,
    requested_symbols: usize,
) -> HlsResult<Vec<CompositeCandle>> {
    if requested_symbols == 0 {
        return Err(HlsError::Config(
            "market composite requires at least one requested symbol".to_owned(),
        ));
    }

    let weights = capped_sqrt_weights(liquidity_by_symbol)?;
    let mut latest_by_bucket = BTreeMap::<(i64, String), &CandleEvent>::new();
    for candle in candles
        .iter()
        .filter(|candle| valid_one_minute_candle(candle))
    {
        let key = (candle.open_ts_ms, candle.hl_coin.clone());
        let replace = latest_by_bucket
            .get(&key)
            .is_none_or(|existing| candle.recv_ts_ns >= existing.recv_ts_ns);
        if replace {
            latest_by_bucket.insert(key, candle);
        }
    }

    let mut buckets = BTreeMap::<i64, Vec<&CandleEvent>>::new();
    for ((open_ts_ms, _), candle) in latest_by_bucket {
        buckets.entry(open_ts_ms).or_default().push(candle);
    }

    let mut previous_symbol_close = HashMap::<String, f64>::new();
    let mut previous_composite_close = INITIAL_COMPOSITE_VALUE;
    let mut output = Vec::with_capacity(buckets.len());

    for (open_ts_ms, mut bucket) in buckets {
        bucket.sort_by(|left, right| left.hl_coin.cmp(&right.hl_coin));
        let coverage = bucket
            .iter()
            .filter_map(|candle| weights.get(&candle.hl_coin))
            .sum::<f64>();
        if coverage <= 0.0 || !coverage.is_finite() {
            continue;
        }

        let mut open_return = 0.0;
        let mut high_return = 0.0;
        let mut low_return = 0.0;
        let mut close_return = 0.0;
        let mut quote_volume = 0.0;
        let mut contributing_symbols = HashSet::new();
        let mut advances = 0;
        let mut declines = 0;
        let mut unchanged = 0;
        let mut close_ts_ms = open_ts_ms;

        for candle in bucket {
            let Some(global_weight) = weights.get(&candle.hl_coin).copied() else {
                continue;
            };
            let weight = global_weight / coverage;
            let previous_close = previous_symbol_close
                .get(&candle.hl_coin)
                .copied()
                .unwrap_or(candle.open);
            if !previous_close.is_finite() || previous_close <= 0.0 {
                continue;
            }

            open_return += weight * (candle.open / previous_close - 1.0);
            high_return += weight * (candle.high / previous_close - 1.0);
            low_return += weight * (candle.low / previous_close - 1.0);
            close_return += weight * (candle.close / previous_close - 1.0);
            quote_volume += candle.volume_base * candle.close;
            close_ts_ms = close_ts_ms.max(candle.close_ts_ms);
            contributing_symbols.insert(candle.hl_coin.clone());
            previous_symbol_close.insert(candle.hl_coin.clone(), candle.close);

            if candle.close > candle.open {
                advances += 1;
            } else if candle.close < candle.open {
                declines += 1;
            } else {
                unchanged += 1;
            }
        }

        if contributing_symbols.is_empty() {
            continue;
        }

        let open = previous_composite_close * (1.0 + open_return);
        let close = previous_composite_close * (1.0 + close_return);
        let high = (previous_composite_close * (1.0 + high_return)).max(open.max(close));
        let low = (previous_composite_close * (1.0 + low_return)).min(open.min(close));
        if ![open, high, low, close, quote_volume]
            .into_iter()
            .all(f64::is_finite)
            || open <= 0.0
            || high <= 0.0
            || low <= 0.0
            || close <= 0.0
            || quote_volume < 0.0
        {
            return Err(HlsError::External(format!(
                "market composite produced invalid values for bucket {open_ts_ms}"
            )));
        }

        let (quote_volume, volume_source) = exact_quote_volume_by_open_ts
            .get(&open_ts_ms)
            .copied()
            .filter(|volume| volume.is_finite() && *volume >= 0.0)
            .map(|volume| (volume, CompositeVolumeSource::ExactTrades))
            .unwrap_or((quote_volume, CompositeVolumeSource::CloseApproximation));
        let liquidity_weight_coverage = coverage.clamp(0.0, 1.0);
        let contributing_symbol_count = contributing_symbols.len();
        output.push(CompositeCandle {
            open_ts_ms,
            close_ts_ms,
            open,
            high,
            low,
            close,
            quote_volume,
            volume_source,
            contributing_symbols: contributing_symbol_count,
            requested_symbols,
            liquidity_weight_coverage,
            coverage_state: coverage_state(liquidity_weight_coverage),
            advances,
            declines,
            unchanged,
            stale_symbols: requested_symbols.saturating_sub(contributing_symbol_count),
        });
        previous_composite_close = close;
    }

    Ok(output)
}

fn capped_sqrt_weights(
    liquidity_by_symbol: &HashMap<String, f64>,
) -> HlsResult<BTreeMap<String, f64>> {
    let raw = liquidity_by_symbol
        .iter()
        .filter(|(_, liquidity)| liquidity.is_finite() && **liquidity > 0.0)
        .map(|(symbol, liquidity)| (symbol.clone(), liquidity.sqrt()))
        .collect::<BTreeMap<_, _>>();
    if raw.is_empty() {
        return Err(HlsError::Config(
            "market composite requires positive finite liquidity weights".to_owned(),
        ));
    }

    let effective_cap = MAX_CONSTITUENT_WEIGHT.max(1.0 / raw.len() as f64);
    let mut uncapped = raw.keys().cloned().collect::<HashSet<_>>();
    let mut weights = BTreeMap::new();
    let mut remaining_mass = 1.0;

    loop {
        let remaining_raw = uncapped
            .iter()
            .filter_map(|symbol| raw.get(symbol))
            .sum::<f64>();
        if uncapped.is_empty() || remaining_raw <= 0.0 {
            break;
        }
        let newly_capped = uncapped
            .iter()
            .filter(|symbol| {
                raw.get(*symbol)
                    .is_some_and(|value| remaining_mass * value / remaining_raw > effective_cap)
            })
            .cloned()
            .collect::<Vec<_>>();
        if newly_capped.is_empty() {
            for symbol in &uncapped {
                let weight = remaining_mass * raw[symbol] / remaining_raw;
                weights.insert(symbol.clone(), weight);
            }
            break;
        }
        for symbol in newly_capped {
            uncapped.remove(&symbol);
            weights.insert(symbol, effective_cap);
            remaining_mass = (remaining_mass - effective_cap).max(0.0);
        }
    }

    let total = weights.values().sum::<f64>();
    if !total.is_finite() || total <= 0.0 {
        return Err(HlsError::External(
            "market composite weight normalization failed".to_owned(),
        ));
    }
    for weight in weights.values_mut() {
        *weight /= total;
    }
    Ok(weights)
}

fn valid_one_minute_candle(candle: &CandleEvent) -> bool {
    candle.interval == "1m"
        && candle.open_ts_ms >= 0
        && candle.close_ts_ms >= candle.open_ts_ms
        && candle.open.is_finite()
        && candle.high.is_finite()
        && candle.low.is_finite()
        && candle.close.is_finite()
        && candle.volume_base.is_finite()
        && candle.open > 0.0
        && candle.high >= candle.open.max(candle.close)
        && candle.low <= candle.open.min(candle.close)
        && candle.low > 0.0
        && candle.volume_base >= 0.0
}

fn coverage_state(coverage: f64) -> CompositeCoverageState {
    if coverage >= HEALTHY_COVERAGE {
        CompositeCoverageState::Healthy
    } else if coverage >= PARTIAL_COVERAGE {
        CompositeCoverageState::Partial
    } else {
        CompositeCoverageState::Degraded
    }
}
