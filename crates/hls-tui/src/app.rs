use hls_core::{
    confidence::{ConfidenceLevel, ConfidenceReason},
    market_state::{
        AdverseSelectionProxy, FeatureSnapshot, LiquidityResilienceState, StalenessState,
        TradeabilityState,
    },
    metadata::{COHORT_FRESH_LIQUIDITY, COHORT_NEW_LISTING, COHORT_UNKNOWN_METADATA},
};
use hls_screen::{ScreenEngine, ScreenRequest};

use crate::theme::{bottom_border, divider, panel_line, section_rule, top_border, truncate_chars};

pub fn render_main_table(rows: &[FeatureSnapshot]) -> String {
    render_table_with_title(rows, "READ-ONLY Hyperliquid spot live screen")
}

pub fn render_confidence_summary(rows: &[FeatureSnapshot]) -> String {
    let stats = TableStats::from_rows(rows);
    format!(
        "confidence_summary=high:{} medium:{} low:{} untrusted:{} min:{} reasons:{}",
        stats.confidence_high,
        stats.confidence_medium,
        stats.confidence_low,
        stats.confidence_untrusted,
        stats
            .min_confidence_score
            .map_or_else(|| "-".to_owned(), |score| score.to_string()),
        stats.confidence_reason_count
    )
}

pub fn render_screened_table(
    rows: &[FeatureSnapshot],
    title: &str,
    request: &ScreenRequest,
) -> hls_core::HlsResult<String> {
    let rows = ScreenEngine.apply(rows, request)?;
    Ok(render_table_with_title(&rows, title))
}

