use hls_core::{
    confidence::{ConfidenceLevel, ConfidenceReason},
    market_state::{
        AdverseSelectionProxy, FeatureSnapshot, LiquidityResilienceState, StalenessState,
        TradeabilityState,
    },
    metadata::{COHORT_FRESH_LIQUIDITY, COHORT_NEW_LISTING, COHORT_UNKNOWN_METADATA},
};
use hls_screen::{ScreenEngine, ScreenRequest, presets::find_preset};

use crate::interaction::{WorkstationDensity, WorkstationUiState, WorkstationView};
use crate::theme::truncate_chars;

#[derive(Clone, Copy)]
enum Align {
    Left,
    Right,
}

#[derive(Clone, Copy)]
enum ColumnKind {
    Symbol,
    Price,
    Spread,
    Imbalance,
    Flow30,
    Rv5m,
    LiquidityCost,
    Confidence,
    WhyNow,
}

#[derive(Clone, Copy)]
struct WorkstationColumn {
    label: &'static str,
    width: usize,
    align: Align,
    kind: ColumnKind,
}

const WIDE_COLUMNS: [WorkstationColumn; 9] = [
    column("symbol", 12, Align::Left, ColumnKind::Symbol),
    column("price", 9, Align::Right, ColumnKind::Price),
    column("sprbp", 7, Align::Right, ColumnKind::Spread),
    column("imb", 7, Align::Right, ColumnKind::Imbalance),
    column("flow30", 8, Align::Right, ColumnKind::Flow30),
    column("rv5m", 7, Align::Right, ColumnKind::Rv5m),
    column("cost", 6, Align::Left, ColumnKind::LiquidityCost),
    column("conf", 6, Align::Right, ColumnKind::Confidence),
    column("why now", 17, Align::Left, ColumnKind::WhyNow),
];

const COMPACT_COLUMNS: [WorkstationColumn; 9] = [
    column("symbol", 11, Align::Left, ColumnKind::Symbol),
    column("price", 8, Align::Right, ColumnKind::Price),
    column("spr", 5, Align::Right, ColumnKind::Spread),
    column("imb", 5, Align::Right, ColumnKind::Imbalance),
    column("flow", 7, Align::Right, ColumnKind::Flow30),
    column("rv", 5, Align::Right, ColumnKind::Rv5m),
    column("cost", 5, Align::Left, ColumnKind::LiquidityCost),
    column("c", 4, Align::Right, ColumnKind::Confidence),
    column("why", 10, Align::Left, ColumnKind::WhyNow),
];

const NARROW_COLUMNS: [WorkstationColumn; 6] = [
    column("symbol", 11, Align::Left, ColumnKind::Symbol),
    column("price", 8, Align::Right, ColumnKind::Price),
    column("spr", 5, Align::Right, ColumnKind::Spread),
    column("flow", 7, Align::Right, ColumnKind::Flow30),
    column("c", 4, Align::Right, ColumnKind::Confidence),
    column("why", 13, Align::Left, ColumnKind::WhyNow),
];

const MINI_COLUMNS: [WorkstationColumn; 5] = [
    column("sym", 8, Align::Left, ColumnKind::Symbol),
    column("px", 7, Align::Right, ColumnKind::Price),
    column("spr", 4, Align::Right, ColumnKind::Spread),
    column("c", 3, Align::Right, ColumnKind::Confidence),
    column("why", 7, Align::Left, ColumnKind::WhyNow),
];

const TINY_COLUMNS: [WorkstationColumn; 3] = [
    column("sym", 8, Align::Left, ColumnKind::Symbol),
    column("px", 7, Align::Right, ColumnKind::Price),
    column("why", 7, Align::Left, ColumnKind::WhyNow),
];

