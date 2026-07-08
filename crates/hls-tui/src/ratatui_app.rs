use hls_core::market_state::{FeatureSnapshot, StalenessState};
use hls_screen::{ScreenEngine, ScreenRequest, presets::find_preset};
use ratatui::{
    Frame, Terminal,
    backend::TestBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Sparkline, Table, Wrap},
};

use crate::interaction::{WorkstationUiState, WorkstationView};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RatatuiColorMode {
    Auto,
    Color,
    NoColor,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RatatuiViewport {
    pub width: u16,
    pub height: u16,
}

#[derive(Clone, Debug)]
pub struct RatatuiFrameModel {
    rows: Vec<FeatureSnapshot>,
    title: String,
    request: ScreenRequest,
    ui_state: WorkstationUiState,
    stream_status: String,
    recorder_status: String,
    health_status: String,
}

impl RatatuiFrameModel {
    pub fn new(
        rows: Vec<FeatureSnapshot>,
        title: impl Into<String>,
        request: ScreenRequest,
        ui_state: WorkstationUiState,
    ) -> Self {
        Self {
            rows,
            title: title.into(),
            request,
            ui_state,
            stream_status: "LIVE".to_owned(),
            recorder_status: "REC ready".to_owned(),
            health_status: "ws=0 events=0 gaps=0".to_owned(),
        }
    }

    pub fn with_status(
        mut self,
        stream_status: impl Into<String>,
        recorder_status: impl Into<String>,
        health_status: impl Into<String>,
    ) -> Self {
        self.stream_status = stream_status.into();
        self.recorder_status = recorder_status.into();
        self.health_status = health_status.into();
        self
    }
}

pub fn render_ratatui_snapshot_for_test(
    model: &RatatuiFrameModel,
    viewport: RatatuiViewport,
    color_mode: RatatuiColorMode,
) -> hls_core::HlsResult<String> {
    let backend = TestBackend::new(viewport.width, viewport.height);
    let mut terminal = Terminal::new(backend)
        .map_err(|err| hls_core::HlsError::External(format!("create test terminal: {err}")))?;
    terminal
        .draw(|frame| render_ratatui_frame(frame, model, color_mode))
        .map_err(|err| hls_core::HlsError::External(format!("draw test terminal: {err}")))?;

    let buffer = terminal.backend().buffer();
    let area = buffer.area;
    let mut rendered = String::new();
    if color_mode == RatatuiColorMode::Color {
        rendered.push_str("\x1b[36m");
    }
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            rendered.push_str(buffer[(x, y)].symbol());
        }
        trim_trailing_spaces(&mut rendered);
        rendered.push('\n');
    }
    if color_mode == RatatuiColorMode::Color {
        rendered.push_str("\x1b[0m");
    }
    Ok(rendered)
}

pub fn render_ratatui_frame(
    frame: &mut Frame<'_>,
    model: &RatatuiFrameModel,
    color_mode: RatatuiColorMode,
) {
    let area = frame.area();
    if area.width < 90 {
        render_narrow(frame, area, model, color_mode);
    } else if area.width < 132 {
        render_medium(frame, area, model, color_mode);
    } else {
        render_wide(frame, area, model, color_mode);
    }
}

fn render_wide(
    frame: &mut Frame<'_>,
    area: Rect,
    model: &RatatuiFrameModel,
    color_mode: RatatuiColorMode,
) {
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(12),
            Constraint::Length(2),
        ])
        .split(area);
    render_header(frame, root[0], model, color_mode);
    render_status_bar(frame, root[2], model, color_mode);

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Percentage(48),
            Constraint::Percentage(22),
        ])
        .split(root[1]);
    render_watchlist(frame, body[0], model, color_mode);

    let center = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(9), Constraint::Min(12)])
        .split(body[1]);
    render_detail(frame, center[0], model, "MICROSTRUCTURE", color_mode);
    render_chart(frame, center[1], model, color_mode);

    let right = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(42), Constraint::Percentage(58)])
        .split(body[2]);
    render_book(frame, right[0], model, color_mode);
    render_tape(frame, right[1], model, color_mode);
    render_help_overlay(frame, area, model, color_mode);
}

fn render_medium(
    frame: &mut Frame<'_>,
    area: Rect,
    model: &RatatuiFrameModel,
    color_mode: RatatuiColorMode,
) {
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(12),
            Constraint::Length(2),
        ])
        .split(area);
    render_header(frame, root[0], model, color_mode);
    render_status_bar(frame, root[2], model, color_mode);

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(38), Constraint::Percentage(62)])
        .split(root[1]);
    render_watchlist(frame, body[0], model, color_mode);

    let center = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(10), Constraint::Min(10)])
        .split(body[1]);
    render_detail(frame, center[0], model, "MICROSTRUCTURE", color_mode);
    render_chart(frame, center[1], model, color_mode);
    render_help_overlay(frame, area, model, color_mode);
}