pub fn render_table_with_title(rows: &[FeatureSnapshot], title: &str) -> String {
    let stats = TableStats::from_rows(rows);
    let mut output = String::new();
    output.push_str(&top_border());
    output.push_str(&panel_line(
        "HLSCREEN",
        "Hyperliquid Microstructure Workstation",
        "READ-ONLY",
    ));
    output.push_str(&divider());
    output.push_str(&panel_line(
        "SESSION",
        &format!("{title} | PUBLIC WS/REST | local replay ready"),
        "SAFE",
    ));
    output.push_str(&panel_line(
        "UNIVERSE",
        &format!(
            "rows {} | fresh {}/{} | stale {} | incomplete {} | coverage {}",
            rows.len(),
            stats.fresh,
            rows.len(),
            stats.stale,
            stats.incomplete,
            format_ratio(stats.fresh, rows.len()),
        ),
        "LOCAL",
    ));
    output.push_str(&panel_line(
        "QUALITY",
        &format!(
            "spread med {} | depth top {} | depth total {} | top liq {}",
            format_bps(stats.median_spread_bps),
            format_usd(stats.top_tob_depth_usd),
            format_usd(stats.total_tob_depth_usd),
            format_score(stats.top_liquidity_score)
        ),
        stats.quality_status(),
    ));
    output.push_str(&panel_line(
        "LATENCY",
        &format!(
            "age med {} | age max {} | freshness-only quality | local render",
            format_age(stats.median_age_ms),
            format_age(stats.max_age_ms),
        ),
        stats.latency_status(),
    ));
    output.push_str(&panel_line(
        "CONFIDENCE",
        &format!(
            "high {} | medium {} | low {} | untrusted {} | min {} | reasons {}",
            stats.confidence_high,
            stats.confidence_medium,
            stats.confidence_low,
            stats.confidence_untrusted,
            stats
                .min_confidence_score
                .map_or_else(|| "-".to_owned(), |score| score.to_string()),
            stats.confidence_reason_count,
        ),
        stats.confidence_status(),
    ));
    output.push_str(&panel_line(
        "RESILIENCE",
        &format!(
            "tradeable {} | costly {} | thin {} | stale {} | unknown {} | brittle {} | max shock {}",
            stats.tradeable,
            stats.costly,
            stats.thin,
            stats.tradeability_stale,
            stats.tradeability_unknown,
            stats.resilience_brittle,
            format_bps(stats.max_spread_shock_bps),
        ),
        stats.resilience_status(),
    ));
    output.push_str(&panel_line(
        "METADATA",
        &format!(
            "complete {} | partial {} | missing {} | new {} | fresh liquidity {}",
            stats.metadata_complete,
            stats.metadata_partial,
            stats.metadata_missing,
            stats.metadata_new_listing,
            stats.metadata_fresh_liquidity,
        ),
        stats.metadata_status(),
    ));
    output.push_str(&bottom_border());
    output.push_str(&section_rule("MARKET BOARD"));

    if rows.is_empty() {
        output.push_str("No rows matched the current screen. Data is unchanged; adjust the read-only filter or wait for fresh public frames.\n");
        output.push_str(
            "\nNo wallet, no private streams, no order routes. Scores are screen heuristics, not orders or advice.\n",
        );
        return output;
    }

    output.push_str("#  SYMBOL        STATE      CONF   TRAD   RESIL    PRICE     SPRD    SHOCK    DEPTH    FLOW30    OFI30    SCORE    AGE   META     OBSERVATION\n");
    output.push_str("── ────────────  ─────────  ─────  ─────  ───────  ───────── ─────── ─────── ──────── ──────── ──────── ───────── ───── ──────── ───────────────────\n");

    for (index, row) in rows.iter().enumerate() {
        output.push_str(&format!(
            "{:>02} {:<12}  {:<9}  {:<5}  {:<5}  {:<7} {:>9} {:>7} {:>7} {:>8} {:>8} {:>8} {:>9} {:>5} {:<8} {}\n",
            index + 1,
            row.symbol,
            format_state(&row.staleness_state),
            format_confidence_chip(row),
            format_tradeability_chip(row.tradeability_state),
            format_resilience_chip(row.resilience_state),
            format_optional(row.price, 4),
            format_bps(row.spread_bps),
            format_bps(row.spread_shock_bps),
            format_usd(row.tob_depth_usd),
            format_signed_usd(row.signed_notional_flow_30s),
            format_signed_usd(row.bbo_ofi_proxy_30s),
            format_score_pair(row),
            format_age(row.updated_ms_ago),
            format_metadata_chip(row),
            truncate_chars(&format_row_observation(row), 28),
        ));
    }

    output.push_str(&section_rule("PAIR DETAIL CARDS"));
    for (index, row) in rows.iter().enumerate() {
        output.push_str(&render_pair_detail_card(index + 1, row));
    }

    output.push_str(
        "\nNo wallet, no private streams, no order routes. Scores are screen heuristics, not orders or advice.\n",
    );

    output
}

