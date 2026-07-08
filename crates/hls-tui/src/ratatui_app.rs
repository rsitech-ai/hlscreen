use hls_core::market_state::{CandleEvent, FeatureSnapshot, StalenessState, TradeabilityState};
use hls_screen::{ScreenEngine, ScreenRequest, presets::find_preset};
use ratatui::{
    Frame, Terminal,
    backend::TestBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, Wrap},
};

use crate::interaction::{
    WorkstationCommand, WorkstationPane, WorkstationUiState, WorkstationView,
};

const MAX_CHART_CANDLES: usize = 48;

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
    candles: Vec<CandleEvent>,
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
            candles: Vec::new(),
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

    pub fn with_candles(mut self, candles: Vec<CandleEvent>) -> Self {
        self.candles = candles;
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
            Constraint::Length(5),
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
    render_command_palette(frame, area, model, color_mode);
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
            Constraint::Length(5),
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
        .constraints([
            Constraint::Length(8),
            Constraint::Min(10),
            Constraint::Length(9),
        ])
        .split(body[1]);
    render_detail(frame, center[0], model, "MICROSTRUCTURE", color_mode);
    render_chart(frame, center[1], model, color_mode);
    let lower = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(center[2]);
    render_book(frame, lower[0], model, color_mode);
    render_tape(frame, lower[1], model, color_mode);
    render_help_overlay(frame, area, model, color_mode);
    render_command_palette(frame, area, model, color_mode);
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
            Constraint::Length(5),
            Constraint::Percentage(48),
            Constraint::Min(8),
            Constraint::Length(2),
        ])
        .split(area);
    render_header(frame, root[0], model, color_mode);
    render_watchlist(frame, root[1], model, color_mode);
    render_narrow_drilldown(frame, root[2], model, color_mode);
    render_status_bar(frame, root[3], model, color_mode);
    render_help_overlay(frame, area, model, color_mode);
    render_command_palette(frame, area, model, color_mode);
}

fn render_narrow_drilldown(
    frame: &mut Frame<'_>,
    area: Rect,
    model: &RatatuiFrameModel,
    color_mode: RatatuiColorMode,
) {
    match model.ui_state.focused_pane() {
        WorkstationPane::Chart => render_chart(frame, area, model, color_mode),
        WorkstationPane::Book => render_book(frame, area, model, color_mode),
        WorkstationPane::Tape => render_tape(frame, area, model, color_mode),
        WorkstationPane::Status => render_status_panel(frame, area, model, color_mode),
        WorkstationPane::Watchlist | WorkstationPane::Detail => {
            render_detail(frame, area, model, "DETAIL", color_mode);
        }
    }
}