const fn column(
    label: &'static str,
    width: usize,
    align: Align,
    kind: ColumnKind,
) -> WorkstationColumn {
    WorkstationColumn {
        label,
        width,
        align,
        kind,
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RenderOptions {
    terminal_width: Option<usize>,
}

impl RenderOptions {
    pub fn for_width(width: usize) -> Self {
        Self {
            terminal_width: Some(width),
        }
    }

    pub fn for_live_terminal_width(width: usize) -> Self {
        Self::for_width(width.saturating_sub(8).min(96))
    }
}

#[derive(Clone, Copy)]
struct RenderLayout {
    columns: &'static [WorkstationColumn],
    width: usize,
    bounded_detail: bool,
}

impl RenderLayout {
    fn from_options(options: RenderOptions) -> Self {
        let columns: &'static [WorkstationColumn] = match options.terminal_width {
            Some(width) if width < column_width(&MINI_COLUMNS) => &TINY_COLUMNS,
            Some(width) if width < column_width(&NARROW_COLUMNS) => &MINI_COLUMNS,
            Some(width) if width < column_width(&COMPACT_COLUMNS) => &NARROW_COLUMNS,
            Some(width) if width < column_width(&WIDE_COLUMNS) => &COMPACT_COLUMNS,
            _ => &WIDE_COLUMNS,
        };

        Self {
            columns,
            width: column_width(columns),
            bounded_detail: options.terminal_width.is_some(),
        }
    }
}

fn column_width(columns: &[WorkstationColumn]) -> usize {
    1 + columns.iter().map(|column| column.width + 3).sum::<usize>()
}

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
    Ok(render_workstation(
        &rows,
        title,
        Some(request),
        None,
        RenderOptions::default(),
    ))
}

pub fn render_screened_table_with_state(
    rows: &[FeatureSnapshot],
    title: &str,
    request: &ScreenRequest,
    ui_state: &WorkstationUiState,
) -> hls_core::HlsResult<String> {
    let rows = ScreenEngine.apply(rows, request)?;
    Ok(render_workstation(
        &rows,
        title,
        Some(request),
        Some(ui_state),
        RenderOptions::default(),
    ))
}

pub fn render_screened_table_with_options(
    rows: &[FeatureSnapshot],
    title: &str,
    request: &ScreenRequest,
    ui_state: Option<&WorkstationUiState>,
    options: RenderOptions,
) -> hls_core::HlsResult<String> {
    let rows = ScreenEngine.apply(rows, request)?;
    Ok(render_workstation(
        &rows,
        title,
        Some(request),
        ui_state,
        options,
    ))
}

pub fn render_table_with_title(rows: &[FeatureSnapshot], title: &str) -> String {
    render_workstation(rows, title, None, None, RenderOptions::default())
}

fn render_workstation(
    rows: &[FeatureSnapshot],
    title: &str,
    request: Option<&ScreenRequest>,
    ui_state: Option<&WorkstationUiState>,
    options: RenderOptions,
) -> String {
    let layout = RenderLayout::from_options(options);
    let stats = TableStats::from_rows(rows);
    let mut output = String::new();
    let selected_index = selected_index(rows, ui_state);
    let view = ui_state.map_or(WorkstationView::Overview, WorkstationUiState::view);
    let density = ui_state.map_or(WorkstationDensity::Dense, WorkstationUiState::density);

    let stream_status = if title.to_ascii_lowercase().contains("replay") {
        "REPLAY ●"
    } else {
        "LIVE ●"
    };
    let ui_status = ui_state
        .map(|state| {
            if state.paused() {
                "UI PAUSED"
            } else {
                "UI ACTIVE"
            }
        })
        .unwrap_or("UI READY");
    let recorder_status = if title.to_ascii_lowercase().contains("recording") {
        "REC ●"
    } else {
        "REC ready"
    };
    let status = format!(
        "{recorder_status}  {stream_status}  {ui_status}  p95 row age {}",
        format_age(stats.p95_age_ms)
    );

    output.push_str(&workstation_top_line(
        "Hyperliquid Spot Microstructure Workstation",
        &status,
        layout,
    ));
    output.push_str(&workstation_full_line(
        &format!("filter: {}", filter_label(title, request)),
        &format!(
            "mode: {} | quality {}",
            mode_label(rows.len(), request),
            stats.quality_status().to_ascii_lowercase()
        ),
        layout,
    ));
    output.push_str(&workstation_full_line(
        &format!(
            "ui: {} · row {} · {} · n={}",
            view.label(),
            focus_label(selected_index, rows.len()),
            density_short_label(density),
            rows.len()
        ),
        if layout.bounded_detail {
            "keys j/k tab d ? sp q"
        } else {
            "keys arrows/jk · tab · d · ? · space · q"
        },
        layout,
    ));
    if let Some(state) = ui_state
        && state.help_open()
    {
        output.push_str(&workstation_full_line(
            "command deck: ↑/↓ row · Enter detail · h health · PgUp/PgDn jump · Shift+Tab previous view",
            "",
            layout,
        ));
        output.push_str(&workstation_full_line(
            "display only: controls change focus, detail, health, density, help, and view; ingestion stays public/read-only",
            "",
            layout,
        ));
    }
    output.push_str(&workstation_border("├", "┬", "┤", layout));
    output.push_str(&workstation_header_row(layout));
    output.push_str(&workstation_border("├", "┼", "┤", layout));

    if rows.is_empty() {
        output.push_str(&workstation_full_line(
            "No rows matched the current read-only screen.",
            "wait for public frames or adjust filter",
            layout,
        ));
        output.push_str(&workstation_border("└", "┴", "┘", layout));
        push_boundary_caveat(&mut output, layout);
        return output;
    }

    let visible = visible_range(rows.len(), selected_index.unwrap_or_default(), ui_state);
    for (row_index, row) in rows
        .iter()
        .enumerate()
        .take(visible.end)
        .skip(visible.start)
    {
        output.push_str(&workstation_data_row(
            row,
            selected_index == Some(row_index),
            ui_state.is_some(),
            layout,
        ));
    }

    output.push_str(&workstation_border("└", "┴", "┘", layout));

    let selected = &rows[selected_index.unwrap_or_default()];
    output.push('\n');
    render_selected_detail(&mut output, selected, view, layout);

    push_boundary_caveat(&mut output, layout);

    output
}