fn render_pair_detail_card(index: usize, row: &FeatureSnapshot) -> String {
    let mut output = String::new();
    output.push_str(&format!(
        "{index:>02} {} | px {} | 24h notional {} | {} | {} | mid {} | mark {}\n",
        row.symbol,
        format_optional(row.price, 4),
        format_usd(row.day_ntl_vlm),
        format_px_qty("bid", row.bid_px, row.bid_sz),
        format_px_qty("ask", row.ask_px, row.ask_sz),
        format_optional(row.mid_px, 4),
        format_optional(row.mark_px, 4),
    ));
    output.push_str(&format!(
        "   micro | spread {} | TOB depth {} | imbalance {} | ret {} | rv {}\n",
        format_bps(row.spread_bps),
        format_usd(row.tob_depth_usd),
        format_imbalance(row.tob_imbalance),
        format_return_triplet(row),
        format_volatility_triplet(row),
    ));
    output.push_str(&format!(
        "   activity | volume z {} | trades z {} | liq/mom/mr {} | score {} | flow30 {} | ofi30 {}\n",
        format_signed_number(row.volume_z_1h),
        format_signed_number(row.trade_count_z_1h),
        format_score_triplet(row),
        format_score_pair(row),
        format_signed_usd(row.signed_notional_flow_30s),
        format_signed_usd(row.bbo_ofi_proxy_30s),
    ));
    output.push_str(&format!(
        "   quality | {} age {} | confidence {} {} | incomplete {} | observation {}\n",
        format_state(&row.staleness_state),
        format_age(row.updated_ms_ago),
        format_confidence_level(row.confidence.level),
        row.confidence.score,
        row.incomplete_window_reason.as_deref().unwrap_or("none"),
        format_observation(row),
    ));
    output.push_str(&format!(
        "   resilience | state {} | shock {} | recovery {} | tradeability {} | adverse proxy {}\n",
        format_resilience_state(row.resilience_state),
        format_bps(row.spread_shock_bps),
        format_recovery(row.spread_recovery_ms),
        format_tradeability_state(row.tradeability_state),
        format_adverse_proxy(row.adverse_selection_proxy),
    ));
    output.push_str(&format!(
        "   flow | signed notional 30s {} | BBO OFI 30s {} | top-of-book proxy only\n",
        format_signed_usd(row.signed_notional_flow_30s),
        format_signed_usd(row.bbo_ofi_proxy_30s),
    ));
    output.push_str(&format!(
        "   metadata | {} | listing age {} | seeded {} | source {}\n",
        format_metadata_tags(row),
        format_listing_age(
            row.metadata
                .as_ref()
                .and_then(|metadata| metadata.listing_age_ms)
        ),
        format_usd(
            row.metadata
                .as_ref()
                .and_then(|metadata| metadata.seeded_usdc)
        ),
        row.metadata
            .as_ref()
            .map(|metadata| metadata.metadata_source.as_str())
            .unwrap_or("missing"),
    ));
    output.push_str(&format!(
        "   metadata detail | deployer {} | unknown fields {}\n",
        format_deployer(
            row.metadata
                .as_ref()
                .and_then(|metadata| metadata.deployer.as_deref())
        ),
        format_unknown_metadata_fields(row),
    ));
    output.push_str(&format!(
        "   confidence | {} {} | reasons {} | incomplete windows {}\n",
        format_confidence_level(row.confidence.level),
        row.confidence.score,
        format_confidence_reasons(&row.confidence.reasons),
        format_confidence_windows(&row.confidence.incomplete_windows),
    ));
    output.push_str(&format!(
        "   why ranked | {}\n",
        format_why_ranked_summary(row),
    ));
    output
}

struct TableStats {
    fresh: usize,
    stale: usize,
    incomplete: usize,
    median_spread_bps: Option<f64>,
    top_tob_depth_usd: Option<f64>,
    total_tob_depth_usd: Option<f64>,
    top_liquidity_score: Option<f64>,
    median_age_ms: Option<i64>,
    max_age_ms: Option<i64>,
    confidence_high: usize,
    confidence_medium: usize,
    confidence_low: usize,
    confidence_untrusted: usize,
    min_confidence_score: Option<u8>,
    confidence_reason_count: usize,
    tradeable: usize,
    costly: usize,
    thin: usize,
    tradeability_stale: usize,
    tradeability_unknown: usize,
    resilience_brittle: usize,
    resilience_active: usize,
    max_spread_shock_bps: Option<f64>,
    metadata_complete: usize,
    metadata_partial: usize,
    metadata_missing: usize,
    metadata_new_listing: usize,
    metadata_fresh_liquidity: usize,
}