fn render_header(
    frame: &mut Frame<'_>,
    area: Rect,
    model: &RatatuiFrameModel,
    color_mode: RatatuiColorMode,
) {
    let filter = filter_label(&model.title, &model.request);
    let text = vec![
        Line::from(vec![
            Span::styled(
                "STATUS ",
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
                "  {}  filter:{filter}",
                compact_ui_mode_label(&model.ui_state)
            )),
        ]),
        Line::from(vec![
            Span::styled(
                "CONTROLS ",
                Style::default()
                    .fg(accent(color_mode))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("j/k row  1-6 panes  tab views  / filter  p preset  s sort  t chart  ? q"),
        ]),
        market_internals_line(model, color_mode),
    ];
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

fn market_internals_line(model: &RatatuiFrameModel, color_mode: RatatuiColorMode) -> Line<'static> {
    let rows = screened_rows(model);
    let up = rows
        .iter()
        .filter(|row| row.ret_1m.is_some_and(|value| value > 0.0))
        .count();
    let down = rows
        .iter()
        .filter(|row| row.ret_1m.is_some_and(|value| value < 0.0))
        .count();
    let tradeable = rows
        .iter()
        .filter(|row| matches!(row.tradeability_state, TradeabilityState::Tradeable))
        .count();
    let stale = rows
        .iter()
        .filter(|row| row.staleness_state != StalenessState::Fresh)
        .count();
    let signed_flow = rows
        .iter()
        .filter_map(|row| row.signed_notional_flow_30s)
        .filter(|value| value.is_finite())
        .sum::<f64>();
    let depth = rows
        .iter()
        .filter_map(|row| row.tob_depth_usd)
        .filter(|value| value.is_finite())
        .sum::<f64>();
    Line::from(vec![
        Span::styled(
            "INTERNALS ",
            Style::default()
                .fg(accent(color_mode))
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!(
            "rows {:02}  up {:02} down {:02}  tradeable {:02} stale {:02}  flow {}  depth {}",
            rows.len().min(99),
            up.min(99),
            down.min(99),
            tradeable.min(99),
            stale.min(99),
            format_usd_signed(Some(signed_flow)),
            format_usd(Some(depth))
        )),
    ])
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
    let compact = area.width < 52;
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
                market_row_style(row, color_mode)
            };
            if compact {
                Row::new(vec![
                    Cell::from(format!("{:02}", index + 1)),
                    Cell::from(display_symbol(row).to_owned()),
                    Cell::from(format_board_price(row.price)),
                    Cell::from(trend_label(row.ret_1m)),
                    Cell::from(format_usd_signed(row.signed_notional_flow_30s)),
                    Cell::from(quality_badge(row)),
                ])
                .style(style)
            } else {
                Row::new(vec![
                    Cell::from(format!("{:02}", index + 1)),
                    Cell::from(display_symbol(row).to_owned()),
                    Cell::from(format_price(row.price)),
                    Cell::from(trend_label(row.ret_1m)),
                    Cell::from(format_usd_signed(row.signed_notional_flow_30s)),
                    Cell::from(format_usd(row.tob_depth_usd)),
                    Cell::from(quality_badge(row)),
                ])
                .style(style)
            }
        });

    let table = if compact {
        Table::new(
            table_rows,
            [
                Constraint::Length(3),
                Constraint::Min(8),
                Constraint::Length(7),
                Constraint::Length(8),
                Constraint::Length(7),
                Constraint::Length(1),
            ],
        )
        .header(
            Row::new(["RK", "CODE", "PX", "1M", "FLOW", "Q"]).style(
                Style::default()
                    .fg(accent(color_mode))
                    .add_modifier(Modifier::BOLD),
            ),
        )
    } else {
        Table::new(
            table_rows,
            [
                Constraint::Length(4),
                Constraint::Min(10),
                Constraint::Length(10),
                Constraint::Length(8),
                Constraint::Length(8),
                Constraint::Length(7),
                Constraint::Length(1),
            ],
        )
        .header(
            Row::new(["RANK", "CODE", "PRICE", "1M", "FLOW30", "DEPTH", "Q"]).style(
                Style::default()
                    .fg(accent(color_mode))
                    .add_modifier(Modifier::BOLD),
            ),
        )
    }
    .column_spacing(1)
    .block(panel_for(
        "WATCHLIST",
        WorkstationPane::Watchlist,
        model,
        color_mode,
    ));
    frame.render_widget(table, area);
}

fn market_row_style(row: &FeatureSnapshot, color_mode: RatatuiColorMode) -> Style {
    if row.ret_1m.unwrap_or(0.0) < 0.0 || row.signed_notional_flow_30s.unwrap_or(0.0) < 0.0 {
        Style::default().fg(danger(color_mode))
    } else if row.ret_1m.unwrap_or(0.0) > 0.0 || row.signed_notional_flow_30s.unwrap_or(0.0) > 0.0 {
        Style::default().fg(success(color_mode))
    } else {
        Style::default().fg(text(color_mode))
    }
}

fn trend_label(value: Option<f64>) -> String {
    value.map_or_else(
        || "-".to_owned(),
        |value| {
            let direction = if value > 0.0 {
                "UP"
            } else if value < 0.0 {
                "DN"
            } else {
                "FL"
            };
            format!("{direction}{:+.2}%", value * 100.0)
        },
    )
}