fn workstation_top_line(title: &str, status: &str, layout: RenderLayout) -> String {
    let mut left = format!(" {title} ");
    let mut right = format!(" {status} ");
    let inner_width = layout.width - 2;
    if char_count(&left) + char_count(&right) > inner_width {
        let right_width = (inner_width / 2).min(char_count(&right));
        right = truncate_chars(&right, right_width);
        let left_width = inner_width.saturating_sub(char_count(&right));
        left = truncate_chars(&left, left_width);
    }
    let fill_width = inner_width.saturating_sub(char_count(&left) + char_count(&right));
    format!("┌{}{}{}┐\n", left, "─".repeat(fill_width), right)
}

fn workstation_full_line(left: &str, right: &str, layout: RenderLayout) -> String {
    let inner_width = layout.width - 4;
    let right = if right.is_empty() {
        String::new()
    } else {
        format!(" {right}")
    };
    let right_width = char_count(&right);
    let left_width = inner_width.saturating_sub(right_width);
    let left_text = truncate_chars(left, left_width);
    let padding = left_width.saturating_sub(char_count(&left_text));
    format!("│ {left_text}{}{right} │\n", " ".repeat(padding))
}

fn workstation_border(left: &str, separator: &str, right: &str, layout: RenderLayout) -> String {
    let segments = layout
        .columns
        .iter()
        .map(|column| "─".repeat(column.width + 2))
        .collect::<Vec<_>>()
        .join(separator);
    format!("{left}{segments}{right}\n")
}

fn workstation_header_row(layout: RenderLayout) -> String {
    let cells = layout
        .columns
        .iter()
        .map(|column| column.label.to_owned())
        .collect::<Vec<_>>();
    workstation_row(&cells, true, layout)
}

fn workstation_data_row(
    row: &FeatureSnapshot,
    selected: bool,
    interactive: bool,
    layout: RenderLayout,
) -> String {
    let cells = layout
        .columns
        .iter()
        .map(|column| format_column_cell(row, column.kind, selected, interactive))
        .collect::<Vec<_>>();
    workstation_row(&cells, false, layout)
}

fn format_column_cell(
    row: &FeatureSnapshot,
    kind: ColumnKind,
    selected: bool,
    interactive: bool,
) -> String {
    match kind {
        ColumnKind::Symbol => {
            if interactive {
                format!(
                    "{} {}",
                    if selected { "▶" } else { " " },
                    display_symbol(row)
                )
            } else {
                display_symbol(row).to_owned()
            }
        }
        ColumnKind::Price => format_optional(row.price, 4),
        ColumnKind::Spread => format_bps_value(row.spread_bps),
        ColumnKind::Imbalance => format_imbalance_cell(row.tob_imbalance),
        ColumnKind::Flow30 => format_signed_usd(row.signed_notional_flow_30s),
        ColumnKind::Rv5m => format_volatility_compact(row.rv_5m),
        ColumnKind::LiquidityCost => format_liquidity_cost_proxy(row),
        ColumnKind::Confidence => format_confidence_decimal(row),
        ColumnKind::WhyNow => format_why_now(row),
    }
}