impl TableStats {
    fn from_rows(rows: &[FeatureSnapshot]) -> Self {
        let fresh = rows
            .iter()
            .filter(|row| row.staleness_state == StalenessState::Fresh)
            .count();

        let depths = finite_values(rows.iter().filter_map(|row| row.tob_depth_usd));

        Self {
            fresh,
            stale: rows
                .iter()
                .filter(|row| row.staleness_state == StalenessState::Stale)
                .count(),
            incomplete: rows
                .iter()
                .filter(|row| row.staleness_state == StalenessState::Incomplete)
                .count(),
            median_spread_bps: median(finite_values(rows.iter().filter_map(|row| row.spread_bps))),
            top_tob_depth_usd: max_value(depths.iter().copied()),
            total_tob_depth_usd: (!depths.is_empty()).then(|| depths.iter().sum()),
            top_liquidity_score: max_value(rows.iter().map(|row| row.liquidity_score)),
            median_age_ms: median_i64(rows.iter().filter_map(|row| row.updated_ms_ago)),
            max_age_ms: rows.iter().filter_map(|row| row.updated_ms_ago).max(),
            confidence_high: rows
                .iter()
                .filter(|row| row.confidence.level == ConfidenceLevel::High)
                .count(),
            confidence_medium: rows
                .iter()
                .filter(|row| row.confidence.level == ConfidenceLevel::Medium)
                .count(),
            confidence_low: rows
                .iter()
                .filter(|row| row.confidence.level == ConfidenceLevel::Low)
                .count(),
            confidence_untrusted: rows
                .iter()
                .filter(|row| row.confidence.level == ConfidenceLevel::Untrusted)
                .count(),
            min_confidence_score: rows.iter().map(|row| row.confidence.score).min(),
            confidence_reason_count: rows.iter().map(|row| row.confidence.reasons.len()).sum(),
            tradeable: rows
                .iter()
                .filter(|row| row.tradeability_state == TradeabilityState::Tradeable)
                .count(),
            costly: rows
                .iter()
                .filter(|row| row.tradeability_state == TradeabilityState::Costly)
                .count(),
            thin: rows
                .iter()
                .filter(|row| row.tradeability_state == TradeabilityState::Thin)
                .count(),
            tradeability_stale: rows
                .iter()
                .filter(|row| row.tradeability_state == TradeabilityState::Stale)
                .count(),
            tradeability_unknown: rows
                .iter()
                .filter(|row| row.tradeability_state == TradeabilityState::Unknown)
                .count(),
            resilience_brittle: rows
                .iter()
                .filter(|row| row.resilience_state == LiquidityResilienceState::Brittle)
                .count(),
            resilience_active: rows
                .iter()
                .filter(|row| {
                    matches!(
                        row.resilience_state,
                        LiquidityResilienceState::Shock | LiquidityResilienceState::Recovering
                    )
                })
                .count(),
            max_spread_shock_bps: max_value(rows.iter().filter_map(|row| row.spread_shock_bps)),
            metadata_complete: rows
                .iter()
                .filter(|row| {
                    row.metadata
                        .as_ref()
                        .is_some_and(|metadata| metadata.is_complete())
                })
                .count(),
            metadata_partial: rows
                .iter()
                .filter(|row| {
                    row.metadata
                        .as_ref()
                        .is_some_and(|metadata| !metadata.is_complete())
                })
                .count(),
            metadata_missing: rows.iter().filter(|row| row.metadata.is_none()).count(),
            metadata_new_listing: rows
                .iter()
                .filter(|row| {
                    row.metadata
                        .as_ref()
                        .is_some_and(|metadata| metadata.has_tag(COHORT_NEW_LISTING))
                })
                .count(),
            metadata_fresh_liquidity: rows
                .iter()
                .filter(|row| {
                    row.metadata
                        .as_ref()
                        .is_some_and(|metadata| metadata.has_tag(COHORT_FRESH_LIQUIDITY))
                })
                .count(),
        }
    }

    fn quality_status(&self) -> &'static str {
        let check_quality = self.incomplete > 0
            || self.median_spread_bps.is_some_and(|spread| spread >= 100.0)
            || self.top_tob_depth_usd.is_some_and(|depth| depth < 1_000.0);
        let watch_quality = self.median_spread_bps.is_some_and(|spread| spread >= 50.0)
            || self.top_tob_depth_usd.is_some_and(|depth| depth < 5_000.0)
            || self.stale > 0;