fn format_board_price(value: Option<f64>) -> String {
    value.map_or_else(
        || "-".to_owned(),
        |value| {
            let abs = value.abs();
            if abs >= 10_000.0 {
                format!("{value:.0}")
            } else if abs >= 1_000.0 {
                format!("{value:.1}")
            } else if abs >= 1.0 {
                format!("{value:.2}")
            } else {
                format!("{value:.4}")
            }
        },
    )
}

fn quality_badge(row: &FeatureSnapshot) -> &'static str {
    if row.confidence.score < 70 || row.staleness_state != StalenessState::Fresh {
        "!"
    } else if matches!(
        row.tradeability_state,
        hls_core::market_state::TradeabilityState::Tradeable
    ) {
        "T"
    } else {
        "Q"
    }
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
            Paragraph::new("No market rows yet. Waiting for public frames.").block(panel_for(
                title,
                WorkstationPane::Detail,
                model,
                color_mode,
            )),
            area,
        );
        return;
    };

    let lines = detail_lines(row, model.ui_state.view(), color_mode);
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: true })
            .block(panel_for(title, WorkstationPane::Detail, model, color_mode)),
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
        Line::from("j/k or arrows  act on focused pane: rows, detail view, or chart window"),
        Line::from("tab / shift-tab  cycle overview, flow, quality, metadata, explain"),
        Line::from("[ / ]  move pane focus: watchlist, detail, chart, book, tape, status"),
        Line::from("1-6 panes  watchlist, detail, chart, book, tape, status"),
        Line::from("mouse wheel moves rows; click focuses panes when terminal mouse is available"),
        Line::from("/ filter  |  p preset  |  s sort  |  t chart window"),
        Line::from("d  density  |  space  pause display  |  ?  help  |  q  quit"),
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

fn render_command_palette(
    frame: &mut Frame<'_>,
    area: Rect,
    model: &RatatuiFrameModel,
    color_mode: RatatuiColorMode,
) {
    let Some(command) = model.ui_state.command() else {
        return;
    };
    let popup = centered_rect(74, 24, area);
    frame.render_widget(Clear, popup);
    frame.render_widget(
        Paragraph::new(command_palette_lines(command, model))
            .wrap(Wrap { trim: true })
            .block(panel("COMMAND", color_mode))
            .style(Style::default().fg(text(color_mode))),
        popup,
    );
}

fn command_palette_lines(
    command: &WorkstationCommand,
    model: &RatatuiFrameModel,
) -> Vec<Line<'static>> {
    let input = if command.input().is_empty() {
        "<empty>"
    } else {
        command.input()
    };
    let mut lines = vec![
        Line::from(format!("{} > {input}", command.prompt())),
        Line::from(match command.target().label() {
            "filter" => "Enter apply filter | Esc cancel | empty clears custom filter",
            "preset" => "Enter apply preset | Esc cancel | empty clears preset",
            "sort" => "Enter apply sort | Esc cancel | empty clears custom sort",
            _ => "Enter apply | Esc cancel",
        }),
    ];
    if let Some(error) = model.ui_state.command_error() {
        lines.push(Line::from(format!("error: {error}")));
    }
    lines
}

fn compact_ui_mode_label(state: &WorkstationUiState) -> String {
    let command = state
        .command()
        .map(|command| format!(" cmd:{}", command.target().label()))
        .unwrap_or_default();
    format!(
        "view:{} pane:{} dens:{} chart:{}{}",
        state.view().label(),
        state.focused_pane().label(),
        state.density().label(),
        state.chart_window().label(),
        command
    )
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
            Paragraph::new("No chart data").block(panel_for(
                "CHART",
                WorkstationPane::Chart,
                model,
                color_mode,
            )),
            area,
        );
        return;
    };
    let candles = selected_candles(
        model,
        &row.symbol,
        area.width.saturating_sub(4) as usize,
        model.ui_state.chart_window().candle_limit(),
    );
    let Some(latest) = candles.last() else {
        frame.render_widget(
            Paragraph::new(vec![
                Line::from("Waiting for public 1m candle frames."),
                Line::from("No synthetic candles are rendered."),
            ])
            .wrap(Wrap { trim: true })
            .block(panel_for(
                "CHART  1m OHLC",
                WorkstationPane::Chart,
                model,
                color_mode,
            )),
            area,
        );
        return;
    };

    let title = format!(
        "CANDLES 1m/{}  O {} H {} L {} C {} VOL {}",
        model.ui_state.chart_window().label(),
        format_plain_number(latest.open),
        format_plain_number(latest.high),
        format_plain_number(latest.low),
        format_plain_number(latest.close),
        format_volume(latest.volume_base)
    );
    let chart_lines = candle_chart_lines(&candles, area.height.saturating_sub(4) as usize);
    frame.render_widget(
        Paragraph::new(chart_lines)
            .wrap(Wrap { trim: false })
            .block(panel_for(&title, WorkstationPane::Chart, model, color_mode))
            .style(Style::default().fg(success(color_mode))),
        area,
    );
}