fn selected_index(
    rows: &[FeatureSnapshot],
    ui_state: Option<&WorkstationUiState>,
) -> Option<usize> {
    ui_state
        .and_then(|state| state.selected_index(rows.len()))
        .or_else(|| (!rows.is_empty()).then_some(0))
}

fn visible_range(
    row_count: usize,
    selected_index: usize,
    ui_state: Option<&WorkstationUiState>,
) -> std::ops::Range<usize> {
    let Some(ui_state) = ui_state else {
        return 0..row_count;
    };
    let limit = ui_state.visible_row_limit().max(1).min(row_count);
    let half = limit / 2;
    let start = selected_index
        .saturating_sub(half)
        .min(row_count.saturating_sub(limit));
    start..start + limit
}

fn focus_label(selected_index: Option<usize>, row_count: usize) -> String {
    selected_index.map_or_else(
        || "-/-".to_owned(),
        |index| format!("{}/{}", index + 1, row_count),
    )
}

fn density_short_label(density: WorkstationDensity) -> &'static str {
    match density {
        WorkstationDensity::Compact => "cmp",
        WorkstationDensity::Balanced => "bal",
        WorkstationDensity::Dense => "dns",
    }
}

fn render_selected_detail(
    output: &mut String,
    selected: &FeatureSnapshot,
    view: WorkstationView,
    layout: RenderLayout,
) {
    if layout.bounded_detail {
        render_selected_detail_bounded(output, selected, view, layout);
        return;
    }

    output.push_str(&format!(
        "Selected: {}  | view {}\n",
        display_symbol(selected),
        view.label()
    ));

    match view {
        WorkstationView::Overview => {
            output.push_str(&format!(
                "Bid/Ask        {:<21} Micro-BBO      {:<12} Mark-Mid basis {}\n",
                format_bid_ask(selected),
                format_optional(selected.mid_px, 4),
                format_basis_bps(selected),
            ));
            output.push_str(&format!(
                "Top book       {:<21} OFI 30s        {:<12} Spread recovery {}\n",
                format_top_book(selected),
                format_signed_usd(selected.bbo_ofi_proxy_30s),
                format_recovery(selected.spread_recovery_ms),
            ));
            output.push_str(&format!(
                "Signed flow    5s:-  30s:{} 1m:-       RV 1m/5m/1h   {}\n",
                format_signed_usd(selected.signed_notional_flow_30s),
                format_volatility_compact_triplet(selected),
            ));
            output.push_str(&format!(
                "Confidence     {}\n",
                format_confidence_counters(selected)
            ));
            output.push_str(&format!(
                "Why ranked     {} | tradeability {} | resilience {}\n",
                format_why_ranked_tokens(selected),
                format_tradeability_state(selected.tradeability_state),
                format_resilience_state(selected.resilience_state),
            ));
            output.push_str(&format!(
                "Metadata       {}\n",
                format_metadata_summary(selected)
            ));
        }
        WorkstationView::Flow => {
            output.push_str(&format!(
                "Flow tape      signed30 {:<12} ofi30 {:<12} adverse proxy {}\n",
                format_signed_usd(selected.signed_notional_flow_30s),
                format_signed_usd(selected.bbo_ofi_proxy_30s),
                selected.adverse_selection_proxy.as_str(),
            ));
            output.push_str(&format!(
                "Volatility     rv1m/5m/1h {:<18} ret1m/5m/1h {}\n",
                format_volatility_compact_triplet(selected),
                format_return_triplet(selected),
            ));
            output.push_str(&format!(
                "Liquidity      spread {} bps | depth {} | imbalance {}\n",
                format_bps_value(selected.spread_bps),
                format_usd(selected.tob_depth_usd),
                format_imbalance_cell(selected.tob_imbalance),
            ));
        }
        WorkstationView::Quality => {
            output.push_str(&format!(
                "Confidence     level {} | score {} | {}\n",
                selected.confidence.level.as_str(),
                selected.confidence.score,
                format_confidence_counters(selected),
            ));
            output.push_str(&format!(
                "Freshness      row age {} | staleness {:?} | parser drops {}\n",
                format_age(selected.updated_ms_ago),
                selected.staleness_state,
                reason_count(selected, ConfidenceReason::ParserDrops),
            ));
            output.push_str(
                "Boundary       quality changes display trust only; it is not an execution gate.\n",
            );
        }
        WorkstationView::Metadata => {
            output.push_str(&format!(
                "Metadata       {}\n",
                format_metadata_summary(selected)
            ));
            output.push_str(&format!(
                "Identifiers    display {} | feed {}\n",
                display_symbol(selected),
                selected.symbol,
            ));
            output.push_str(
                "Boundary       metadata is public discovery context; missing fields stay explicit.\n",
            );
        }
        WorkstationView::Explain => {
            output.push_str(&format!(
                "Why ranked     {} | score {}\n",
                format_why_ranked_tokens(selected),
                format_score_pair(selected),
            ));
            if let Some(breakdown) = &selected.score_breakdown {
                output.push_str(&format!(
                    "Score totals   adjusted {} | raw {} | confidence penalty {}\n",
                    format_score(Some(breakdown.adjusted_total)),
                    format_score(Some(breakdown.raw_total)),
                    format_score(Some(breakdown.confidence_penalty())),
                ));
                if !breakdown.unavailable_evidence.is_empty() {
                    output.push_str(&format!(
                        "Unavailable   {}\n",
                        breakdown.unavailable_evidence.join(", ")
                    ));
                }
            }
            output.push_str(
                "Boundary       score components are screen heuristics, not advice or order intent.\n",
            );
        }
    }
}