        if check_quality {
            "CHECK"
        } else if watch_quality {
            "WATCH"
        } else {
            "GOOD"
        }
    }

    fn latency_status(&self) -> &'static str {
        match self.max_age_ms {
            Some(age) if age > 10_000 => "WATCH",
            Some(_) => "FAST",
            None => "CHECK",
        }
    }

    fn confidence_status(&self) -> &'static str {
        if self.confidence_untrusted > 0 {
            "BLOCK"
        } else if self.confidence_low > 0 {
            "CHECK"
        } else if self.confidence_medium > 0 || self.confidence_reason_count > 0 {
            "WATCH"
        } else {
            "GOOD"
        }
    }

    fn resilience_status(&self) -> &'static str {
        if self.resilience_brittle > 0 || self.tradeability_stale > 0 {
            "CHECK"
        } else if self.resilience_active > 0 || self.costly > 0 || self.thin > 0 {
            "WATCH"
        } else if self.tradeability_unknown > 0 {
            "PARTIAL"
        } else {
            "GOOD"
        }
    }

    fn metadata_status(&self) -> &'static str {
        if self.metadata_missing > 0 || self.metadata_partial > 0 {
            "PARTIAL"
        } else if self.metadata_new_listing > 0 || self.metadata_fresh_liquidity > 0 {
            "PUBLIC"
        } else {
            "READY"
        }
    }
}

fn format_optional(value: Option<f64>, decimals: usize) -> String {
    value.map_or_else(|| "-".to_owned(), |value| format!("{value:.decimals$}"))
}

fn format_bps(value: Option<f64>) -> String {
    value.map_or_else(|| "-".to_owned(), |value| format!("{value:.1} bps"))
}

fn format_usd(value: Option<f64>) -> String {
    value.map_or_else(
        || "-".to_owned(),
        |value| {
            let abs = value.abs();
            if abs >= 1_000_000_000.0 {
                format!("${:.1}B", value / 1_000_000_000.0)
            } else if abs >= 1_000_000.0 {
                format!("${:.1}M", value / 1_000_000.0)
            } else if abs >= 1_000.0 {
                format!("${:.1}K", value / 1_000.0)
            } else {
                format!("${value:.0}")
            }
        },
    )
}

fn format_signed_usd(value: Option<f64>) -> String {
    value.map_or_else(
        || "-".to_owned(),
        |value| {
            let sign = if value >= 0.0 { "+" } else { "-" };
            let formatted = format_usd(Some(value.abs()));
            format!("{sign}{formatted}")
        },
    )
}

fn format_imbalance(value: Option<f64>) -> String {
    value.map_or_else(|| "-".to_owned(), |value| format!("{:+.0}%", value * 100.0))
}

fn format_percent(value: Option<f64>) -> String {
    value.map_or_else(|| "-".to_owned(), |value| format!("{:+.2}%", value * 100.0))
}

fn format_volatility(value: Option<f64>) -> String {
    value.map_or_else(|| "-".to_owned(), |value| format!("{:.2}%", value * 100.0))
}

fn format_return_triplet(row: &FeatureSnapshot) -> String {
    format!(
        "1m {} / 5m {} / 1h {}",
        format_percent(row.ret_1m),
        format_percent(row.ret_5m),
        format_percent(row.ret_1h),
    )
}

fn format_volatility_triplet(row: &FeatureSnapshot) -> String {
    format!(
        "1m {} / 5m {} / 1h {}",
        format_volatility(row.rv_1m),
        format_volatility(row.rv_5m),
        format_volatility(row.rv_1h),
    )
}

fn format_score(value: Option<f64>) -> String {
    value.map_or_else(|| "-".to_owned(), |value| format!("{value:.1}"))
}

fn format_score_triplet(row: &FeatureSnapshot) -> String {
    format!(
        "{:.1}/{:.1}/{:.1}",
        row.liquidity_score, row.momentum_score, row.mean_reversion_score,
    )
}

fn format_score_pair(row: &FeatureSnapshot) -> String {
    row.score_breakdown.as_ref().map_or_else(
        || format!("{:.1}/{:.1}", row.liquidity_score, row.momentum_score),
        |breakdown| format!("{:.1}/{:.1}", breakdown.adjusted_total, breakdown.raw_total),
    )
}