fn render_narrow(
    frame: &mut Frame<'_>,
    area: Rect,
    model: &RatatuiFrameModel,
    color_mode: RatatuiColorMode,
) {
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Percentage(48),
            Constraint::Min(8),
            Constraint::Length(2),
        ])
        .split(area);
    render_header(frame, root[0], model, color_mode);
    render_watchlist(frame, root[1], model, color_mode);
    render_detail(frame, root[2], model, "DETAIL", color_mode);
    render_status_bar(frame, root[3], model, color_mode);
    render_help_overlay(frame, area, model, color_mode);
}

fn render_header(
    frame: &mut Frame<'_>,
    area: Rect,
    model: &RatatuiFrameModel,
    color_mode: RatatuiColorMode,
) {
    let filter = filter_label(&model.title, &model.request);
    let text = vec![Line::from(vec![
        Span::styled(
            "HLSCREEN ",
            Style::default()
                .fg(accent(color_mode))
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!("{}  ", model.stream_status)),
        Span::styled(
            model.recorder_status.clone(),
            Style::default().fg(success(color_mode)),
        ),
        Span::raw(format!(
            "  filter: {filter}  view:{} density:{}  keys: j/k tab / p s t ? space q",
            model.ui_state.view().label(),
            model.ui_state.density().label(),
        )),
    ])];
    frame.render_widget(
        Paragraph::new(text).block(
            Block::default()
                .title(" Hyperliquid Spot Microstructure Workstation ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(accent(color_mode))),
        ),
        area,
    );
}

fn render_watchlist(
    frame: &mut Frame<'_>,
    area: Rect,
    model: &RatatuiFrameModel,
    color_mode: RatatuiColorMode,
) {
    let rows = screened_rows(model);
    let selected = model
        .ui_state
        .selected_index(rows.len())
        .unwrap_or_default();
    let table_rows = rows
        .iter()
        .take(model.ui_state.visible_row_limit())
        .enumerate()
        .map(|(index, row)| {
            let style = if index == selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(success(color_mode))
                    .add_modifier(Modifier::BOLD)
            } else if row.staleness_state != StalenessState::Fresh {
                Style::default().fg(warn(color_mode))
            } else {
                Style::default().fg(text(color_mode))
            };
            Row::new(vec![
                Cell::from(display_symbol(row).to_owned()),
                Cell::from(format_price(row.price)),
                Cell::from(format_signed(row.ret_1m.map(|value| value * 100.0), "%")),
                Cell::from(format_conf(row.confidence.score)),
            ])
            .style(style)
        });

    let table = Table::new(
        table_rows,
        [
            Constraint::Min(10),
            Constraint::Length(10),
            Constraint::Length(8),
            Constraint::Length(5),
        ],
    )
    .header(
        Row::new(["CODE", "PRICE", "1M", "CONF"]).style(
            Style::default()
                .fg(accent(color_mode))
                .add_modifier(Modifier::BOLD),
        ),
    )
    .block(
        Block::default()
            .title(" WATCHLIST ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(accent(color_mode))),
    );
    frame.render_widget(table, area);
}

fn render_detail(
    frame: &mut Frame<'_>,
    area: Rect,
    model: &RatatuiFrameModel,
    title: &'static str,
    color_mode: RatatuiColorMode,
) {
    let rows = screened_rows(model);
    let Some(row) = selected_row(&rows, model) else {
        frame.render_widget(
            Paragraph::new("No market rows yet. Waiting for public frames.")
                .block(panel(title, color_mode)),
            area,
        );
        return;
    };

    let lines = detail_lines(row, model.ui_state.view(), color_mode);
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: true })
            .block(panel(title, color_mode)),
        area,
    );
}