fn render_selected_detail_bounded(
    output: &mut String,
    selected: &FeatureSnapshot,
    view: WorkstationView,
    layout: RenderLayout,
) {
    output.push_str(&workstation_full_line(
        &format!(
            "Selected: {} | view {}",
            display_symbol(selected),
            view.label()
        ),
        "",
        layout,
    ));

    match view {
        WorkstationView::Overview => {
            push_detail_line(
                output,
                "BBO",
                &format!(
                    "{} | mid {} | basis {}",
                    format_bid_ask(selected),
                    format_optional(selected.mid_px, 4),
                    format_basis_bps(selected)
                ),
                layout,
            );
            push_detail_line(
                output,
                "Depth",
                &format!(
                    "{} | OFI {} | recovery {}",
                    format_top_book(selected),
                    format_signed_usd(selected.bbo_ofi_proxy_30s),
                    format_recovery(selected.spread_recovery_ms)
                ),
                layout,
            );
            push_detail_line(
                output,
                "Why",
                &format!(
                    "{} | trade {} | resil {}",
                    format_why_ranked_tokens(selected),
                    format_tradeability_state(selected.tradeability_state),
                    format_resilience_state(selected.resilience_state)
                ),
                layout,
            );
        }
        WorkstationView::Flow => {
            push_detail_line(
                output,
                "Flow",
                &format!(
                    "signed30 {} | ofi30 {} | adverse {}",
                    format_signed_usd(selected.signed_notional_flow_30s),
                    format_signed_usd(selected.bbo_ofi_proxy_30s),
                    selected.adverse_selection_proxy.as_str()
                ),
                layout,
            );
            push_detail_line(
                output,
                "Vol",
                &format!(
                    "rv {} | ret {}",
                    format_volatility_compact_triplet(selected),
                    format_return_triplet(selected)
                ),
                layout,
            );
        }
        WorkstationView::Quality => {
            push_detail_line(
                output,
                "Quality",
                &format!(
                    "{} {} | {}",
                    selected.confidence.level.as_str(),
                    selected.confidence.score,
                    format_confidence_counters(selected)
                ),
                layout,
            );
            push_detail_line(
                output,
                "Fresh",
                &format!(
                    "age {} | staleness {:?} | parser drops {}",
                    format_age(selected.updated_ms_ago),
                    selected.staleness_state,
                    reason_count(selected, ConfidenceReason::ParserDrops)
                ),
                layout,
            );
        }
        WorkstationView::Metadata => {
            push_detail_line(output, "Meta", &format_metadata_summary(selected), layout);
            push_detail_line(
                output,
                "IDs",
                &format!(
                    "display {} | feed {}",
                    display_symbol(selected),
                    selected.symbol
                ),
                layout,
            );
        }
        WorkstationView::Explain => {
            push_detail_line(
                output,
                "Score",
                &format!(
                    "{} | pair {}",
                    format_why_ranked_tokens(selected),
                    format_score_pair(selected)
                ),
                layout,
            );
            if let Some(breakdown) = &selected.score_breakdown {
                push_detail_line(
                    output,
                    "Totals",
                    &format!(
                        "adj {} | raw {} | penalty {}",
                        format_score(Some(breakdown.adjusted_total)),
                        format_score(Some(breakdown.raw_total)),
                        format_score(Some(breakdown.confidence_penalty()))
                    ),
                    layout,
                );
            }
        }
    }
}