fn format_signed_number(value: Option<f64>) -> String {
    value.map_or_else(
        || "-".to_owned(),
        |value| {
            if value.is_finite() {
                format!("{value:+.1}")
            } else {
                "-".to_owned()
            }
        },
    )
}

fn format_signed_score(value: f64) -> String {
    if !value.is_finite() {
        return "-".to_owned();
    }
    if value >= 0.0 {
        format!("+{value:.1}")
    } else {
        format!("{value:.1}")
    }
}

fn format_metadata_chip(row: &FeatureSnapshot) -> String {
    let Some(metadata) = &row.metadata else {
        return "UNKNOWN".to_owned();
    };
    if metadata.has_tag(COHORT_NEW_LISTING) && metadata.has_tag(COHORT_FRESH_LIQUIDITY) {
        "NEW+SEED".to_owned()
    } else if metadata.has_tag(COHORT_NEW_LISTING) {
        "NEW".to_owned()
    } else if metadata.has_tag(COHORT_FRESH_LIQUIDITY) {
        "SEEDED".to_owned()
    } else if metadata.has_tag(COHORT_UNKNOWN_METADATA) {
        "UNKNOWN".to_owned()
    } else {
        "PUBLIC".to_owned()
    }
}

fn format_metadata_tags(row: &FeatureSnapshot) -> String {
    match &row.metadata {
        Some(metadata) => format!("tags {}", metadata.cohort_label()),
        None => "tags unknown_metadata".to_owned(),
    }
}

fn format_unknown_metadata_fields(row: &FeatureSnapshot) -> String {
    row.metadata
        .as_ref()
        .map(|metadata| {
            if metadata.unknown_fields.is_empty() {
                "none".to_owned()
            } else {
                metadata.unknown_fields.join(",")
            }
        })
        .unwrap_or_else(|| "all".to_owned())
}

fn format_why_ranked_summary(row: &FeatureSnapshot) -> String {
    row.score_breakdown.as_ref().map_or_else(
        || {
            format!(
                "score {} adjusted from raw - | confidence penalty - | components 0",
                format_score_pair(row),
            )
        },
        |breakdown| {
            format!(
                "score {} adjusted from raw {} | confidence penalty {} | components {}",
                format_score(Some(breakdown.adjusted_total)),
                format_score(Some(breakdown.raw_total)),
                format_signed_score(breakdown.confidence_penalty()),
                breakdown.components.len(),
            )
        },
    )
}

fn format_listing_age(value: Option<i64>) -> String {
    value.map_or_else(
        || "-".to_owned(),
        |value| {
            let value = value.max(0);
            if value < 60 * 60 * 1_000 {
                format!("{:.0}m", value as f64 / (60.0 * 1_000.0))
            } else if value < 48 * 60 * 60 * 1_000 {
                format!("{:.1}h", value as f64 / (60.0 * 60.0 * 1_000.0))
            } else {
                format!("{:.1}d", value as f64 / (24.0 * 60.0 * 60.0 * 1_000.0))
            }
        },
    )
}

fn format_deployer(value: Option<&str>) -> String {
    value.map_or_else(
        || "-".to_owned(),
        |value| {
            if value.chars().count() <= 14 {
                value.to_owned()
            } else {
                let prefix: String = value.chars().take(8).collect();
                let suffix: String = value
                    .chars()
                    .rev()
                    .take(6)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect();
                format!("{prefix}…{suffix}")
            }
        },
    )
}

fn format_confidence_chip(row: &FeatureSnapshot) -> String {
    let prefix = match row.confidence.level {
        ConfidenceLevel::High => "H",
        ConfidenceLevel::Medium => "M",
        ConfidenceLevel::Low => "L",
        ConfidenceLevel::Untrusted => "U",
    };
    format!("{prefix}{:03}", row.confidence.score)
}