fn selected_candles<'a>(
    model: &'a RatatuiFrameModel,
    symbol: &str,
    width_limit: usize,
    window_limit: usize,
) -> Vec<&'a CandleEvent> {
    let limit = width_limit.min(window_limit).clamp(1, MAX_CHART_CANDLES);
    let mut candles = model
        .candles
        .iter()
        .filter(|candle| candle.hl_coin == symbol && candle.interval == "1m")
        .collect::<Vec<_>>();
    candles.sort_by_key(|candle| candle.open_ts_ms);
    if candles.len() > limit {
        candles.drain(0..candles.len() - limit);
    }
    candles
}

fn candle_chart_lines(candles: &[&CandleEvent], chart_height: usize) -> Vec<Line<'static>> {
    if candles.is_empty() {
        return vec![Line::from("No 1m candles")];
    }
    let high = candles
        .iter()
        .map(|candle| candle.high)
        .fold(f64::NEG_INFINITY, f64::max);
    let low = candles
        .iter()
        .map(|candle| candle.low)
        .fold(f64::INFINITY, f64::min);
    let height = chart_height.clamp(4, 18);
    let mut lines = Vec::with_capacity(height + 2);

    for row in 0..height {
        let level = price_level(high, low, row, height);
        let mut body = String::with_capacity(candles.len());
        for candle in candles {
            body.push(candle_glyph(candle, level));
        }
        lines.push(Line::from(body));
    }

    lines.push(Line::from(format!(
        "range {} - {}   candles {}",
        format_plain_number(low),
        format_plain_number(high),
        candles.len()
    )));
    lines.push(Line::from(volume_bar(candles)));
    lines
}

fn price_level(high: f64, low: f64, row: usize, height: usize) -> f64 {
    if height <= 1 || (high - low).abs() < f64::EPSILON {
        return high;
    }
    high - ((high - low) * row as f64 / (height - 1) as f64)
}

fn candle_glyph(candle: &CandleEvent, level: f64) -> char {
    let body_high = candle.open.max(candle.close);
    let body_low = candle.open.min(candle.close);
    if level <= body_high && level >= body_low {
        if candle.close >= candle.open {
            '█'
        } else {
            '▓'
        }
    } else if level <= candle.high && level >= candle.low {
        '│'
    } else {
        ' '
    }
}

fn volume_bar(candles: &[&CandleEvent]) -> String {
    let max_volume = candles
        .iter()
        .map(|candle| candle.volume_base)
        .fold(0.0_f64, f64::max);
    if max_volume <= 0.0 {
        return "vol -".to_owned();
    }
    let bars = candles
        .iter()
        .map(|candle| volume_glyph(candle.volume_base / max_volume))
        .collect::<String>();
    format!("vol {bars}")
}

fn volume_glyph(ratio: f64) -> char {
    match (ratio * 8.0).ceil() as u8 {
        0 | 1 => '▁',
        2 => '▂',
        3 => '▃',
        4 => '▄',
        5 => '▅',
        6 => '▆',
        7 => '▇',
        _ => '█',
    }
}

fn render_book(
    frame: &mut Frame<'_>,
    area: Rect,
    model: &RatatuiFrameModel,
    color_mode: RatatuiColorMode,
) {
    let rows = screened_rows(model);
    let lines = selected_row(&rows, model).map_or_else(
        || vec![Line::from("No book data")],
        |row| book_lines(row, color_mode),
    );
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: true })
            .block(panel_for("BOOK", WorkstationPane::Book, model, color_mode)),
        area,
    );
}