fn push_detail_line(output: &mut String, label: &str, body: &str, layout: RenderLayout) {
    output.push_str(&workstation_full_line(
        &format!("{label:<8} {body}"),
        "",
        layout,
    ));
}

fn push_boundary_caveat(output: &mut String, layout: RenderLayout) {
    if layout.bounded_detail {
        output.push_str(&workstation_full_line(
            "No wallet/private streams/order routes. Screen heuristic, not advice.",
            "",
            layout,
        ));
    } else {
        output.push_str(
            "\nNo wallet, no private streams, no order routes. Scores are screen heuristics, not orders or advice.\n",
        );
    }
}

fn display_symbol(row: &FeatureSnapshot) -> &str {
    row.metadata
        .as_ref()
        .map(|metadata| metadata.display_name.as_str())
        .filter(|display_name| !display_name.trim().is_empty())
        .unwrap_or(&row.symbol)
}

fn workstation_row(cells: &[String], header: bool, layout: RenderLayout) -> String {
    let mut output = String::from("│");
    for (column, cell) in layout.columns.iter().zip(cells.iter()) {
        let align = if header { Align::Left } else { column.align };
        output.push(' ');
        output.push_str(&pad_cell(cell, column.width, align));
        output.push(' ');
        output.push('│');
    }
    output.push('\n');
    output
}

fn pad_cell(value: &str, width: usize, align: Align) -> String {
    let value = truncate_chars(value, width);
    let padding = width.saturating_sub(char_count(&value));
    match align {
        Align::Left => format!("{value}{}", " ".repeat(padding)),
        Align::Right => format!("{}{value}", " ".repeat(padding)),
    }
}

fn filter_label(title: &str, request: Option<&ScreenRequest>) -> String {
    let Some(request) = request else {
        return title.to_owned();
    };
    match (&request.preset, &request.where_expr) {
        (Some(preset), Some(where_expr)) => format!("{where_expr}; preset {preset}"),
        (Some(preset), None) => preset.clone(),
        (None, Some(where_expr)) => where_expr.clone(),
        (None, None) => title.to_owned(),
    }
}

fn mode_label(row_count: usize, request: Option<&ScreenRequest>) -> String {
    let sort = request.and_then(|request| {
        request.sort.clone().or_else(|| {
            request
                .preset
                .as_deref()
                .and_then(find_preset)
                .map(|preset| preset.sort.to_owned())
        })
    });
    sort.map_or_else(
        || format!("top-{row_count} by screen rank"),
        |sort| format!("top-{row_count} by {}", sort.replace(':', " ")),
    )
}

fn format_bid_ask(row: &FeatureSnapshot) -> String {
    format!(
        "{} / {}",
        format_optional(row.bid_px, 4),
        format_optional(row.ask_px, 4)
    )
}

fn format_basis_bps(row: &FeatureSnapshot) -> String {
    match (row.mark_px, row.mid_px) {
        (Some(mark), Some(mid)) if mid != 0.0 => {
            format!("{:+.1} bps", ((mark / mid) - 1.0) * 10_000.0)
        }
        _ => "-".to_owned(),
    }
}

fn format_top_book(row: &FeatureSnapshot) -> String {
    format!(
        "{} / {}",
        format_usd(notional(row.bid_px, row.bid_sz)),
        format_usd(notional(row.ask_px, row.ask_sz))
    )
}