fn format_tradeability_chip(state: TradeabilityState) -> &'static str {
    match state {
        TradeabilityState::Unknown => "UNK",
        TradeabilityState::Tradeable => "TRADE",
        TradeabilityState::Costly => "COST",
        TradeabilityState::Thin => "THIN",
        TradeabilityState::Stale => "STALE",
    }
}

fn format_tradeability_state(state: TradeabilityState) -> &'static str {
    state.as_str()
}

fn format_resilience_chip(state: LiquidityResilienceState) -> &'static str {
    match state {
        LiquidityResilienceState::Unknown => "UNK",
        LiquidityResilienceState::Normal => "NORMAL",
        LiquidityResilienceState::Shock => "SHOCK",
        LiquidityResilienceState::Recovering => "RECOV",
        LiquidityResilienceState::Brittle => "BRITTLE",
    }
}

fn format_resilience_state(state: LiquidityResilienceState) -> &'static str {
    state.as_str()
}

fn format_adverse_proxy(state: AdverseSelectionProxy) -> &'static str {
    state.as_str()
}

fn format_recovery(value: Option<i64>) -> String {
    value.map_or_else(
        || "-".to_owned(),
        |value| {
            if value < 1_000 {
                format!("{value}ms")
            } else {
                format!("{:.1}s", value as f64 / 1_000.0)
            }
        },
    )
}

fn format_confidence_level(level: ConfidenceLevel) -> &'static str {
    match level {
        ConfidenceLevel::High => "high",
        ConfidenceLevel::Medium => "medium",
        ConfidenceLevel::Low => "low",
        ConfidenceLevel::Untrusted => "untrusted",
    }
}

fn format_confidence_reason(reason: ConfidenceReason) -> &'static str {
    match reason {
        ConfidenceReason::ReconnectGap => "reconnect_gap",
        ConfidenceReason::StaleQuote => "stale_quote",
        ConfidenceReason::SparseTrades => "sparse_trades",
        ConfidenceReason::DuplicateEvents => "duplicate_events",
        ConfidenceReason::ParserDrops => "parser_drops",
        ConfidenceReason::WriterBacklog => "writer_backlog",
        ConfidenceReason::IncompleteWindow => "incomplete_window",
    }
}

fn format_confidence_reasons(reasons: &[ConfidenceReason]) -> String {
    if reasons.is_empty() {
        return "none".to_owned();
    }

    reasons
        .iter()
        .map(|reason| format_confidence_reason(*reason))
        .collect::<Vec<_>>()
        .join(",")
}

fn format_confidence_windows(windows: &[String]) -> String {
    if windows.is_empty() {
        "none".to_owned()
    } else {
        windows.join(",")
    }
}

fn format_px_qty(label: &str, px: Option<f64>, qty: Option<f64>) -> String {
    match (px, qty) {
        (Some(px), Some(qty)) => format!("{label} {px:.4} x {qty:.4}"),
        (Some(px), None) => format!("{label} {px:.4} x -"),
        _ => format!("{label} -"),
    }
}

fn format_ratio(numerator: usize, denominator: usize) -> String {
    if denominator == 0 {
        return "0%".to_owned();
    }

    format!("{:.0}%", (numerator as f64 / denominator as f64) * 100.0)
}

fn format_age(value: Option<i64>) -> String {
    value.map_or_else(
        || "-".to_owned(),
        |value| {
            let value = value.max(0);
            if value < 1_000 {
                format!("{value}ms")
            } else {
                format!("{:.1}s", value as f64 / 1_000.0)
            }
        },
    )
}

fn format_state(state: &StalenessState) -> &'static str {
    match state {
        StalenessState::Fresh => "● fresh",
        StalenessState::Stale => "▲ stale",
        StalenessState::Incomplete => "○ partial",
    }
}

fn format_observation(row: &FeatureSnapshot) -> String {
    let parts = observation_parts(row);
    if parts.is_empty() {
        "steady".to_owned()
    } else {
        parts.join(" · ")
    }
}