fn book_lines(row: &FeatureSnapshot, color_mode: RatatuiColorMode) -> Vec<Line<'static>> {
    let bid_notional = notional(row.bid_px, row.bid_sz);
    let ask_notional = notional(row.ask_px, row.ask_sz);
    vec![
        Line::from(vec![
            Span::styled(
                "BID ",
                Style::default()
                    .fg(success(color_mode))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!(
                "{} x {}  notional {}",
                format_price(row.bid_px),
                format_size(row.bid_sz),
                format_usd(bid_notional)
            )),
        ]),
        Line::from(vec![
            Span::styled(
                "ASK ",
                Style::default()
                    .fg(danger(color_mode))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!(
                "{} x {}  notional {}",
                format_price(row.ask_px),
                format_size(row.ask_sz),
                format_usd(ask_notional)
            )),
        ]),
        Line::from(format!(
            "spread {} bps  depth {}",
            format_optional(row.spread_bps, 1),
            format_usd(row.tob_depth_usd)
        )),
        Line::from(format!(
            "imbalance {}  OFI {}",
            format_signed(row.tob_imbalance, ""),
            format_usd_signed(row.bbo_ofi_proxy_30s)
        )),
        Line::from(format!(
            "pressure {}",
            signed_meter(row.tob_imbalance.unwrap_or(0.0))
        )),
        Line::from(format!(
            "state {} / {}",
            row.tradeability_state.as_str(),
            row.resilience_state.as_str()
        )),
        Line::from(format!(
            "adverse {} | BOOK proxy only",
            row.adverse_selection_proxy.as_str()
        )),
    ]
}

fn render_tape(
    frame: &mut Frame<'_>,
    area: Rect,
    model: &RatatuiFrameModel,
    color_mode: RatatuiColorMode,
) {
    let rows = screened_rows(model);
    let lines = tape_lines(&rows, model);
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: true })
            .block(panel_for("TAPE", WorkstationPane::Tape, model, color_mode)),
        area,
    );
}

fn tape_lines(rows: &[FeatureSnapshot], model: &RatatuiFrameModel) -> Vec<Line<'static>> {
    let Some(selected) = selected_row(rows, model) else {
        return vec![Line::from("No flow data")];
    };

    let mut leaders = rows.iter().collect::<Vec<_>>();
    leaders.sort_by(|left, right| {
        let left_abs = left.signed_notional_flow_30s.unwrap_or(0.0).abs();
        let right_abs = right.signed_notional_flow_30s.unwrap_or(0.0).abs();
        right_abs
            .partial_cmp(&left_abs)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| display_symbol(left).cmp(display_symbol(right)))
    });

    let mut lines = vec![
        Line::from(format!("Selected flow {}", display_symbol(selected))),
        Line::from(format!(
            "flow30 {} | OFI {}",
            format_usd_signed(selected.signed_notional_flow_30s),
            format_usd_signed(selected.bbo_ofi_proxy_30s)
        )),
        Line::from(format!(
            "ret1m {} | rv1m {} | spread {} bps",
            format_signed(selected.ret_1m.map(|value| value * 100.0), "%"),
            format_optional(selected.rv_1m, 2),
            format_optional(selected.spread_bps, 1)
        )),
        Line::from("Flow leaderboard"),
    ];

    let limit = model.ui_state.visible_row_limit().min(10);
    lines.extend(leaders.into_iter().take(limit).map(|row| {
        Line::from(format!(
            "{} flow {} OFI {}",
            display_symbol(row),
            format_usd_signed(row.signed_notional_flow_30s),
            format_usd_signed(row.bbo_ofi_proxy_30s)
        ))
    }));
    lines.push(Line::from("Public BBO/flow proxy; no private fills."));
    lines
}