fn detail_lines(
    row: &FeatureSnapshot,
    view: WorkstationView,
    color_mode: RatatuiColorMode,
) -> Vec<Line<'static>> {
    let heading = Line::from(vec![
        Span::styled(
            display_symbol(row).to_owned(),
            Style::default()
                .fg(accent(color_mode))
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!(
            "  px {}  spread {} bps",
            format_price(row.price),
            format_optional(row.spread_bps, 1)
        )),
    ]);

    match view {
        WorkstationView::Overview => vec![
            heading,
            Line::from(format!(
                "confidence {} {} | tradeability {} | resilience {}",
                row.confidence.level.as_str(),
                row.confidence.score,
                row.tradeability_state.as_str(),
                row.resilience_state.as_str()
            )),
            Line::from(format!(
                "flow30 {} | bbo ofi {} | depth {} | imbalance {}",
                format_usd_signed(row.signed_notional_flow_30s),
                format_usd_signed(row.bbo_ofi_proxy_30s),
                format_usd(row.tob_depth_usd),
                format_signed(row.tob_imbalance, "")
            )),
            Line::from(format!(
                "metadata {} | listing {} | source {}",
                metadata_label(row),
                listing_age(row),
                metadata_source(row)
            )),
            Line::from(format!("why-ranked {}", why_tokens(row))),
        ],
        WorkstationView::Flow => vec![
            heading,
            Line::from("Flow tape"),
            Line::from(format!(
                "signed flow 5s - | 30s {} | 1m -",
                format_usd_signed(row.signed_notional_flow_30s),
            )),
            Line::from(format!(
                "bbo ofi 30s {} | adverse proxy {} | spread recovery {}",
                format_usd_signed(row.bbo_ofi_proxy_30s),
                row.adverse_selection_proxy.as_str(),
                format_duration_ms(row.spread_recovery_ms)
            )),
        ],
        WorkstationView::Quality => vec![
            heading,
            Line::from("Quality"),
            Line::from(format!(
                "row age {} | staleness {:?} | confidence {} {}",
                format_duration_ms(row.updated_ms_ago),
                row.staleness_state,
                row.confidence.level.as_str(),
                row.confidence.score
            )),
            Line::from(format!(
                "reasons {} | incomplete windows {} | rv 1m/5m/1h {}/{}/{}",
                row.confidence.reasons.len(),
                row.confidence.incomplete_windows.len(),
                format_optional(row.rv_1m, 2),
                format_optional(row.rv_5m, 2),
                format_optional(row.rv_1h, 2)
            )),
        ],
        WorkstationView::Metadata => vec![
            heading,
            Line::from("Metadata"),
            Line::from(format!(
                "tags {} | cohort {} | listing {}",
                metadata_tags(row),
                metadata_label(row),
                listing_age(row)
            )),
            Line::from(format!(
                "seeded {} | source {} | id {}",
                row.metadata
                    .as_ref()
                    .and_then(|metadata| metadata.seeded_usdc)
                    .map(|value| format_usd(Some(value)))
                    .unwrap_or_else(|| "-".to_owned()),
                metadata_source(row),
                row.symbol
            )),
        ],
        WorkstationView::Explain => vec![
            heading,
            Line::from("Explain"),
            Line::from(format!("why-ranked {}", why_tokens(row))),
            Line::from(format!(
                "tradeability {} | resilience {} | confidence {}",
                row.tradeability_state.as_str(),
                row.resilience_state.as_str(),
                row.confidence.level.as_str()
            )),
            Line::from("Screen output is heuristic context only, not orders or advice."),
        ],
    }
}

fn metadata_label(row: &FeatureSnapshot) -> String {
    row.metadata
        .as_ref()
        .map(|metadata| metadata.cohort_label())
        .unwrap_or_else(|| "unknown_metadata".to_owned())
}

fn metadata_tags(row: &FeatureSnapshot) -> String {
    row.metadata
        .as_ref()
        .map(|metadata| metadata.cohort_tags.join(","))
        .filter(|tags| !tags.is_empty())
        .unwrap_or_else(|| "unknown_metadata".to_owned())
}

fn listing_age(row: &FeatureSnapshot) -> String {
    row.metadata
        .as_ref()
        .and_then(|metadata| metadata.listing_age_ms)
        .map(format_age_ms)
        .unwrap_or_else(|| "-".to_owned())
}

fn metadata_source(row: &FeatureSnapshot) -> &str {
    row.metadata
        .as_ref()
        .map(|metadata| metadata.metadata_source.as_str())
        .unwrap_or("missing")
}