fn format_row_observation(row: &FeatureSnapshot) -> String {
    let parts = observation_parts(row);
    if parts.is_empty() {
        "steady".to_owned()
    } else {
        parts.into_iter().take(2).collect::<Vec<_>>().join(" · ")
    }
}

fn observation_parts(row: &FeatureSnapshot) -> Vec<String> {
    let mut parts = Vec::new();

    if matches!(row.staleness_state, StalenessState::Stale) {
        parts.push("stale feed".to_owned());
    } else if matches!(row.staleness_state, StalenessState::Incomplete) {
        parts.push("partial data".to_owned());
    }

    match row.confidence.level {
        ConfidenceLevel::Low => parts.push("low confidence".to_owned()),
        ConfidenceLevel::Untrusted => parts.push("untrusted data".to_owned()),
        ConfidenceLevel::High | ConfidenceLevel::Medium => {}
    }

    match row.tradeability_state {
        TradeabilityState::Thin => parts.push("thin tradeability".to_owned()),
        TradeabilityState::Costly => parts.push("costly tradeability".to_owned()),
        TradeabilityState::Stale => parts.push("stale tradeability".to_owned()),
        TradeabilityState::Unknown | TradeabilityState::Tradeable => {}
    }

    match row.resilience_state {
        LiquidityResilienceState::Shock => parts.push("spread shock".to_owned()),
        LiquidityResilienceState::Recovering => parts.push("recovering book".to_owned()),
        LiquidityResilienceState::Brittle => parts.push("brittle book".to_owned()),
        LiquidityResilienceState::Unknown | LiquidityResilienceState::Normal => {}
    }

    match row.adverse_selection_proxy {
        AdverseSelectionProxy::Watch => parts.push("flow watch".to_owned()),
        AdverseSelectionProxy::Brittle => parts.push("adverse proxy".to_owned()),
        AdverseSelectionProxy::Unknown | AdverseSelectionProxy::Normal => {}
    }

    if row.tob_depth_usd.is_some_and(|depth| depth < 1_000.0) {
        parts.push("thin book".to_owned());
    }
    if row.spread_bps.is_some_and(|spread| spread >= 50.0) {
        parts.push("wide spread".to_owned());
    } else if row.spread_bps.is_some_and(|spread| spread <= 10.0) {
        parts.push("tight spread".to_owned());
    }
    if row.ret_1m.is_some_and(|ret| ret.abs() >= 0.005) {
        parts.push("move active".to_owned());
    }
    if row
        .tob_imbalance
        .is_some_and(|imbalance| imbalance.abs() >= 0.4)
    {
        parts.push("imbalanced".to_owned());
    }
    if let Some(metadata) = &row.metadata {
        if metadata.has_tag(COHORT_NEW_LISTING) {
            parts.push("new listing".to_owned());
        }
        if metadata.has_tag(COHORT_FRESH_LIQUIDITY) {
            parts.push("fresh liquidity".to_owned());
        }
        if metadata.has_tag(COHORT_UNKNOWN_METADATA) {
            parts.push("metadata partial".to_owned());
        }
    }

    parts
}

fn finite_values(values: impl Iterator<Item = f64>) -> Vec<f64> {
    values.filter(|value| value.is_finite()).collect()
}

fn median(mut values: Vec<f64>) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(f64::total_cmp);
    let mid = values.len() / 2;
    if values.len() % 2 == 0 {
        Some((values[mid - 1] + values[mid]) / 2.0)
    } else {
        Some(values[mid])
    }
}

fn median_i64(values: impl Iterator<Item = i64>) -> Option<i64> {
    let mut values: Vec<_> = values.collect();
    if values.is_empty() {
        return None;
    }
    values.sort_unstable();
    let mid = values.len() / 2;
    if values.len() % 2 == 0 {
        Some((values[mid - 1] + values[mid]) / 2)
    } else {
        Some(values[mid])
    }
}

fn max_value(values: impl Iterator<Item = f64>) -> Option<f64> {
    values
        .filter(|value| value.is_finite())
        .max_by(f64::total_cmp)
}