fn render_status_bar(
    frame: &mut Frame<'_>,
    area: Rect,
    model: &RatatuiFrameModel,
    color_mode: RatatuiColorMode,
) {
    let status = format!(
        " {} | {} | focus {} | {} | No wallet, no private streams, no order routes. Screen heuristic, not advice. ",
        model.health_status,
        pause_label(model),
        model.ui_state.focused_pane().label(),
        mode_label(&model.request, model.rows.len())
    );
    frame.render_widget(
        Paragraph::new(status)
            .style(Style::default().fg(warn(color_mode)))
            .block(
                Block::default()
                    .borders(Borders::TOP)
                    .border_style(focus_style(
                        model.ui_state.focused_pane() == WorkstationPane::Status,
                        color_mode,
                    )),
            ),
        area,
    );
}

fn render_status_panel(
    frame: &mut Frame<'_>,
    area: Rect,
    model: &RatatuiFrameModel,
    color_mode: RatatuiColorMode,
) {
    let rows = screened_rows(model);
    let lines = vec![
        Line::from(vec![
            Span::styled(
                "stream ",
                Style::default()
                    .fg(accent(color_mode))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(model.stream_status.clone()),
            Span::raw("  "),
            Span::styled(
                "recorder ",
                Style::default()
                    .fg(accent(color_mode))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(model.recorder_status.clone()),
        ]),
        Line::from(format!("health {}", model.health_status)),
        Line::from(format!(
            "view {} | pane {} | density {} | chart {}",
            model.ui_state.view().label(),
            model.ui_state.focused_pane().label(),
            model.ui_state.density().label(),
            model.ui_state.chart_window().label()
        )),
        Line::from(format!(
            "screen {} | rows {} | display {}",
            mode_label(&model.request, rows.len()),
            rows.len(),
            pause_label(model)
        )),
        Line::from("controls j/k rows | 1-6 panes | tab views | / p s t commands"),
        Line::from("read-only safety: No wallet, no private streams, no order routes."),
        Line::from("Screen output is heuristic context only, not orders or advice."),
    ];
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: true })
            .block(panel_for(
                "STATUS",
                WorkstationPane::Status,
                model,
                color_mode,
            ))
            .style(Style::default().fg(text(color_mode))),
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

fn panel_for(
    title: &str,
    pane: WorkstationPane,
    model: &RatatuiFrameModel,
    color_mode: RatatuiColorMode,
) -> Block<'static> {
    let focused = model.ui_state.focused_pane() == pane;
    let title = if focused {
        format!(" [FOCUS] {title} ")
    } else {
        format!(" {title} ")
    };
    Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(focus_style(focused, color_mode))
}

fn focus_style(focused: bool, color_mode: RatatuiColorMode) -> Style {
    let style = if focused {
        Style::default().fg(warn(color_mode))
    } else {
        Style::default().fg(accent(color_mode))
    };
    if focused {
        style.add_modifier(Modifier::BOLD)
    } else {
        style
    }
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

fn format_size(value: Option<f64>) -> String {
    value.map_or_else(|| "-".to_owned(), format_volume)
}

fn format_optional(value: Option<f64>, decimals: usize) -> String {
    value.map_or_else(|| "-".to_owned(), |value| format!("{value:.decimals$}"))
}

fn format_plain_number(value: f64) -> String {
    format!("{value:.4}")
}

fn format_volume(value: f64) -> String {
    if value >= 1_000_000.0 {
        format!("{:.1}M", value / 1_000_000.0)
    } else if value >= 10_000.0 {
        format!("{:.1}K", value / 1_000.0)
    } else if value.fract().abs() < f64::EPSILON {
        format!("{value:.0}")
    } else {
        format!("{value:.1}")
    }
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

fn signed_meter(value: f64) -> String {
    let normalized = value.clamp(-1.0, 1.0);
    let center = 5_i32;
    let marker = ((normalized + 1.0) * center as f64).round() as i32;
    (0..=10)
        .map(|index| {
            if index == center {
                '|'
            } else if index == marker {
                '█'
            } else {
                '─'
            }
        })
        .collect()
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

fn danger(color_mode: RatatuiColorMode) -> Color {
    match color_mode {
        RatatuiColorMode::NoColor => Color::White,
        RatatuiColorMode::Auto | RatatuiColorMode::Color => Color::Red,
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