fn notional(px: Option<f64>, qty: Option<f64>) -> Option<f64> {
    match (px, qty) {
        (Some(px), Some(qty)) => Some(px * qty),
        _ => None,
    }
}

fn format_confidence_counters(row: &FeatureSnapshot) -> String {
    format!(
        "window:{} stale:{} sparse:{} reconnect:{} parser_drop:{}",
        row.confidence.incomplete_windows.len(),
        reason_count(row, ConfidenceReason::StaleQuote),
        reason_count(row, ConfidenceReason::SparseTrades),
        reason_count(row, ConfidenceReason::ReconnectGap),
        reason_count(row, ConfidenceReason::ParserDrops),
    )
}

fn reason_count(row: &FeatureSnapshot, reason: ConfidenceReason) -> usize {
    row.confidence
        .reasons
        .iter()
        .filter(|candidate| **candidate == reason)
        .count()
}

fn format_why_ranked_tokens(row: &FeatureSnapshot) -> String {
    let Some(breakdown) = &row.score_breakdown else {
        return format!("score {}", format_score_pair(row));
    };

    let mut tokens = breakdown
        .components
        .iter()
        .filter_map(|component| {
            if component.signed_contribution > 0.5 {
                Some(format!("+{}", component.name))
            } else if component.signed_contribution < -0.5 {
                Some(format!("-{}", component.name))
            } else {
                None
            }
        })
        .take(4)
        .collect::<Vec<_>>();

    if tokens.is_empty() {
        tokens.push(format!(
            "score {}",
            format_score(Some(breakdown.adjusted_total))
        ));
    }
    if !breakdown.unavailable_evidence.is_empty() {
        tokens.push(format!(
            "missing:{}",
            breakdown.unavailable_evidence.join(",")
        ));
    }

    tokens.join(" ")
}

fn format_metadata_summary(row: &FeatureSnapshot) -> String {
    format!(
        "metadata | {} | listing age {} | seeded {} | source {}",
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
    )
}

fn char_count(value: &str) -> usize {
    value.chars().count()
}

struct TableStats {
    row_count: usize,
    stale: usize,
    incomplete: usize,
    spread_count: usize,
    median_spread_bps: Option<f64>,
    depth_count: usize,
    top_tob_depth_usd: Option<f64>,
    p95_age_ms: Option<i64>,
    confidence_high: usize,
    confidence_medium: usize,
    confidence_low: usize,
    confidence_untrusted: usize,
    min_confidence_score: Option<u8>,
    confidence_reason_count: usize,
}

impl TableStats {
    fn from_rows(rows: &[FeatureSnapshot]) -> Self {
        let spreads = finite_values(rows.iter().filter_map(|row| row.spread_bps));
        let depths = finite_values(rows.iter().filter_map(|row| row.tob_depth_usd));
        let ages = rows
            .iter()
            .filter_map(|row| row.updated_ms_ago)
            .collect::<Vec<_>>();

        Self {
            row_count: rows.len(),
            stale: rows
                .iter()
                .filter(|row| row.staleness_state == StalenessState::Stale)
                .count(),
            incomplete: rows
                .iter()
                .filter(|row| row.staleness_state == StalenessState::Incomplete)
                .count(),
            spread_count: spreads.len(),
            median_spread_bps: median(spreads),
            depth_count: depths.len(),
            top_tob_depth_usd: max_value(depths.iter().copied()),
            p95_age_ms: percentile_i64(ages.iter().copied(), 0.95),
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
        }
    }

    fn quality_status(&self) -> &'static str {
        if self.incomplete > 0 {
            return "CHECK";
        }
        if self.row_count == 0
            || self.median_spread_bps.is_none()
            || self.top_tob_depth_usd.is_none()
            || self.spread_count < self.row_count
            || self.depth_count < self.row_count
        {
            return "PARTIAL";
        }

        let check_quality = self.median_spread_bps.is_some_and(|spread| spread >= 100.0)
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
}

fn format_optional(value: Option<f64>, decimals: usize) -> String {
    value.map_or_else(|| "-".to_owned(), |value| format!("{value:.decimals$}"))
}