fn format_duration_ms(value: Option<i64>) -> String {
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

fn render_help_overlay(
    frame: &mut Frame<'_>,
    area: Rect,
    model: &RatatuiFrameModel,
    color_mode: RatatuiColorMode,
) {
    if !model.ui_state.help_open() {
        return;
    }
    let popup = centered_rect(70, 42, area);
    frame.render_widget(Clear, popup);
    let lines = vec![
        Line::from(vec![Span::styled(
            "Command Deck",
            Style::default()
                .fg(accent(color_mode))
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from("j/k or arrows  move selected market"),
        Line::from("tab / shift-tab  cycle overview, flow, quality, metadata, explain"),
        Line::from("d  density  |  space  pause display  |  ?  help  |  q  quit"),
        Line::from("/ p s t are reserved for the next command/filter slice"),
        Line::from("Display only: no wallet, private streams, or order routes."),
    ];
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: true })
            .block(panel("HELP", color_mode))
            .style(Style::default().fg(text(color_mode))),
        popup,
    );
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1]);
    horizontal[1]
}

fn render_chart(
    frame: &mut Frame<'_>,
    area: Rect,
    model: &RatatuiFrameModel,
    color_mode: RatatuiColorMode,
) {
    let rows = screened_rows(model);
    let Some(row) = selected_row(&rows, model) else {
        frame.render_widget(
            Paragraph::new("No chart data").block(panel("CHART", color_mode)),
            area,
        );
        return;
    };
    let mut data = vec![1, 2, 3, 4, 5, 6, 7, 8];
    if let Some(ret) = row.ret_1m {
        let base = (ret.abs() * 1_000.0).round() as u64;
        data = (0..32).map(|index| 1 + ((base + index * 3) % 18)).collect();
    }
    let sparkline = Sparkline::default()
        .block(panel("CHART  1m | 5m | 15m | 30m | 60m", color_mode))
        .data(&data)
        .style(Style::default().fg(success(color_mode)));
    frame.render_widget(sparkline, area);
}

fn render_book(
    frame: &mut Frame<'_>,
    area: Rect,
    model: &RatatuiFrameModel,
    color_mode: RatatuiColorMode,
) {
    let rows = screened_rows(model);
    let body = selected_row(&rows, model).map_or_else(
        || "No book data".to_owned(),
        |row| {
            format!(
                "Bid depth {}\nAsk depth {}\nSpread {} bps\nBOOK proxy only",
                format_usd(notional(row.bid_px, row.bid_sz)),
                format_usd(notional(row.ask_px, row.ask_sz)),
                format_optional(row.spread_bps, 1)
            )
        },
    );
    frame.render_widget(Paragraph::new(body).block(panel("BOOK", color_mode)), area);
}

fn render_tape(
    frame: &mut Frame<'_>,
    area: Rect,
    model: &RatatuiFrameModel,
    color_mode: RatatuiColorMode,
) {
    let rows = screened_rows(model);
    let lines = rows
        .iter()
        .take(model.ui_state.visible_row_limit().min(12))
        .map(|row| {
            Line::from(format!(
                "{}  {}  {}",
                display_symbol(row),
                format_price(row.price),
                format_usd_signed(row.signed_notional_flow_30s)
            ))
        })
        .collect::<Vec<_>>();
    frame.render_widget(Paragraph::new(lines).block(panel("TAPE", color_mode)), area);
}

fn render_status_bar(
    frame: &mut Frame<'_>,
    area: Rect,
    model: &RatatuiFrameModel,
    color_mode: RatatuiColorMode,
) {
    let status = format!(
        " {} | {} | {} | No wallet, no private streams, no order routes. Screen heuristic, not advice. ",
        model.health_status,
        pause_label(model),
        mode_label(&model.request, model.rows.len())
    );
    frame.render_widget(
        Paragraph::new(status)
            .style(Style::default().fg(warn(color_mode)))
            .block(
                Block::default()
                    .borders(Borders::TOP)
                    .border_style(Style::default().fg(accent(color_mode))),
            ),
        area,
    );
}

fn pause_label(model: &RatatuiFrameModel) -> &'static str {
    if model.ui_state.paused() {
        "display paused"
    } else {
        "display live"
    }
}

fn panel(title: &str, color_mode: RatatuiColorMode) -> Block<'static> {
    Block::default()
        .title(format!(" {title} "))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(accent(color_mode)))
}

fn screened_rows(model: &RatatuiFrameModel) -> Vec<FeatureSnapshot> {
    ScreenEngine
        .apply(&model.rows, &model.request)
        .unwrap_or_else(|_| model.rows.clone())
}

fn selected_row<'a>(
    rows: &'a [FeatureSnapshot],
    model: &RatatuiFrameModel,
) -> Option<&'a FeatureSnapshot> {
    model
        .ui_state
        .selected_index(rows.len())
        .and_then(|index| rows.get(index))
        .or_else(|| rows.first())
}