fn format_bps_value(value: Option<f64>) -> String {
    value.map_or_else(|| "-".to_owned(), |value| format!("{value:.1}"))
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

fn format_imbalance_cell(value: Option<f64>) -> String {
    value.map_or_else(|| "-".to_owned(), |value| format!("{value:+.2}"))
}

fn format_volatility_compact(value: Option<f64>) -> String {
    value.map_or_else(|| "-".to_owned(), |value| format!("{:.2}", value * 100.0))
}

fn format_volatility_compact_triplet(row: &FeatureSnapshot) -> String {
    format!(
        "{}/{}/{}",
        format_volatility_compact(row.rv_1m),
        format_volatility_compact(row.rv_5m),
        format_volatility_compact(row.rv_1h),
    )
}

fn format_return_triplet(row: &FeatureSnapshot) -> String {
    format!(
        "{}/{}/{}",
        format_percent(row.ret_1m),
        format_percent(row.ret_5m),
        format_percent(row.ret_1h),
    )
}

fn format_percent(value: Option<f64>) -> String {
    value.map_or_else(|| "-".to_owned(), |value| format!("{:+.2}%", value * 100.0))
}

fn format_liquidity_cost_proxy(row: &FeatureSnapshot) -> String {
    match (row.spread_bps, row.tob_depth_usd) {
        (Some(spread), Some(depth))
            if row.liquidity_score >= 70.0 && spread <= 20.0 && depth >= 10_000.0 =>
        {
            "low".to_owned()
        }
        (Some(spread), Some(depth))
            if row.liquidity_score >= 20.0 && spread <= 75.0 && depth >= 1_000.0 =>
        {
            "med".to_owned()
        }
        (Some(_), Some(_)) => "high".to_owned(),
        _ => "unknown".to_owned(),
    }
}

fn format_confidence_decimal(row: &FeatureSnapshot) -> String {
    format!("{:.2}", f64::from(row.confidence.score) / 100.0)
}

fn format_score(value: Option<f64>) -> String {
    value.map_or_else(|| "-".to_owned(), |value| format!("{value:.1}"))
}

fn format_score_pair(row: &FeatureSnapshot) -> String {
    row.score_breakdown.as_ref().map_or_else(
        || format!("{:.1}/{:.1}", row.liquidity_score, row.momentum_score),
        |breakdown| format!("{:.1}/{:.1}", breakdown.adjusted_total, breakdown.raw_total),
    )
}

fn format_metadata_tags(row: &FeatureSnapshot) -> String {
    match &row.metadata {
        Some(metadata) => format!("tags {}", metadata.cohort_label()),
        None => "tags unknown_metadata".to_owned(),
    }
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

fn format_tradeability_state(state: TradeabilityState) -> &'static str {
    state.as_str()
}

fn format_resilience_state(state: LiquidityResilienceState) -> &'static str {
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

fn format_why_now(row: &FeatureSnapshot) -> String {
    let parts = observation_parts(row);
    if parts.is_empty() {
        "steady".to_owned()
    } else {
        parts
            .into_iter()
            .map(|part| match part.as_str() {
                "thin book" => "thin".to_owned(),
                "wide spread" => "wide".to_owned(),
                "tight spread" => "tight".to_owned(),
                "move active" => "move".to_owned(),
                "imbalanced" => "imbalance".to_owned(),
                "spread shock" => "shock".to_owned(),
                "recovering book" => "recovering".to_owned(),
                "brittle book" => "brittle".to_owned(),
                "fresh liquidity" => "fresh liq".to_owned(),
                "new listing" => "new".to_owned(),
                other => other.to_owned(),
            })
            .take(2)
            .collect::<Vec<_>>()
            .join(" + ")
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

fn percentile_i64(values: impl Iterator<Item = i64>, percentile: f64) -> Option<i64> {
    let mut values: Vec<_> = values.collect();
    if values.is_empty() {
        return None;
    }
    values.sort_unstable();
    let percentile = percentile.clamp(0.0, 1.0);
    let index = ((values.len() - 1) as f64 * percentile).ceil() as usize;
    values.get(index).copied()
}

fn max_value(values: impl Iterator<Item = f64>) -> Option<f64> {
    values
        .filter(|value| value.is_finite())
        .max_by(f64::total_cmp)
}