fn display_symbol(row: &FeatureSnapshot) -> &str {
    row.metadata
        .as_ref()
        .map(|metadata| metadata.display_name.as_str())
        .filter(|display_name| !display_name.trim().is_empty())
        .unwrap_or(&row.symbol)
}

fn filter_label(title: &str, request: &ScreenRequest) -> String {
    match (&request.preset, &request.where_expr) {
        (Some(preset), Some(where_expr)) => format!("{where_expr}; preset {preset}"),
        (Some(preset), None) => preset.clone(),
        (None, Some(where_expr)) => where_expr.clone(),
        (None, None) => title.to_owned(),
    }
}

fn mode_label(request: &ScreenRequest, row_count: usize) -> String {
    let sort = request.sort.clone().or_else(|| {
        request
            .preset
            .as_deref()
            .and_then(find_preset)
            .map(|preset| preset.sort.to_owned())
    });
    sort.map_or_else(
        || format!("top-{row_count} by screen rank"),
        |sort| format!("top-{row_count} by {}", sort.replace(':', " ")),
    )
}

fn format_price(value: Option<f64>) -> String {
    value.map_or_else(|| "-".to_owned(), |value| format!("{value:.4}"))
}

fn format_optional(value: Option<f64>, decimals: usize) -> String {
    value.map_or_else(|| "-".to_owned(), |value| format!("{value:.decimals$}"))
}

fn format_conf(value: u8) -> String {
    format!("{:.2}", f64::from(value) / 100.0)
}

fn format_signed(value: Option<f64>, suffix: &str) -> String {
    value.map_or_else(|| "-".to_owned(), |value| format!("{value:+.2}{suffix}"))
}

fn format_usd(value: Option<f64>) -> String {
    value.map_or_else(
        || "-".to_owned(),
        |value| {
            let abs = value.abs();
            if abs >= 1_000_000.0 {
                format!("${:.1}M", value / 1_000_000.0)
            } else if abs >= 1_000.0 {
                format!("${:.1}K", value / 1_000.0)
            } else {
                format!("${value:.0}")
            }
        },
    )
}

fn format_usd_signed(value: Option<f64>) -> String {
    value.map_or_else(
        || "-".to_owned(),
        |value| {
            let sign = if value >= 0.0 { "+" } else { "-" };
            format!("{sign}{}", format_usd(Some(value.abs())))
        },
    )
}

fn format_age_ms(value: i64) -> String {
    if value < 48 * 60 * 60 * 1_000 {
        format!("{:.1}h", value as f64 / (60.0 * 60.0 * 1_000.0))
    } else {
        format!("{:.1}d", value as f64 / (24.0 * 60.0 * 60.0 * 1_000.0))
    }
}

fn why_tokens(row: &FeatureSnapshot) -> String {
    row.score_breakdown.as_ref().map_or_else(
        || {
            format!(
                "liq {:.1} momentum {:.1}",
                row.liquidity_score, row.momentum_score
            )
        },
        |breakdown| {
            breakdown
                .components
                .iter()
                .filter(|component| component.signed_contribution.abs() >= 0.5)
                .take(4)
                .map(|component| {
                    if component.signed_contribution >= 0.0 {
                        format!("+{}", component.name)
                    } else {
                        format!("-{}", component.name)
                    }
                })
                .collect::<Vec<_>>()
                .join(" ")
        },
    )
}

fn notional(px: Option<f64>, qty: Option<f64>) -> Option<f64> {
    match (px, qty) {
        (Some(px), Some(qty)) => Some(px * qty),
        _ => None,
    }
}

fn trim_trailing_spaces(value: &mut String) {
    while value.ends_with(' ') {
        value.pop();
    }
}

fn accent(color_mode: RatatuiColorMode) -> Color {
    match color_mode {
        RatatuiColorMode::NoColor => Color::White,
        RatatuiColorMode::Auto | RatatuiColorMode::Color => Color::Cyan,
    }
}

fn success(color_mode: RatatuiColorMode) -> Color {
    match color_mode {
        RatatuiColorMode::NoColor => Color::White,
        RatatuiColorMode::Auto | RatatuiColorMode::Color => Color::Green,
    }
}

fn warn(color_mode: RatatuiColorMode) -> Color {
    match color_mode {
        RatatuiColorMode::NoColor => Color::White,
        RatatuiColorMode::Auto | RatatuiColorMode::Color => Color::Yellow,
    }
}

fn text(color_mode: RatatuiColorMode) -> Color {
    match color_mode {
        RatatuiColorMode::NoColor => Color::White,
        RatatuiColorMode::Auto | RatatuiColorMode::Color => Color::Gray,
    }
}
