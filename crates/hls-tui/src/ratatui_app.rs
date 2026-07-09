use hls_core::market_state::{
    CandleEvent, FeatureSnapshot, StalenessState, TradeEvent, TradeSide, TradeabilityState,
};
use hls_screen::{
    ScreenEngine, ScreenRequest,
    presets::{builtin_presets, find_preset},
};
use ratatui::{
    Frame, Terminal,
    backend::TestBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, Wrap},
};

use crate::interaction::{
    WorkstationChartWindow, WorkstationCommand, WorkstationPane, WorkstationUiState,
    WorkstationView,
};

const MAX_CHART_CANDLES: usize = 48;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RatatuiColorMode {
    Auto,
    Color,
    NoColor,
}

impl RatatuiColorMode {
    fn label(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Color => "color",
            Self::NoColor => "no-color",
        }
    }

    fn palette_label(self) -> &'static str {
        match self {
            Self::NoColor => "plain",
            Self::Auto | Self::Color => "ansi",
        }
    }
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
    trades: Vec<TradeEvent>,
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
            trades: Vec::new(),
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

    pub fn with_trades(mut self, trades: Vec<TradeEvent>) -> Self {
        self.trades = trades;
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
    for y in area.y..area.y + area.height {
        let mut line = String::new();
        let last_visible_x = (area.x..area.x + area.width)
            .rev()
            .find(|x| !buffer[(*x, y)].symbol().trim().is_empty());
        if let Some(last_visible_x) = last_visible_x {
            let mut active_fg = Color::Reset;
            let mut active_bg = Color::Reset;
            for x in area.x..=last_visible_x {
                let cell = &buffer[(x, y)];
                if color_mode == RatatuiColorMode::Color && cell.fg != active_fg {
                    push_ansi_fg(&mut line, cell.fg);
                    active_fg = cell.fg;
                }
                if color_mode == RatatuiColorMode::Color && cell.bg != active_bg {
                    push_ansi_bg(&mut line, cell.bg);
                    active_bg = cell.bg;
                }
                line.push_str(cell.symbol());
            }
            if color_mode == RatatuiColorMode::Color
                && (active_fg != Color::Reset || active_bg != Color::Reset)
            {
                line.push_str("\x1b[0m");
            }
        }
        rendered.push_str(&line);
        rendered.push('\n');
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
            Constraint::Length(7),
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

    let detail_height = adaptive_detail_height(model.ui_state.view(), body[1].height, 12);
    let center = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(detail_height), Constraint::Min(12)])
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
            Constraint::Length(6),
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

    let detail_height = adaptive_detail_height(model.ui_state.view(), body[1].height, 19);
    let center = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(detail_height),
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

fn adaptive_detail_height(
    view: WorkstationView,
    available_height: u16,
    reserved_height: u16,
) -> u16 {
    let desired = match view {
        WorkstationView::Overview | WorkstationView::Flow | WorkstationView::Explain => 10,
        WorkstationView::Quality | WorkstationView::Metadata => 8,
    };
    let max_without_starving_neighbors = available_height.saturating_sub(reserved_height).max(6);
    desired.min(max_without_starving_neighbors).max(6)
}

fn render_narrow(
    frame: &mut Frame<'_>,
    area: Rect,
    model: &RatatuiFrameModel,
    color_mode: RatatuiColorMode,
) {
    let watchlist_height = if model.ui_state.focused_pane() == WorkstationPane::Status {
        Constraint::Percentage(36)
    } else {
        Constraint::Percentage(48)
    };
    let drilldown_height = if model.ui_state.focused_pane() == WorkstationPane::Status {
        Constraint::Min(10)
    } else {
        Constraint::Min(8)
    };
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            watchlist_height,
            drilldown_height,
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
    let narrow = area.width < 90;
    let mode_label = if narrow {
        narrow_ui_mode_label(&model.ui_state)
    } else {
        compact_ui_mode_label(&model.ui_state)
    };
    let status_tail = if narrow {
        format!("  {mode_label}")
    } else {
        format!("  {mode_label}  filter:{filter}")
    };
    let mut text = vec![Line::from(vec![
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
        Span::raw(status_tail),
    ])];
    if !narrow {
        text.push(desk_tab_rail_line(
            &model.ui_state,
            area.width < 132,
            color_mode,
        ));
    }
    text.extend([
        Line::from(vec![
            Span::styled(
                "CONTROLS ",
                Style::default()
                    .fg(accent(color_mode))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(pane_hotkey_rail(&model.ui_state, narrow)),
            Span::raw(if narrow {
                " | j/k 1-6 tab / p s t ? q"
            } else {
                " | j/k row 1-6 panes tab views / p s t ? q"
            }),
        ]),
        market_internals_line(model, color_mode, narrow),
    ]);
    if !narrow && area.height >= 7 {
        text.push(market_pulse_line(model, color_mode));
    }
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

fn desk_tab_rail_line(
    state: &WorkstationUiState,
    compact: bool,
    color_mode: RatatuiColorMode,
) -> Line<'static> {
    let panes = [
        (WorkstationPane::Watchlist, "WATCHLIST 1", "W1"),
        (WorkstationPane::Detail, "DETAIL 2", "D2"),
        (WorkstationPane::Chart, "CHART 3", "C3"),
        (WorkstationPane::Book, "BOOK 4", "B4"),
        (WorkstationPane::Tape, "TAPE 5", "T5"),
        (WorkstationPane::Status, "OPS 6", "O6"),
    ];
    let mut spans = vec![Span::styled(
        "DESK ",
        Style::default()
            .fg(accent(color_mode))
            .add_modifier(Modifier::BOLD),
    )];
    for (index, (pane, label, short_label)) in panes.iter().enumerate() {
        if index > 0 {
            spans.push(Span::raw(" "));
        }
        let tab_label = if compact { *short_label } else { *label };
        if state.focused_pane() == *pane {
            spans.push(Span::styled(
                format!("[{tab_label}]"),
                Style::default()
                    .fg(warn(color_mode))
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            spans.push(Span::raw(tab_label.to_owned()));
        }
    }
    if compact {
        spans.push(Span::raw(format!(
            " | v {} | d {} | read-only",
            state.view().label(),
            state.density().label()
        )));
    } else {
        spans.push(Span::raw(format!(
            " | view {} | density {} | read-only",
            state.view().label(),
            state.density().label()
        )));
    }
    Line::from(spans)
}

fn pane_hotkey_rail(state: &WorkstationUiState, narrow: bool) -> String {
    let panes = [
        (WorkstationPane::Watchlist, "1W", "1 WATCH"),
        (WorkstationPane::Detail, "2D", "2 DETAIL"),
        (WorkstationPane::Chart, "3C", "3 CHART"),
        (WorkstationPane::Book, "4B", "4 BOOK"),
        (WorkstationPane::Tape, "5T", "5 TAPE"),
        (WorkstationPane::Status, "6S", "6 STATUS"),
    ];
    panes
        .iter()
        .map(|(pane, compact, full)| {
            let label = if narrow { *compact } else { *full };
            if state.focused_pane() == *pane {
                format!("[{label}]")
            } else {
                label.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn market_internals_line(
    model: &RatatuiFrameModel,
    color_mode: RatatuiColorMode,
    compact: bool,
) -> Line<'static> {
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
    if compact {
        return Line::from(vec![
            Span::styled(
                "INT ",
                Style::default()
                    .fg(accent(color_mode))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!(
                "rows {:02} up {:02} dn {:02} heat {} tr {:02} st {:02} fl {} dp {}",
                rows.len().min(99),
                up.min(99),
                down.min(99),
                market_heat_bar(up, down),
                tradeable.min(99),
                stale.min(99),
                format_usd_signed(Some(signed_flow)),
                format_usd(Some(depth))
            )),
        ]);
    }

    Line::from(vec![
        Span::styled(
            "INTERNALS ",
            Style::default()
                .fg(accent(color_mode))
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!(
            "rows {:02}  heat {}  up {:02} down {:02}  tradeable {:02} stale {:02}  flow {}  depth {}",
            rows.len().min(99),
            market_heat_bar(up, down),
            up.min(99),
            down.min(99),
            tradeable.min(99),
            stale.min(99),
            format_usd_signed(Some(signed_flow)),
            format_usd(Some(depth))
        )),
    ])
}

fn market_pulse_line(model: &RatatuiFrameModel, color_mode: RatatuiColorMode) -> Line<'static> {
    let rows = screened_rows(model);
    let up = rows
        .iter()
        .filter(|row| row.ret_1m.is_some_and(|value| value > 0.0))
        .count();
    let down = rows
        .iter()
        .filter(|row| row.ret_1m.is_some_and(|value| value < 0.0))
        .count();
    let move_leader = rows
        .iter()
        .filter_map(|row| {
            row.ret_1m
                .filter(|value| value.is_finite())
                .map(|value| (row, value))
        })
        .max_by(|(_, left), (_, right)| {
            left.abs()
                .partial_cmp(&right.abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    let flow_leader = rows
        .iter()
        .filter_map(|row| {
            row.signed_notional_flow_30s
                .filter(|value| value.is_finite())
                .map(|value| (row, value))
        })
        .max_by(|(_, left), (_, right)| {
            left.abs()
                .partial_cmp(&right.abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    let move_text = move_leader.map_or_else(
        || "move -".to_owned(),
        |(row, _)| format!("move {} {}", display_symbol(row), trend_label(row.ret_1m)),
    );
    let flow_text = flow_leader.map_or_else(
        || "flow -".to_owned(),
        |(row, flow)| {
            format!(
                "flow {} {}",
                display_symbol(row),
                format_usd_signed(Some(flow))
            )
        },
    );

    Line::from(vec![
        Span::styled(
            "MARKET PULSE ",
            Style::default()
                .fg(accent(color_mode))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("regime {} ", market_regime_label(up, down)),
            Style::default()
                .fg(market_regime_color(up, down, color_mode))
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!("pulse {}  ", market_heat_bar(up, down))),
        Span::raw(format!(
            "breadth {:02}/{:02}  {}  {}  public rows",
            up.min(99),
            down.min(99),
            move_text,
            flow_text
        )),
    ])
}

fn market_regime_label(up: usize, down: usize) -> &'static str {
    let total = up + down;
    if total == 0 {
        return "idle";
    }
    let up_ratio = up as f64 / total as f64;
    if up_ratio >= 0.65 {
        "risk-on"
    } else if up_ratio <= 0.35 {
        "risk-off"
    } else {
        "mixed"
    }
}

fn market_regime_color(up: usize, down: usize, color_mode: RatatuiColorMode) -> Color {
    match market_regime_label(up, down) {
        "risk-on" => success(color_mode),
        "risk-off" => danger(color_mode),
        "mixed" => warn(color_mode),
        _ => text(color_mode),
    }
}

fn market_heat_bar(up: usize, down: usize) -> String {
    let total = up + down;
    if total == 0 {
        return "----".to_owned();
    }
    let up_slots = ((up * 4) + (total / 2)) / total;
    let down_slots = 4usize.saturating_sub(up_slots);
    format!("{}{}", "█".repeat(up_slots), "░".repeat(down_slots))
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
    let compact = area.width <= 64;
    let enhanced = !compact && area.width >= 72 && !model.candles.is_empty();
    let show_row_router = !compact && area.width >= 72 && area.height >= 18 && !rows.is_empty();
    let row_router_height = if area.height >= 20 { 5 } else { 4 };
    let chunks = if show_row_router {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(8), Constraint::Length(row_router_height)])
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1)])
            .split(area)
    };
    let table_area = chunks[0];
    let visible_range = watchlist_visible_range(
        selected,
        rows.len(),
        model.ui_state.visible_row_limit(),
        table_area.height,
    );
    let title = if rows.is_empty() {
        "WATCHLIST 0/0".to_owned()
    } else {
        format!(
            "WATCHLIST {}/{} VIEW {:02}-{:02}{}",
            selected + 1,
            rows.len(),
            visible_range.start + 1,
            visible_range.end,
            if enhanced { " 1m spark" } else { "" }
        )
    };
    let table_rows = rows
        .iter()
        .enumerate()
        .skip(visible_range.start)
        .take(visible_range.len())
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
                    Cell::from(watchlist_rank_label(index, selected)),
                    Cell::from(display_symbol(row).to_owned()),
                    Cell::from(format_board_price(row.price)),
                    Cell::from(micro_heat_lane(row, true)),
                    Cell::from(trend_label(row.ret_1m)),
                    Cell::from(format_usd_signed(row.signed_notional_flow_30s)),
                    Cell::from(quality_badge(row)),
                ])
                .style(style)
            } else if enhanced {
                Row::new(vec![
                    Cell::from(watchlist_rank_label(index, selected)),
                    Cell::from(display_symbol(row).to_owned()),
                    Cell::from(format_board_price(row.price)),
                    Cell::from(watchlist_candle_sparkline(&model.candles, &row.symbol, 5)),
                    Cell::from(score_signal_label(row)),
                    Cell::from(score_edge_bar(row)),
                    Cell::from(micro_heat_lane(row, false)),
                    Cell::from(score_bias_label(row)),
                    Cell::from(format_optional(row.spread_bps, 1)),
                    Cell::from(trend_label(row.ret_1m)),
                    Cell::from(format_usd_signed(row.signed_notional_flow_30s)),
                    Cell::from(format_usd(row.tob_depth_usd)),
                    Cell::from(quality_badge(row)),
                ])
                .style(style)
            } else {
                Row::new(vec![
                    Cell::from(watchlist_rank_label(index, selected)),
                    Cell::from(display_symbol(row).to_owned()),
                    Cell::from(format_board_price(row.price)),
                    Cell::from(score_signal_label(row)),
                    Cell::from(score_edge_bar(row)),
                    Cell::from(micro_heat_lane(row, false)),
                    Cell::from(score_bias_label(row)),
                    Cell::from(format_optional(row.spread_bps, 1)),
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
                Constraint::Length(6),
                Constraint::Length(4),
                Constraint::Length(8),
                Constraint::Length(6),
                Constraint::Length(1),
            ],
        )
        .header(
            Row::new(["RK", "CODE", "PX", "HT", "1M", "FLOW", "Q"]).style(
                Style::default()
                    .fg(accent(color_mode))
                    .add_modifier(Modifier::BOLD),
            ),
        )
    } else if enhanced {
        Table::new(
            table_rows,
            [
                Constraint::Length(4),
                Constraint::Min(8),
                Constraint::Length(7),
                Constraint::Length(5),
                Constraint::Length(3),
                Constraint::Length(5),
                Constraint::Length(5),
                Constraint::Length(4),
                Constraint::Length(4),
                Constraint::Length(7),
                Constraint::Length(6),
                Constraint::Length(5),
                Constraint::Length(1),
            ],
        )
        .header(
            Row::new([
                "RANK", "CODE", "PX", "SPK", "SIG", "EDGE", "HEAT", "BIAS", "SPR", "1M", "FLOW30",
                "DEPTH", "Q",
            ])
            .style(
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
                Constraint::Min(8),
                Constraint::Length(7),
                Constraint::Length(3),
                Constraint::Length(5),
                Constraint::Length(5),
                Constraint::Length(4),
                Constraint::Length(4),
                Constraint::Length(7),
                Constraint::Length(6),
                Constraint::Length(5),
                Constraint::Length(1),
            ],
        )
        .header(
            Row::new([
                "RANK", "CODE", "PX", "SIG", "EDGE", "HEAT", "BIAS", "SPR", "1M", "FLOW30",
                "DEPTH", "Q",
            ])
            .style(
                Style::default()
                    .fg(accent(color_mode))
                    .add_modifier(Modifier::BOLD),
            ),
        )
    }
    .column_spacing(1)
    .block(panel_for(
        &title,
        WorkstationPane::Watchlist,
        model,
        color_mode,
    ));
    frame.render_widget(table, table_area);
    if show_row_router {
        if let Some(row) = selected_row(&rows, model) {
            frame.render_widget(
                Paragraph::new(watchlist_row_router_lines(row, &rows, color_mode))
                    .wrap(Wrap { trim: true })
                    .style(Style::default().fg(text(color_mode)))
                    .block(
                        Block::default()
                            .borders(Borders::TOP)
                            .border_style(Style::default().fg(accent(color_mode))),
                    ),
                chunks[1],
            );
        }
    }
}

fn watchlist_rank_label(index: usize, selected: usize) -> String {
    if index == selected {
        format!(">{:02}", index + 1)
    } else {
        format!(" {:02}", index + 1)
    }
}

fn watchlist_visible_range(
    selected: usize,
    row_count: usize,
    density_limit: usize,
    area_height: u16,
) -> std::ops::Range<usize> {
    if row_count == 0 || density_limit == 0 {
        return 0..0;
    }

    let table_row_capacity = usize::from(area_height.saturating_sub(3)).max(1);
    let capacity = density_limit.min(table_row_capacity).min(row_count);
    let selected = selected.min(row_count - 1);
    let mut start = selected.saturating_sub(capacity / 2);
    if start + capacity > row_count {
        start = row_count - capacity;
    }
    start..start + capacity
}

fn watchlist_candle_sparkline(candles: &[CandleEvent], symbol: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    let mut closes = candles
        .iter()
        .filter(|candle| candle.hl_coin == symbol && candle.interval == "1m")
        .map(|candle| (candle.open_ts_ms, candle.close))
        .filter(|(_, close)| close.is_finite())
        .collect::<Vec<_>>();
    closes.sort_by_key(|(ts, _)| *ts);
    if closes.len() > width {
        closes.drain(0..closes.len() - width);
    }
    if closes.is_empty() {
        return "-".repeat(width);
    }

    let min = closes
        .iter()
        .map(|(_, close)| *close)
        .fold(f64::INFINITY, f64::min);
    let max = closes
        .iter()
        .map(|(_, close)| *close)
        .fold(f64::NEG_INFINITY, f64::max);
    let range = max - min;
    let body = closes
        .iter()
        .map(|(_, close)| {
            if range.abs() < f64::EPSILON {
                '▄'
            } else {
                spark_price_glyph((*close - min) / range)
            }
        })
        .collect::<String>();
    left_pad_to_width(body, width)
}

fn spark_price_glyph(ratio: f64) -> char {
    match (ratio.clamp(0.0, 1.0) * 7.0).round() as u8 {
        0 => '▁',
        1 => '▂',
        2 => '▃',
        3 => '▄',
        4 => '▅',
        5 => '▆',
        6 => '▇',
        _ => '█',
    }
}

fn left_pad_to_width(value: String, width: usize) -> String {
    let len = value.chars().count();
    if len >= width {
        value
    } else {
        format!("{}{}", " ".repeat(width - len), value)
    }
}

fn watchlist_row_router_lines(
    row: &FeatureSnapshot,
    rows: &[FeatureSnapshot],
    color_mode: RatatuiColorMode,
) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(vec![
            Span::styled(
                "ROW ROUTER ",
                Style::default()
                    .fg(accent(color_mode))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!("selected {} | ", display_symbol(row))),
            Span::raw(format!("spr {}bps | ", format_optional(row.spread_bps, 1))),
            Span::styled(
                format!("flow {}", format_usd_signed(row.signed_notional_flow_30s)),
                Style::default().fg(flow_color(
                    row.signed_notional_flow_30s.unwrap_or_default(),
                    color_mode,
                )),
            ),
        ]),
        Line::from(format!(
            "trade {} | quality {} | j/k move | tab detail",
            row.tradeability_state.as_str(),
            quality_badge(row)
        )),
    ];
    lines.extend(watchlist_scanner_rail_lines(rows, color_mode));
    lines
}

fn watchlist_scanner_rail_lines(
    rows: &[FeatureSnapshot],
    color_mode: RatatuiColorMode,
) -> Vec<Line<'static>> {
    if rows.is_empty() {
        return vec![Line::from("read-only row context")];
    }
    let move_leader = rows
        .iter()
        .filter_map(|row| {
            row.ret_1m
                .filter(|value| value.is_finite())
                .map(|value| (row, value))
        })
        .max_by(|(_, left), (_, right)| {
            left.abs()
                .partial_cmp(&right.abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    let flow_leader = rows
        .iter()
        .filter_map(|row| {
            row.signed_notional_flow_30s
                .filter(|value| value.is_finite())
                .map(|value| (row, value))
        })
        .max_by(|(_, left), (_, right)| {
            left.abs()
                .partial_cmp(&right.abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    let depth_leader = rows
        .iter()
        .filter_map(|row| {
            row.tob_depth_usd
                .filter(|value| value.is_finite())
                .map(|value| (row, value))
        })
        .max_by(|(_, left), (_, right)| {
            left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal)
        });

    let mut first_line = vec![
        Span::styled(
            "SCANNER RAIL ",
            Style::default()
                .fg(accent(color_mode))
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("read-only row context | "),
    ];
    if let Some((row, _)) = move_leader {
        first_line.push(Span::styled(
            format!("mover {} {}", display_symbol(row), trend_label(row.ret_1m)),
            market_row_style(row, color_mode).add_modifier(Modifier::BOLD),
        ));
    } else {
        first_line.push(Span::raw("mover -".to_owned()));
    }
    let mut second_line = vec![Span::raw("read-only scan | ")];
    if let Some((row, _)) = flow_leader {
        second_line.push(Span::styled(
            format!(
                "flow {} {}",
                display_symbol(row),
                format_usd_signed(row.signed_notional_flow_30s)
            ),
            Style::default()
                .fg(flow_color(
                    row.signed_notional_flow_30s.unwrap_or_default(),
                    color_mode,
                ))
                .add_modifier(Modifier::BOLD),
        ));
    } else {
        second_line.push(Span::raw("flow -".to_owned()));
    }
    second_line.push(Span::raw(" | "));
    if let Some((row, depth)) = depth_leader {
        second_line.push(Span::styled(
            format!("depth {} {}", display_symbol(row), format_usd(Some(depth))),
            Style::default()
                .fg(success(color_mode))
                .add_modifier(Modifier::BOLD),
        ));
    } else {
        second_line.push(Span::raw("depth -".to_owned()));
    }

    vec![Line::from(first_line), Line::from(second_line)]
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

fn score_signal_label(row: &FeatureSnapshot) -> String {
    format!("{:.0}", score_signal_value(row))
}

fn score_signal_value(row: &FeatureSnapshot) -> f64 {
    row.score_breakdown.as_ref().map_or_else(
        || (row.liquidity_score + row.momentum_score).clamp(0.0, 99.0),
        |breakdown| breakdown.adjusted_total.clamp(0.0, 99.0),
    )
}

fn score_edge_bar(row: &FeatureSnapshot) -> String {
    let width = 4;
    let ratio = (score_signal_value(row) / 100.0).clamp(0.0, 1.0);
    let filled = ((ratio * width as f64).round() as usize).clamp(1, width);
    format!(
        "{}{}{}",
        edge_direction_glyph(row),
        "█".repeat(filled),
        "░".repeat(width.saturating_sub(filled))
    )
}

fn micro_heat_lane(row: &FeatureSnapshot, compact: bool) -> String {
    let spread_quality = row
        .spread_bps
        .filter(|value| value.is_finite())
        .map(|value| 1.0 - (value / 20.0).clamp(0.0, 1.0))
        .unwrap_or(0.0);
    let depth_quality = row
        .tob_depth_usd
        .filter(|value| value.is_finite() && *value > 0.0)
        .map(|value| (value.log10() / 6.0).clamp(0.0, 1.0))
        .unwrap_or(0.0);
    let liquidity_heat = (spread_quality * 0.55) + (depth_quality * 0.45);
    let flow = row.signed_notional_flow_30s.unwrap_or(0.0);
    let flow_glyph = if flow > 0.0 {
        '+'
    } else if flow < 0.0 {
        '-'
    } else {
        '='
    };
    let width = if compact { 2 } else { 3 };
    format!(
        "{}{}{}",
        edge_direction_glyph(row),
        depth_bar(liquidity_heat, width),
        flow_glyph
    )
}

fn edge_direction_glyph(row: &FeatureSnapshot) -> &'static str {
    if row.ret_1m.unwrap_or(0.0) > 0.0 {
        "▲"
    } else if row.ret_1m.unwrap_or(0.0) < 0.0 {
        "▼"
    } else if row.signed_notional_flow_30s.unwrap_or(0.0) > 0.0 {
        "▲"
    } else if row.signed_notional_flow_30s.unwrap_or(0.0) < 0.0 {
        "▼"
    } else {
        "◆"
    }
}

fn score_bias_label(row: &FeatureSnapshot) -> String {
    let Some(breakdown) = row.score_breakdown.as_ref() else {
        if row.momentum_score.abs() >= row.liquidity_score.abs() {
            return "MOM+".to_owned();
        }
        return "LIQ+".to_owned();
    };
    breakdown
        .components
        .iter()
        .max_by(|left, right| {
            left.signed_contribution
                .abs()
                .partial_cmp(&right.signed_contribution.abs())
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| right.name.cmp(&left.name))
        })
        .map(|component| {
            let prefix = compact_factor_name(&component.name).to_ascii_uppercase();
            let sign = if component.signed_contribution < 0.0 {
                '-'
            } else {
                '+'
            };
            format!("{prefix}{sign}")
        })
        .unwrap_or_else(|| "-".to_owned())
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

    let lines = detail_lines(
        row,
        model.ui_state.view(),
        color_mode,
        area.width,
        title == "DETAIL",
    );
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
    width: u16,
    force_compact: bool,
) -> Vec<Line<'static>> {
    let compact = force_compact || width <= 72;
    let tabs = detail_view_tabs_line(view, color_mode, compact);
    let heading = detail_heading_line(row, color_mode, compact);

    match view {
        WorkstationView::Overview => {
            let show_pair_snapshot = !compact && width >= 96;
            let mut lines = vec![
                heading,
                tabs,
                quote_strip_line(row, color_mode, show_pair_snapshot),
            ];
            if compact {
                lines.insert(2, selected_bbo_line(row, color_mode));
            }
            lines.extend(factor_stack_lines(row, color_mode, compact));
            lines.extend(liquidity_radar_lines(row, color_mode));
            lines.extend([Line::from(format!(
                "flow30 {} | bbo ofi {} | depth {} | imbalance {}",
                format_usd_signed(row.signed_notional_flow_30s),
                format_usd_signed(row.bbo_ofi_proxy_30s),
                format_usd(row.tob_depth_usd),
                format_signed(row.tob_imbalance, "")
            ))]);
            lines
        }
        WorkstationView::Flow => {
            let flow = row.signed_notional_flow_30s.unwrap_or(0.0);
            let imbalance = row.tob_imbalance.unwrap_or(0.0);
            let flow_scale = flow
                .abs()
                .max(row.tob_depth_usd.unwrap_or(0.0).abs())
                .max(1.0);
            let bar_width = if compact { 8 } else { 12 };

            vec![
                heading,
                tabs,
                Line::from("Flow tape | Public BBO/trade context only"),
                Line::from(vec![
                    Span::styled(
                        "FLOW LADDER ",
                        Style::default()
                            .fg(accent(color_mode))
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw("public microstructure console"),
                ]),
                Line::from(format!(
                    "signed flow 30s {} | bbo ofi {} | depth {}",
                    format_usd_signed(row.signed_notional_flow_30s),
                    format_usd_signed(row.bbo_ofi_proxy_30s),
                    format_usd(row.tob_depth_usd),
                )),
                Line::from(vec![
                    Span::styled(
                        "pressure ",
                        Style::default().fg(flow_color(flow, color_mode)),
                    ),
                    Span::raw(signed_flow_bar(flow, flow_scale, bar_width)),
                    Span::raw(" | "),
                    Span::styled(
                        "imbalance ",
                        Style::default().fg(flow_color(imbalance, color_mode)),
                    ),
                    Span::raw(signed_meter(imbalance)),
                    Span::raw(format!(" {}", format_signed(row.tob_imbalance, ""))),
                ]),
                Line::from(format!(
                    "friction spr {} bps | recovery {} | adverse {}",
                    format_optional(row.spread_bps, 1),
                    format_duration_ms(row.spread_recovery_ms),
                    row.adverse_selection_proxy.as_str()
                )),
                Line::from("Public BBO/trade context only | display heuristic, not advice."),
            ]
        }
        WorkstationView::Quality => vec![
            heading,
            tabs,
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
            tabs,
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
        WorkstationView::Explain => {
            let mut lines = vec![heading, tabs];
            lines.extend(why_ranked_deck_lines(row, color_mode));
            lines.extend(factor_stack_lines(row, color_mode, compact));
            lines
        }
    }
}

fn detail_heading_line(
    row: &FeatureSnapshot,
    color_mode: RatatuiColorMode,
    compact: bool,
) -> Line<'static> {
    let mut spans = vec![
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
    ];
    if !compact {
        spans.push(Span::raw(format!(
            "  confidence {} {}",
            row.confidence.level.as_str(),
            row.confidence.score
        )));
    }
    Line::from(spans)
}

fn quote_strip_line(
    row: &FeatureSnapshot,
    color_mode: RatatuiColorMode,
    show_pair_snapshot: bool,
) -> Line<'static> {
    let mut spans = vec![Span::styled(
        "QUOTE STRIP ",
        Style::default()
            .fg(accent(color_mode))
            .add_modifier(Modifier::BOLD),
    )];
    if show_pair_snapshot {
        spans.extend(pair_snapshot_spans(row, color_mode));
        spans.push(Span::raw(" | "));
    }
    spans.extend([
        Span::styled("bid ", Style::default().fg(success(color_mode))),
        Span::raw(format!("{}  ", format_price(row.bid_px))),
        Span::styled("ask ", Style::default().fg(danger(color_mode))),
        Span::raw(format!("{}  ", format_price(row.ask_px))),
        Span::raw(format!("mid {}  ", format_price(mid_price(row)))),
        Span::raw("read-only quote"),
    ]);
    Line::from(spans)
}

fn pair_snapshot_spans(row: &FeatureSnapshot, color_mode: RatatuiColorMode) -> Vec<Span<'static>> {
    vec![
        Span::styled(
            "PAIR SNAPSHOT ",
            Style::default()
                .fg(accent(color_mode))
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("read-only selected pair | "),
        Span::raw(format!(
            "trade {} | resilience {} | freshness {} | ",
            row.tradeability_state.as_str(),
            row.resilience_state.as_str(),
            staleness_label(&row.staleness_state)
        )),
        Span::styled(
            format!("conf {}", row.confidence.score),
            Style::default().fg(confidence_color(row.confidence.score, color_mode)),
        ),
    ]
}

fn staleness_label(staleness: &StalenessState) -> &'static str {
    match staleness {
        StalenessState::Fresh => "fresh",
        StalenessState::Stale => "stale",
        StalenessState::Incomplete => "incomplete",
    }
}

fn mid_price(row: &FeatureSnapshot) -> Option<f64> {
    match (row.bid_px, row.ask_px) {
        (Some(bid), Some(ask)) if bid.is_finite() && ask.is_finite() => Some((bid + ask) / 2.0),
        _ => row.price,
    }
}

fn selected_bbo_line(row: &FeatureSnapshot, color_mode: RatatuiColorMode) -> Line<'static> {
    let bid_notional = notional(row.bid_px, row.bid_sz);
    let ask_notional = notional(row.ask_px, row.ask_sz);
    Line::from(vec![
        Span::styled(
            "BBO ",
            Style::default()
                .fg(accent(color_mode))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("bid ", Style::default().fg(success(color_mode))),
        Span::raw(format!(
            "{} {}",
            format_price(row.bid_px),
            format_usd(bid_notional)
        )),
        Span::raw(" / "),
        Span::styled("ask ", Style::default().fg(danger(color_mode))),
        Span::raw(format!(
            "{} {}",
            format_price(row.ask_px),
            format_usd(ask_notional)
        )),
        Span::raw(format!(
            " | depth {} | imb {}",
            format_usd(row.tob_depth_usd),
            format_signed(row.tob_imbalance, "")
        )),
    ])
}

fn detail_view_tabs_line(
    active: WorkstationView,
    color_mode: RatatuiColorMode,
    compact: bool,
) -> Line<'static> {
    let labels = [
        (WorkstationView::Overview, "overview", "ov"),
        (WorkstationView::Flow, "flow", "fl"),
        (WorkstationView::Quality, "quality", "ql"),
        (WorkstationView::Metadata, "metadata", "mt"),
        (WorkstationView::Explain, "explain", "ex"),
    ];
    let mut spans = vec![Span::styled(
        "VIEWS ",
        Style::default()
            .fg(accent(color_mode))
            .add_modifier(Modifier::BOLD),
    )];
    for (index, (view, full, short)) in labels.iter().enumerate() {
        if index > 0 {
            spans.push(Span::raw(" "));
        }
        let label = if compact { *short } else { *full };
        if *view == active {
            spans.push(Span::styled(
                format!("[{label}]"),
                Style::default()
                    .fg(warn(color_mode))
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            spans.push(Span::raw(label.to_owned()));
        }
    }
    Line::from(spans)
}

fn liquidity_radar_lines(
    row: &FeatureSnapshot,
    color_mode: RatatuiColorMode,
) -> Vec<Line<'static>> {
    let spread_quality = row
        .spread_bps
        .filter(|value| value.is_finite())
        .map(|value| 1.0 - (value / 25.0).clamp(0.0, 1.0))
        .unwrap_or(0.0);
    let depth_ratio = row
        .tob_depth_usd
        .filter(|value| value.is_finite() && *value > 0.0)
        .map(|value| (value.log10() / 6.0).clamp(0.0, 1.0))
        .unwrap_or(0.0);
    let flow = row.signed_notional_flow_30s.unwrap_or(0.0);
    let flow_scale = flow
        .abs()
        .max(row.tob_depth_usd.unwrap_or(0.0).abs())
        .max(1.0);

    vec![
        Line::from(vec![
            Span::styled(
                "LIQUIDITY RADAR ",
                Style::default()
                    .fg(accent(color_mode))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("Public BBO/flow only"),
        ]),
        Line::from(vec![
            Span::styled("spread cost ", Style::default().fg(warn(color_mode))),
            Span::raw(format!(
                "{} {} bps",
                depth_bar(spread_quality, 10),
                format_optional(row.spread_bps, 1)
            )),
            Span::raw(" | "),
            Span::styled("depth ", Style::default().fg(success(color_mode))),
            Span::raw(format!(
                "{} {}",
                depth_bar(depth_ratio, 10),
                format_usd(row.tob_depth_usd)
            )),
        ]),
        Line::from(vec![
            Span::styled(
                "imbalance ",
                Style::default().fg(flow_color(row.tob_imbalance.unwrap_or(0.0), color_mode)),
            ),
            Span::raw(signed_meter(row.tob_imbalance.unwrap_or(0.0))),
            Span::raw(" | "),
            Span::styled("flow ", Style::default().fg(flow_color(flow, color_mode))),
            Span::raw(signed_flow_bar(flow, flow_scale, 10)),
            Span::raw(format!(
                " {}",
                format_usd_signed(row.signed_notional_flow_30s)
            )),
        ]),
        Line::from("Public BBO/flow only | screen heuristic, not advice."),
    ]
}

fn factor_stack_lines(
    row: &FeatureSnapshot,
    color_mode: RatatuiColorMode,
    compact: bool,
) -> Vec<Line<'static>> {
    let Some(breakdown) = row.score_breakdown.as_ref() else {
        let label = if compact { "FACTORS" } else { "FACTOR STACK" };
        return vec![Line::from(format!(
            "{label} unavailable | confidence {}",
            row.confidence.score
        ))];
    };

    let mut components = breakdown.components.iter().collect::<Vec<_>>();
    components.sort_by(|left, right| {
        right
            .signed_contribution
            .abs()
            .partial_cmp(&left.signed_contribution.abs())
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.name.cmp(&right.name))
    });

    if compact {
        let compact_components = components
            .iter()
            .take(2)
            .map(|component| {
                format!(
                    "{} {} {}",
                    compact_factor_name(&component.name),
                    score_contribution_bar(component.signed_contribution, 4),
                    format_signed(Some(component.signed_contribution), "")
                )
            })
            .collect::<Vec<_>>()
            .join(" | ");
        return vec![Line::from(vec![
            Span::styled(
                "FACTORS ",
                Style::default()
                    .fg(accent(color_mode))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!(
                "raw {:.1} adj {:.1} c{} | {}",
                breakdown.raw_total,
                breakdown.adjusted_total,
                breakdown.confidence_score,
                compact_components
            )),
        ])];
    }

    let component_text = components
        .into_iter()
        .take(3)
        .map(|component| {
            format!(
                "{} {} {}",
                compact_factor_name(&component.name),
                score_contribution_bar(component.signed_contribution, 10),
                format_signed(Some(component.signed_contribution), "")
            )
        })
        .collect::<Vec<_>>()
        .join(" | ");

    vec![Line::from(vec![
        Span::styled(
            "FACTOR STACK ",
            Style::default()
                .fg(accent(color_mode))
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!(
            "score raw {:.1} adj {:.1} conf {} | {}",
            breakdown.raw_total,
            breakdown.adjusted_total,
            breakdown.confidence_score,
            component_text
        )),
    ])]
}

fn why_ranked_deck_lines(
    row: &FeatureSnapshot,
    color_mode: RatatuiColorMode,
) -> Vec<Line<'static>> {
    let Some(breakdown) = row.score_breakdown.as_ref() else {
        return vec![
            Line::from(vec![
                Span::styled(
                    "WHY RANKED ",
                    Style::default()
                        .fg(accent(color_mode))
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("score explanation unavailable"),
            ]),
            Line::from("unavailable evidence | row has no generated score breakdown"),
            Line::from("BBO/top-of-book proxy only | screen heuristic, not advice."),
        ];
    };

    let confidence_penalty = (breakdown.raw_total - breakdown.adjusted_total).max(0.0);
    let unavailable = if breakdown.unavailable_evidence.is_empty() {
        "none".to_owned()
    } else {
        breakdown.unavailable_evidence.join(", ")
    };
    let components = breakdown
        .components
        .iter()
        .take(4)
        .map(|component| {
            format!(
                "{} {} {}",
                component.name,
                component.direction.as_str(),
                format_signed(Some(component.signed_contribution), "")
            )
        })
        .collect::<Vec<_>>()
        .join(" | ");

    vec![
        Line::from(vec![
            Span::styled(
                "WHY RANKED ",
                Style::default()
                    .fg(accent(color_mode))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!("{} score explanation", display_symbol(row))),
        ]),
        Line::from(format!(
            "SCORE adjusted {:.1} | raw {:.1} | confidence penalty {:.1} | confidence {}",
            breakdown.adjusted_total,
            breakdown.raw_total,
            confidence_penalty,
            breakdown.confidence_score
        )),
        Line::from(format!("COMPONENTS {components}")),
        Line::from(format!("unavailable evidence | {unavailable}")),
        Line::from("BBO/top-of-book proxy only | screen heuristic, not advice."),
    ]
}

fn compact_factor_name(name: &str) -> &'static str {
    match name {
        "liquidity_resilience" => "liq",
        "momentum" => "mom",
        "mean_reversion_context" => "mean",
        "signed_flow" => "flow",
        "spread_cost" => "spread",
        _ => "factor",
    }
}

fn score_contribution_bar(value: f64, width: usize) -> String {
    let half = (width / 2).max(1);
    let ratio = (value.abs() / 25.0).clamp(0.0, 1.0);
    let filled = ((ratio * half as f64).round() as usize).min(half);
    if value < 0.0 {
        format!(
            "{}{}|{}",
            "░".repeat(half.saturating_sub(filled)),
            "█".repeat(filled),
            "░".repeat(half)
        )
    } else if value > 0.0 {
        format!(
            "{}|{}{}",
            "░".repeat(half),
            "█".repeat(filled),
            "░".repeat(half.saturating_sub(filled))
        )
    } else {
        format!("{}|{}", "░".repeat(half), "░".repeat(half))
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
    let popup = centered_rect(76, 54, area);
    frame.render_widget(Clear, popup);
    let state = &model.ui_state;
    let lines = vec![
        Line::from(vec![Span::styled(
            "Command Deck",
            Style::default()
                .fg(accent(color_mode))
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled(
                "STATE ",
                Style::default()
                    .fg(accent(color_mode))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!(
                "view {} | pane {} | density {} | focus {} | chart {} | {}",
                state.view().label(),
                state.focused_pane().label(),
                state.density().label(),
                state.focused_pane().label(),
                state.chart_window().label(),
                pause_label(model)
            )),
        ]),
        Line::from(vec![
            Span::styled(
                "KEY MATRIX ",
                Style::default()
                    .fg(warn(color_mode))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("arrows/j/k navigate | tab views | [ ] panes | space pause"),
        ]),
        Line::from("PANES 1W 2D 3C 4B 5T 6S | watchlist detail chart book tape status"),
        Line::from("MARKET OPS / filter p preset s sort | t chart window | d density"),
        Line::from("MOUSE wheel rows | click focus | terminal support required"),
        Line::from("j/k or arrows  act on focused pane: rows, detail view, or chart window"),
        Line::from("tab / shift-tab  cycle overview, flow, quality, metadata, explain"),
        Line::from("[ / ]  move pane focus: watchlist, detail, chart, book, tape, status"),
        Line::from("1-6 panes  watchlist, detail, chart, book, tape, status"),
        Line::from(format!(
            "PALETTE DIAGNOSTIC mode {} palette {} | truecolor ANSI | force --color always",
            color_mode.label(),
            color_mode.palette_label()
        )),
        Line::from("mouse wheel moves rows; click focuses panes when terminal mouse is available"),
        Line::from("/ filter  |  p preset  |  s sort  |  t chart window"),
        Line::from("d  density  |  space  pause display  |  ?  help  |  q  quit"),
        Line::from(vec![
            Span::styled(
                "READ-ONLY ",
                Style::default()
                    .fg(success(color_mode))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("public market data only | no wallet | no order routes"),
        ]),
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
    let popup = centered_rect(74, 54, area);
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
    let target = command.target().label();
    let input = if command.input().is_empty() {
        "<empty>"
    } else {
        command.input()
    };
    let mut lines = vec![
        Line::from("COMMAND CENTER"),
        Line::from(format!("TARGET {target} | INPUT {input}")),
        command_router_line(command),
        Line::from(active_command_context_line(&model.request)),
        command_result_preview_line(model),
        Line::from("KEYFLOW / filter | p preset | s sort | t timeframe | d density | ? help"),
        Line::from("GUARDRAILS read-only display mutation only | last valid screen retained"),
        Line::from(format!(
            "SCOPE read-only screened rows {} | view {} | pane {}",
            screened_rows(model).len(),
            model.ui_state.view().label(),
            model.ui_state.focused_pane().label()
        )),
        Line::from(format!(
            "visible rows {:02} | read-only command preview",
            screened_rows(model).len().min(99)
        )),
        Line::from(format!("{} > {input}", command.prompt())),
        Line::from(match command.target().label() {
            "filter" => "Enter apply filter | Esc cancel | empty clears custom filter",
            "preset" => "Enter apply preset | Esc cancel | empty clears preset",
            "sort" => "Enter apply sort | Esc cancel | empty clears custom sort",
            _ => "Enter apply | Esc cancel",
        }),
        Line::from("EXAMPLES"),
        Line::from(command_examples_line(target)),
        command_deck_line(command.target()),
        Line::from("SAFETY no orders | no wallet | public market data only"),
    ];
    if let Some(error) = model.ui_state.command_error() {
        lines.push(Line::from(format!("error: {error}")));
    }
    lines
}

fn command_router_line(command: &WorkstationCommand) -> Line<'static> {
    Line::from(format!(
        "COMMAND ROUTER target {} | Enter apply | Esc rollback | live ingestion continues",
        command.target().label()
    ))
}

fn command_result_preview_line(model: &RatatuiFrameModel) -> Line<'static> {
    let rows = screened_rows(model);
    let top = rows
        .first()
        .map_or_else(|| "-".to_owned(), |row| display_symbol(row).to_owned());
    let selected = selected_row(&rows, model)
        .map_or_else(|| "-".to_owned(), |row| display_symbol(row).to_owned());
    Line::from(format!(
        "RESULT PREVIEW rows {:02} | top {} | selected {} | last valid screen retained",
        rows.len().min(99),
        top,
        selected
    ))
}

fn active_command_context_line(request: &ScreenRequest) -> String {
    let preset = request.preset.as_deref().unwrap_or("-");
    let filter = request.where_expr.as_deref().unwrap_or("-");
    let sort = request.sort.as_deref().unwrap_or("-");
    format!("ACTIVE preset {preset} | filter {filter} | sort {sort}")
}

fn command_examples_line(target: &str) -> &'static str {
    match target {
        "filter" => "filter: spread_bps < 5 | abs(ret_1m) > 0.001 | confidence >= 70",
        "preset" => "preset: tight | resilient | metadata_partial | clear empty",
        "sort" => "sort: score desc | spread_bps asc | signed_flow desc",
        _ => "filter: spread_bps < 5 | preset: tight | sort: score desc",
    }
}

fn command_deck_line(target: crate::interaction::WorkstationCommandTarget) -> Line<'static> {
    match target {
        crate::interaction::WorkstationCommandTarget::Preset => {
            let names = builtin_presets()
                .into_iter()
                .map(|preset| preset.name)
                .collect::<Vec<_>>();
            Line::from(format!(
                "PRESET DECK {} | read-only presets",
                names.join(" ")
            ))
        }
        crate::interaction::WorkstationCommandTarget::Filter => Line::from(
            "FILTER DECK confidence_score spread_bps tradeability_state cohort_tag signed_notional_flow_30s",
        ),
        crate::interaction::WorkstationCommandTarget::Sort => Line::from(
            "SORT DECK score:desc spread_bps:asc signed_notional_flow_30s:desc listing_age_ms:asc",
        ),
    }
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

fn narrow_ui_mode_label(state: &WorkstationUiState) -> String {
    let command = state
        .command()
        .map(|command| format!(" cmd:{}", command.target().label()))
        .unwrap_or_default();
    format!(
        "v:{} p:{} d:{} c:{}{}",
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
    let show_prints_strip = show_chart_prints_strip(area, model, &row.symbol);
    let chart_trades = if show_prints_strip {
        selected_trades(model, &row.symbol, 4)
    } else {
        Vec::new()
    };
    let Some(latest) = candles.last() else {
        frame.render_widget(
            Paragraph::new({
                let mut lines = vec![chart_window_tabs_line(
                    model.ui_state.chart_window(),
                    color_mode,
                    area.width <= 72,
                )];
                lines.extend(selected_pair_edge_hud_lines(row, color_mode));
                if show_chart_order_pressure(area) {
                    lines.extend(selected_pair_order_pressure_lines(row, color_mode));
                }
                if show_prints_strip {
                    lines.extend(chart_prints_strip_lines(&chart_trades, color_mode));
                }
                lines.extend(chart_session_strip_lines(row, color_mode));
                lines.extend([
                    Line::from("Waiting for public 1m candle frames."),
                    Line::from("No synthetic candles are rendered."),
                ]);
                lines
            })
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
    let mut chart_lines = vec![chart_window_tabs_line(
        model.ui_state.chart_window(),
        color_mode,
        area.width <= 72,
    )];
    let show_order_pressure = show_chart_order_pressure(area);
    let show_crosshair_context = show_chart_crosshair_context(area);
    chart_lines.extend(selected_pair_edge_hud_lines(row, color_mode));
    if show_order_pressure {
        chart_lines.extend(selected_pair_order_pressure_lines(row, color_mode));
    }
    if show_prints_strip {
        chart_lines.extend(chart_prints_strip_lines(&chart_trades, color_mode));
    }
    chart_lines.push(chart_move_summary_line(&candles, color_mode));
    chart_lines.push(chart_candle_hud_line(latest, color_mode));
    if show_crosshair_context {
        chart_lines.extend(chart_crosshair_context_lines(row, &candles, color_mode));
    }
    chart_lines.extend(chart_session_strip_lines(row, color_mode));
    let chart_overhead = 11
        + u16::from(show_order_pressure) * 3
        + u16::from(show_prints_strip) * 3
        + u16::from(show_crosshair_context) * 2;
    chart_lines.extend(candle_chart_lines(
        &candles,
        area.height.saturating_sub(chart_overhead) as usize,
        model.ui_state.chart_window().label(),
        color_mode,
    ));
    frame.render_widget(
        Paragraph::new(chart_lines)
            .wrap(Wrap { trim: false })
            .block(panel_for(&title, WorkstationPane::Chart, model, color_mode))
            .style(Style::default().fg(text(color_mode))),
        area,
    );
}

fn show_chart_order_pressure(area: Rect) -> bool {
    area.width >= 96 && area.height >= 26
}

fn show_chart_prints_strip(area: Rect, model: &RatatuiFrameModel, symbol: &str) -> bool {
    area.width >= 96
        && area.height >= 30
        && model.trades.iter().any(|trade| trade.hl_coin == symbol)
}

fn show_chart_crosshair_context(area: Rect) -> bool {
    area.width >= 96 && area.height >= 30
}

fn chart_window_tabs_line(
    active: WorkstationChartWindow,
    color_mode: RatatuiColorMode,
    compact: bool,
) -> Line<'static> {
    let mut spans = vec![Span::styled(
        if compact { "WIN " } else { "WINDOWS " },
        Style::default()
            .fg(accent(color_mode))
            .add_modifier(Modifier::BOLD),
    )];
    for (index, window) in WorkstationChartWindow::ALL.iter().enumerate() {
        if index > 0 {
            spans.push(Span::raw(" "));
        }
        let label = window.label();
        let label = if compact {
            label.trim_end_matches('m')
        } else {
            label
        };
        if *window == active {
            spans.push(Span::styled(
                format!("[{label}]"),
                Style::default()
                    .fg(warn(color_mode))
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            spans.push(Span::raw(label.to_owned()));
        }
    }
    spans.push(Span::styled(
        if compact {
            "  t:window"
        } else {
            "  t:cycle window"
        },
        Style::default().fg(text(color_mode)),
    ));
    Line::from(spans)
}

fn selected_pair_edge_hud_lines(
    row: &FeatureSnapshot,
    color_mode: RatatuiColorMode,
) -> Vec<Line<'static>> {
    let flow = row.signed_notional_flow_30s.unwrap_or(0.0);
    let regime = chart_regime_label(row);
    let spread_gate = spread_gate_label(row.spread_bps);
    vec![
        Line::from(vec![
            Span::styled(
                "EDGE HUD ",
                Style::default()
                    .fg(accent(color_mode))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!("trade {} ", row.tradeability_state.as_str())),
            Span::styled(
                format!("conf {} ", row.confidence.score),
                Style::default().fg(if row.confidence.score >= 85 {
                    success(color_mode)
                } else if row.confidence.score >= 70 {
                    warn(color_mode)
                } else {
                    danger(color_mode)
                }),
            ),
            Span::raw(format!("spr {}bps ", format_optional(row.spread_bps, 1))),
            Span::raw(format!("risk {} ", row.resilience_state.as_str())),
            Span::styled("REGIME ", Style::default().fg(warn(color_mode))),
            Span::raw(regime),
        ]),
        Line::from(vec![
            Span::styled(
                "LIQ MICRO ",
                Style::default()
                    .fg(success(color_mode))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("flow {}  ", format_usd_signed(row.signed_notional_flow_30s)),
                Style::default().fg(flow_color(flow, color_mode)),
            ),
            Span::raw(format!("depth {}  ", format_usd(row.tob_depth_usd))),
            Span::styled(
                format!("imb {}  ", format_signed(row.tob_imbalance, "")),
                Style::default().fg(flow_color(row.tob_imbalance.unwrap_or(0.0), color_mode)),
            ),
            Span::raw(format!("score {:.0}", row.liquidity_score)),
        ]),
        Line::from(vec![
            Span::styled(
                "GATE ",
                Style::default()
                    .fg(warn(color_mode))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!(
                "spread gate {spread_gate} | no execution | public bbo proxy"
            )),
        ]),
    ]
}

fn selected_pair_order_pressure_lines(
    row: &FeatureSnapshot,
    color_mode: RatatuiColorMode,
) -> Vec<Line<'static>> {
    let bid_notional = notional(row.bid_px, row.bid_sz);
    let ask_notional = notional(row.ask_px, row.ask_sz);
    let (bid_share, ask_share) = quote_share(bid_notional, ask_notional).unwrap_or((0.0, 0.0));
    let skew = bid_share - ask_share;
    let bid_wall = depth_bar(bid_share, 12);
    let ask_wall = depth_bar(ask_share, 12);
    let skew_label = if skew > 0.08 {
        "bid-heavy"
    } else if skew < -0.08 {
        "ask-heavy"
    } else {
        "balanced"
    };

    vec![
        Line::from(vec![
            Span::styled(
                "ORDER PRESSURE ",
                Style::default()
                    .fg(accent(color_mode))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("selected pair "),
            Span::styled("BID ", Style::default().fg(success(color_mode))),
            Span::raw(format!(
                "{} {} | ",
                percent_label(bid_share),
                format_usd(bid_notional)
            )),
            Span::styled("ASK ", Style::default().fg(danger(color_mode))),
            Span::raw(format!(
                "{} {} | read-only top-book lens",
                percent_label(ask_share),
                format_usd(ask_notional)
            )),
        ]),
        Line::from(vec![
            Span::styled("bid wall ", Style::default().fg(success(color_mode))),
            Span::raw(bid_wall),
            Span::raw("  "),
            Span::styled("ask wall ", Style::default().fg(danger(color_mode))),
            Span::raw(ask_wall),
        ]),
        Line::from(vec![
            Span::styled(
                "book skew ",
                Style::default().fg(flow_color(skew, color_mode)),
            ),
            Span::raw(signed_meter(skew)),
            Span::raw(format!(
                " {skew_label} | tob imbalance {} | ofi {}",
                format_signed(row.tob_imbalance, ""),
                format_usd_signed(row.bbo_ofi_proxy_30s)
            )),
        ]),
    ]
}

fn chart_prints_strip_lines(
    trades: &[&TradeEvent],
    color_mode: RatatuiColorMode,
) -> Vec<Line<'static>> {
    if trades.is_empty() {
        return Vec::new();
    }
    let buy_count = trades
        .iter()
        .filter(|trade| trade.side == TradeSide::Buy)
        .count();
    let sell_count = trades
        .iter()
        .filter(|trade| trade.side == TradeSide::Sell)
        .count();
    let net_notional = trades
        .iter()
        .map(|trade| match trade.side {
            TradeSide::Buy => trade.notional,
            TradeSide::Sell => -trade.notional,
        })
        .sum::<f64>();
    let latest = trades[0];
    let recent = trades
        .iter()
        .take(3)
        .map(|trade| {
            format!(
                "{} {} {}",
                trade_side_label(trade.side),
                format_plain_number(trade.price),
                format_usd(Some(trade.notional))
            )
        })
        .collect::<Vec<_>>()
        .join(" | ");

    vec![
        Line::from(vec![
            Span::styled(
                "PRINTS STRIP ",
                Style::default()
                    .fg(accent(color_mode))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!("public time-and-sales | prints {} ", trades.len())),
            Span::styled("buy ", Style::default().fg(success(color_mode))),
            Span::raw(format!("{buy_count} ")),
            Span::styled("sell ", Style::default().fg(danger(color_mode))),
            Span::raw(format!(
                "{sell_count} net {}",
                format_usd_signed(Some(net_notional))
            )),
        ]),
        Line::from(vec![
            Span::raw("last "),
            Span::styled(
                trade_side_label(latest.side),
                Style::default()
                    .fg(trade_side_color(latest.side, color_mode))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!(
                " px {} size {} notional {} | no fills",
                format_plain_number(latest.price),
                format_size(Some(latest.size)),
                format_usd(Some(latest.notional))
            )),
        ]),
        Line::from(format!("recent {recent} | public trades only")),
    ]
}

fn chart_session_strip_lines(
    row: &FeatureSnapshot,
    color_mode: RatatuiColorMode,
) -> Vec<Line<'static>> {
    vec![
        Line::from(vec![
            Span::styled(
                "SESSION STRIP ",
                Style::default()
                    .fg(accent(color_mode))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!("RET 1m {} | ", trend_label(row.ret_1m))),
            Span::raw(format!(
                "RV 1m/5m/1h {}/{}/{} | ",
                format_optional(row.rv_1m, 2),
                format_optional(row.rv_5m, 2),
                format_optional(row.rv_1h, 2)
            )),
            Span::styled(
                format!("OFI {}", format_usd_signed(row.bbo_ofi_proxy_30s)),
                Style::default().fg(flow_color(
                    row.bbo_ofi_proxy_30s.unwrap_or_default(),
                    color_mode,
                )),
            ),
        ]),
        Line::from(vec![
            Span::raw(format!(
                "context adverse {} | spread {}bps | age {} | ",
                row.adverse_selection_proxy.as_str(),
                format_optional(row.spread_bps, 1),
                format_duration_ms(row.updated_ms_ago)
            )),
            Span::styled(
                "public signal context",
                Style::default().fg(warn(color_mode)),
            ),
        ]),
    ]
}

fn chart_regime_label(row: &FeatureSnapshot) -> String {
    let ret_pct = row.ret_1m.map(|value| value * 100.0);
    let trend = ret_pct.unwrap_or_default();
    if trend >= 0.10 {
        format!("MOMENTUM {}", format_signed(ret_pct, "%"))
    } else if trend <= -0.10 {
        format!("DOWNTREND {}", format_signed(ret_pct, "%"))
    } else if row.mean_reversion_score > row.momentum_score {
        format!("MEAN REVERSION {}", format_signed(ret_pct, "%"))
    } else {
        format!("BALANCED {}", format_signed(ret_pct, "%"))
    }
}

fn spread_gate_label(spread_bps: Option<f64>) -> &'static str {
    match spread_bps {
        Some(value) if value.is_finite() && value <= 5.0 => "tight",
        Some(value) if value.is_finite() && value <= 25.0 => "workable",
        Some(value) if value.is_finite() => "wide",
        _ => "unknown",
    }
}

fn chart_move_summary_line(
    candles: &[&CandleEvent],
    color_mode: RatatuiColorMode,
) -> Line<'static> {
    let first = candles[0];
    let latest = candles.last().copied().unwrap_or(first);
    let high = candles
        .iter()
        .map(|candle| candle.high)
        .fold(f64::NEG_INFINITY, f64::max);
    let low = candles
        .iter()
        .map(|candle| candle.low)
        .fold(f64::INFINITY, f64::min);
    let move_abs = latest.close - first.open;
    let range_pct = if first.open.abs() < f64::EPSILON {
        0.0
    } else {
        ((high - low).abs() / first.open.abs()) * 100.0
    };
    let summary_style = if move_abs < 0.0 {
        Style::default().fg(danger(color_mode))
    } else {
        Style::default().fg(success(color_mode))
    };
    Line::from(vec![
        Span::styled("MOVE ", Style::default().fg(accent(color_mode))),
        Span::styled(format!("{move_abs:+.4}"), summary_style),
        Span::raw("  RANGE "),
        Span::styled(
            format!("{range_pct:.2}%"),
            Style::default().fg(warn(color_mode)),
        ),
        Span::raw(format!(
            "  LAST {}  VOL {}",
            format_plain_number(latest.close),
            format_volume(latest.volume_base)
        )),
    ])
}

fn chart_candle_hud_line(candle: &CandleEvent, color_mode: RatatuiColorMode) -> Line<'static> {
    let body = candle.close - candle.open;
    let range = candle.high - candle.low;
    let range_pct = if candle.open.abs() < f64::EPSILON {
        0.0
    } else {
        (range.abs() / candle.open.abs()) * 100.0
    };
    let direction = if body >= 0.0 { "UP" } else { "DOWN" };
    let direction_style = if body >= 0.0 {
        Style::default().fg(success(color_mode))
    } else {
        Style::default().fg(danger(color_mode))
    };
    Line::from(vec![
        Span::styled(
            "CANDLE HUD ",
            Style::default()
                .fg(accent(color_mode))
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("latest "),
        Span::styled(direction, direction_style),
        Span::raw(" | body "),
        Span::styled(format!("{body:+.4}"), direction_style),
        Span::raw(format!(
            " | range {range_pct:.2}% | vol {} | trades {} | public OHLCV",
            format_volume(candle.volume_base),
            candle.trade_count
        )),
    ])
}

fn chart_crosshair_context_lines(
    row: &FeatureSnapshot,
    candles: &[&CandleEvent],
    color_mode: RatatuiColorMode,
) -> Vec<Line<'static>> {
    let Some(latest) = candles.last().copied() else {
        return Vec::new();
    };
    let high = candles
        .iter()
        .map(|candle| candle.high)
        .fold(f64::NEG_INFINITY, f64::max);
    let low = candles
        .iter()
        .map(|candle| candle.low)
        .fold(f64::INFINITY, f64::min);
    let range_pos = if (high - low).abs() < f64::EPSILON {
        0.5
    } else {
        ((latest.close - low) / (high - low)).clamp(0.0, 1.0)
    };

    vec![
        Line::from(vec![
            Span::styled(
                "CROSSHAIR ",
                Style::default()
                    .fg(accent(color_mode))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!("selected {} | ", display_symbol(row))),
            Span::raw(format!("last {} | ", format_plain_number(latest.close))),
            Span::styled(
                format!(
                    "range pos {} {}",
                    percent_label(range_pos),
                    depth_bar(range_pos, 10)
                ),
                Style::default().fg(warn(color_mode)),
            ),
        ]),
        Line::from(vec![
            Span::raw(format!(
                "session high {} | session low {} | spread {}bps | ",
                format_plain_number(high),
                format_plain_number(low),
                format_optional(row.spread_bps, 1)
            )),
            Span::styled(
                format!("momentum {} | ", trend_label(row.ret_1m)),
                market_row_style(row, color_mode),
            ),
            Span::styled(
                format!("confidence {}", row.confidence.score),
                Style::default().fg(confidence_color(row.confidence.score, color_mode)),
            ),
            Span::raw(" | read-only chart lens"),
        ]),
    ]
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

fn candle_chart_lines(
    candles: &[&CandleEvent],
    content_height: usize,
    window_label: &'static str,
    color_mode: RatatuiColorMode,
) -> Vec<Line<'static>> {
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
    let height = content_height.saturating_sub(3).clamp(4, 18);
    let mut lines = Vec::with_capacity(height + 3);

    for row in 0..height {
        let level = price_level(high, low, row, height);
        let mut spans = vec![Span::raw(format!("{} ┤", price_axis_label(level)))];
        for candle in candles {
            let glyph = candle_glyph(candle, level);
            spans.push(Span::styled(
                glyph.to_string(),
                candle_glyph_style(candle, glyph, color_mode),
            ));
        }
        lines.push(Line::from(spans));
    }

    lines.push(Line::from(format!(
        "px axis {} - {}   candles {} window {}",
        format_plain_number(low),
        format_plain_number(high),
        candles.len(),
        window_label
    )));
    lines.push(Line::from(format!(
        "OHLC {} / {} / {} / {}",
        format_plain_number(candles[0].open),
        format_plain_number(high),
        format_plain_number(low),
        format_plain_number(
            candles
                .last()
                .map_or(candles[0].close, |candle| candle.close)
        )
    )));
    lines.push(Line::from(format!(
        "{} | Public 1m candles only",
        volume_bar(candles)
    )));
    lines
}

fn price_axis_label(value: f64) -> String {
    let label = if value.abs() >= 1_000.0 {
        format!("{value:.0}")
    } else if value.abs() >= 10.0 {
        format!("{value:.2}")
    } else {
        format!("{value:.4}")
    };
    format!("{label:>8}")
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

fn candle_glyph_style(candle: &CandleEvent, glyph: char, color_mode: RatatuiColorMode) -> Style {
    match glyph {
        '█' => Style::default().fg(success(color_mode)),
        '▓' => Style::default().fg(danger(color_mode)),
        '│' if candle.close >= candle.open => Style::default().fg(success(color_mode)),
        '│' => Style::default().fg(danger(color_mode)),
        _ => Style::default().fg(text(color_mode)),
    }
}

fn volume_bar(candles: &[&CandleEvent]) -> String {
    let max_volume = candles
        .iter()
        .map(|candle| candle.volume_base)
        .fold(0.0_f64, f64::max);
    if max_volume <= 0.0 {
        return "VOL LANE -".to_owned();
    }
    let bars = candles
        .iter()
        .map(|candle| volume_glyph(candle.volume_base / max_volume))
        .collect::<String>();
    let latest_volume = candles.last().map_or(0.0, |candle| candle.volume_base);
    format!(
        "VOL LANE {bars} max {} last {}",
        format_volume(max_volume),
        format_volume(latest_volume)
    )
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
    let content_height = area.height.saturating_sub(2) as usize;
    let content_width = area.width.saturating_sub(2);
    let lines = selected_row(&rows, model).map_or_else(
        || vec![Line::from("No book data")],
        |row| {
            book_lines(
                row,
                color_mode,
                content_height,
                content_width,
                model.ui_state.view(),
            )
        },
    );
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: true })
            .block(panel_for("BOOK", WorkstationPane::Book, model, color_mode)),
        area,
    );
}

fn book_lines(
    row: &FeatureSnapshot,
    color_mode: RatatuiColorMode,
    content_height: usize,
    content_width: u16,
    view: WorkstationView,
) -> Vec<Line<'static>> {
    let bid_notional = notional(row.bid_px, row.bid_sz);
    let ask_notional = notional(row.ask_px, row.ask_sz);
    let quote_share = quote_share(bid_notional, ask_notional);
    let (bid_share, ask_share) = quote_share
        .map(|(bid, ask)| (percent_label(bid), percent_label(ask)))
        .unwrap_or_else(|| ("-".to_owned(), "-".to_owned()));
    let (bid_bar, ask_bar) = quote_share
        .map(|(bid, ask)| (depth_bar(bid, 16), depth_bar(ask, 16)))
        .unwrap_or_else(|| (depth_bar_empty(16), depth_bar_empty(16)));
    let compact_book = content_height <= 7;
    let share_prefix = if view == WorkstationView::Flow {
        "share bid "
    } else {
        "DEPTH CONSOLE share bid "
    };
    let book_snap = if compact_book {
        book_snap_compact_line(quote_share, color_mode)
    } else {
        Line::from(vec![
            Span::raw(share_prefix),
            Span::styled(
                bid_share,
                Style::default()
                    .fg(success(color_mode))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" / "),
            Span::styled(
                format!("ask {ask_share}"),
                Style::default()
                    .fg(danger(color_mode))
                    .add_modifier(Modifier::BOLD),
            ),
        ])
    };
    let mut lines = vec![
        Line::from(vec![
            Span::styled(
                "BID ",
                Style::default()
                    .fg(success(color_mode))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!(
                "{} x {} BBO depth proxy",
                format_price(row.bid_px),
                format_size(row.bid_sz)
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
                "{} x {} BOOK proxy only",
                format_price(row.ask_px),
                format_size(row.ask_sz)
            )),
        ]),
        book_snap,
    ];
    if !compact_book && content_width >= 64 {
        lines.insert(2, book_bbo_ladder_line(row, color_mode));
        lines.insert(3, book_microprice_line(row, quote_share, color_mode));
    }
    if !compact_book {
        lines.extend(book_snap_lines(
            row,
            quote_share,
            bid_notional,
            ask_notional,
            color_mode,
            false,
        ));
    }

    if view == WorkstationView::Flow {
        lines.extend([
            Line::from(vec![
                Span::styled(
                    "BOOK FLOW MODE ",
                    Style::default()
                        .fg(accent(color_mode))
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("Public top-book only"),
            ]),
            Line::from(format!(
                "depth skew {}  bid {} / ask {}",
                signed_meter(row.tob_imbalance.unwrap_or(0.0)),
                format_usd(bid_notional),
                format_usd(ask_notional)
            )),
            Line::from(format!(
                "spread gate {} bps  OFI {}",
                format_optional(row.spread_bps, 1),
                format_usd_signed(row.bbo_ofi_proxy_30s)
            )),
        ]);
        if content_height > 7 {
            lines.extend([
                Line::from(vec![
                    Span::styled("bid wall ", Style::default().fg(success(color_mode))),
                    Span::raw(format!("{bid_bar} {}", format_usd(bid_notional))),
                ]),
                Line::from(vec![
                    Span::styled("ask wall ", Style::default().fg(danger(color_mode))),
                    Span::raw(format!("{ask_bar} {}", format_usd(ask_notional))),
                ]),
                Line::from(format!(
                    "state {} / {}",
                    row.tradeability_state.as_str(),
                    row.resilience_state.as_str()
                )),
            ]);
        }
        return lines;
    }

    if compact_book {
        let (bid_bar, ask_bar) = quote_share
            .map(|(bid, ask)| (depth_bar(bid, 8), depth_bar(ask, 8)))
            .unwrap_or_else(|| (depth_bar_empty(8), depth_bar_empty(8)));
        lines.extend([
            Line::from(vec![
                Span::styled("BID notional ", Style::default().fg(success(color_mode))),
                Span::raw(format!(
                    "bid pressure {bid_bar} {}",
                    format_usd(bid_notional)
                )),
            ]),
            Line::from(vec![
                Span::styled("ASK notional ", Style::default().fg(danger(color_mode))),
                Span::raw(format!(
                    "ask pressure {ask_bar} {}",
                    format_usd(ask_notional)
                )),
            ]),
            Line::from(format!(
                "queue map imbalance {}  OFI {}",
                format_signed(row.tob_imbalance, ""),
                format_usd_signed(row.bbo_ofi_proxy_30s)
            )),
            Line::from("BBO depth proxy | BOOK proxy only | read-only top-book"),
        ]);
        return lines;
    }

    lines.extend([
        Line::from("DEPTH CONSOLE | BBO depth proxy | BOOK proxy only | public top-book"),
        Line::from(vec![
            Span::styled("BID notional ", Style::default().fg(success(color_mode))),
            Span::raw(format!(
                "bid pressure {bid_bar} {}",
                format_usd(bid_notional)
            )),
        ]),
        Line::from(vec![
            Span::styled("ASK notional ", Style::default().fg(danger(color_mode))),
            Span::raw(format!(
                "ask pressure {ask_bar} {}",
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
        Line::from(format!("adverse {}", row.adverse_selection_proxy.as_str())),
    ]);
    lines
}

fn book_snap_compact_line(
    quote_share: Option<(f64, f64)>,
    color_mode: RatatuiColorMode,
) -> Line<'static> {
    let (bid_share, ask_share) = quote_share
        .map(|(bid, ask)| (percent_label(bid), percent_label(ask)))
        .unwrap_or_else(|| ("-".to_owned(), "-".to_owned()));
    Line::from(vec![
        Span::styled(
            "BOOK SNAP DEPTH CONSOLE queue map ",
            Style::default()
                .fg(accent(color_mode))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("share bid ", Style::default().fg(success(color_mode))),
        Span::raw(format!("{bid_share} / ")),
        Span::styled("ask share ", Style::default().fg(danger(color_mode))),
        Span::raw(ask_share),
    ])
}

fn book_bbo_ladder_line(row: &FeatureSnapshot, color_mode: RatatuiColorMode) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            "BBO LADDER ",
            Style::default()
                .fg(accent(color_mode))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("bid {} ", format_price(row.bid_px)),
            Style::default().fg(success(color_mode)),
        ),
        Span::raw(format!("| mid {} | ", format_price(mid_price(row)))),
        Span::styled(
            format!("ask {} ", format_price(row.ask_px)),
            Style::default().fg(danger(color_mode)),
        ),
        Span::raw(format!(
            "| spr {}bps | read-only BBO",
            format_optional(row.spread_bps, 1)
        )),
    ])
}

fn book_microprice_line(
    row: &FeatureSnapshot,
    quote_share: Option<(f64, f64)>,
    color_mode: RatatuiColorMode,
) -> Line<'static> {
    let microprice = microprice(row);
    let edge_bps = match (microprice, mid_price(row)) {
        (Some(micro), Some(mid)) if mid.abs() > f64::EPSILON => {
            Some(((micro - mid) / mid) * 10_000.0)
        }
        _ => None,
    };
    let skew = quote_share.map_or(0.0, |(bid, ask)| bid - ask);
    Line::from(vec![
        Span::styled(
            "MICROPRICE ",
            Style::default()
                .fg(accent(color_mode))
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("read-only top-book model | "),
        Span::raw(format!("queue skew {} | ", signed_meter(skew))),
        Span::raw(format!("px {} ", format_price(microprice))),
        Span::styled(
            format!("edge {}bps ", format_signed(edge_bps, "")),
            Style::default().fg(flow_color(edge_bps.unwrap_or_default(), color_mode)),
        ),
    ])
}

fn microprice(row: &FeatureSnapshot) -> Option<f64> {
    match (row.bid_px, row.ask_px, row.bid_sz, row.ask_sz) {
        (Some(bid_px), Some(ask_px), Some(bid_sz), Some(ask_sz))
            if bid_px.is_finite()
                && ask_px.is_finite()
                && bid_sz.is_finite()
                && ask_sz.is_finite()
                && bid_sz > 0.0
                && ask_sz > 0.0 =>
        {
            Some(((ask_px * bid_sz) + (bid_px * ask_sz)) / (bid_sz + ask_sz))
        }
        _ => None,
    }
}

fn book_snap_lines(
    row: &FeatureSnapshot,
    quote_share: Option<(f64, f64)>,
    bid_notional: Option<f64>,
    ask_notional: Option<f64>,
    color_mode: RatatuiColorMode,
    compact: bool,
) -> Vec<Line<'static>> {
    let (bid_share, ask_share) = quote_share
        .map(|(bid, ask)| (percent_label(bid), percent_label(ask)))
        .unwrap_or_else(|| ("-".to_owned(), "-".to_owned()));
    let (bid_bar, ask_bar) = quote_share
        .map(|(bid, ask)| {
            let width = if compact { 6 } else { 8 };
            (depth_bar(bid, width), depth_bar(ask, width))
        })
        .unwrap_or_else(|| {
            let width = if compact { 6 } else { 8 };
            (depth_bar_empty(width), depth_bar_empty(width))
        });
    vec![
        Line::from(vec![
            Span::styled(
                "BOOK SNAP ",
                Style::default()
                    .fg(accent(color_mode))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("bid share ", Style::default().fg(success(color_mode))),
            Span::raw(format!("{bid_share} {bid_bar} ")),
            Span::styled("ask share ", Style::default().fg(danger(color_mode))),
            Span::raw(format!("{ask_share} {ask_bar}")),
        ]),
        Line::from(format!(
            "queue map read-only top-book {} / {} spr {} bps",
            format_usd(bid_notional),
            format_usd(ask_notional),
            format_optional(row.spread_bps, 1)
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
    let content_height = area.height.saturating_sub(2) as usize;
    let content_width = area.width.saturating_sub(2) as usize;
    let lines = tape_lines(&rows, model, content_height, content_width, color_mode);
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: true })
            .block(panel_for("TAPE", WorkstationPane::Tape, model, color_mode)),
        area,
    );
}

fn tape_lines(
    rows: &[FeatureSnapshot],
    model: &RatatuiFrameModel,
    content_height: usize,
    content_width: usize,
    color_mode: RatatuiColorMode,
) -> Vec<Line<'static>> {
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

    let max_abs_flow = leaders
        .iter()
        .filter_map(|row| row.signed_notional_flow_30s)
        .filter(|value| value.is_finite())
        .map(f64::abs)
        .fold(0.0_f64, f64::max);
    let net_flow = rows
        .iter()
        .filter_map(|row| row.signed_notional_flow_30s)
        .filter(|value| value.is_finite())
        .sum::<f64>();
    let signed_abs_total = rows
        .iter()
        .filter_map(|row| row.signed_notional_flow_30s)
        .filter(|value| value.is_finite())
        .map(f64::abs)
        .sum::<f64>();
    let pressure_scale = max_abs_flow.max(signed_abs_total);
    let compact = content_width < 42 || content_height <= 7;
    let pulse_width = if compact { 8 } else { 18 };

    let mut lines = vec![
        Line::from(format!("Selected flow {}", display_symbol(selected))),
        Line::from(vec![
            Span::styled(
                "FLOW pulse ",
                Style::default()
                    .fg(accent(color_mode))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(signed_flow_bar(
                selected.signed_notional_flow_30s.unwrap_or(0.0),
                max_abs_flow,
                pulse_width,
            )),
            Span::raw(format!(
                " {}",
                format_usd_signed(selected.signed_notional_flow_30s)
            )),
        ]),
        Line::from(vec![
            Span::styled(
                "net pressure ",
                Style::default()
                    .fg(flow_color(net_flow, color_mode))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(signed_flow_bar(net_flow, pressure_scale, pulse_width)),
            Span::raw(format!(" {}", format_usd_signed(Some(net_flow)))),
        ]),
        Line::from(format!(
            "flow30 {} | OFI {}",
            format_usd_signed(selected.signed_notional_flow_30s),
            format_usd_signed(selected.bbo_ofi_proxy_30s)
        )),
    ];
    if !compact {
        lines.push(Line::from(format!(
            "ret1m {} | rv1m {} | spread {} bps",
            format_signed(selected.ret_1m.map(|value| value * 100.0), "%"),
            format_optional(selected.rv_1m, 2),
            format_optional(selected.spread_bps, 1)
        )));
    }

    let recent_trades = selected_trades(model, &selected.symbol, content_height);
    if !recent_trades.is_empty() {
        lines.push(tape_radar_line(&recent_trades, color_mode));
        if let Some(latest_trade) = recent_trades.first() {
            lines.push(last_trade_hud_line(latest_trade, color_mode));
        }
        if model.ui_state.view() == WorkstationView::Flow {
            lines.extend(trade_pressure_lines(&recent_trades, compact, color_mode));
        }
        lines.push(Line::from(vec![
            Span::styled(
                "PUBLIC TRADES ",
                Style::default()
                    .fg(accent(color_mode))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("time side price notional"),
        ]));

        let reserved = lines.len() + 1;
        let available_trades = content_height.saturating_sub(reserved).max(1);
        let limit = if compact {
            available_trades.min(3)
        } else {
            available_trades.min(8)
        };
        lines.extend(
            recent_trades
                .into_iter()
                .take(limit)
                .map(|trade| trade_tape_line(trade, compact, color_mode)),
        );
        lines.push(Line::from(
            "Public trades only | no fills, no private streams.",
        ));
        return lines;
    }

    lines.push(Line::from("Flow leaderboard"));

    if compact {
        let reserved = lines.len() + 1;
        let available_leaders = content_height.saturating_sub(reserved);
        let limit = model
            .ui_state
            .visible_row_limit()
            .min(3)
            .min(available_leaders);
        lines.extend(
            leaders
                .into_iter()
                .take(limit)
                .map(compact_tape_leader_line),
        );
        lines.push(Line::from("Tape proxy only | public flow"));
        return lines;
    }

    let reserved = lines.len() + 1;
    let available_leaders = content_height.saturating_sub(reserved).max(2);
    let limit = model
        .ui_state
        .visible_row_limit()
        .min(10)
        .min(available_leaders);
    lines.extend(
        leaders
            .into_iter()
            .take(limit)
            .map(|row| tape_leader_line(row, max_abs_flow, color_mode)),
    );
    lines.push(Line::from("Tape proxy only | public BBO/flow; no fills."));
    lines
}

fn last_trade_hud_line(trade: &TradeEvent, color_mode: RatatuiColorMode) -> Line<'static> {
    let side = trade_side_label(trade.side);
    let mut spans = vec![
        Span::styled(
            "LAST TRADE HUD ",
            Style::default()
                .fg(accent(color_mode))
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("latest "),
        Span::styled(
            side,
            Style::default()
                .fg(trade_side_color(trade.side, color_mode))
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!(
            " tid {} px {} size {} notional {}",
            trade.tid,
            format_plain_number(trade.price),
            format_size(Some(trade.size)),
            format_usd(Some(trade.notional))
        )),
    ];
    spans.push(Span::raw(" | public trades only"));
    Line::from(spans)
}

fn tape_radar_line(trades: &[&TradeEvent], color_mode: RatatuiColorMode) -> Line<'static> {
    let buy_count = trades
        .iter()
        .filter(|trade| trade.side == TradeSide::Buy)
        .count();
    let sell_count = trades
        .iter()
        .filter(|trade| trade.side == TradeSide::Sell)
        .count();
    let net_notional = trades
        .iter()
        .map(|trade| match trade.side {
            TradeSide::Buy => trade.notional,
            TradeSide::Sell => -trade.notional,
        })
        .sum::<f64>();
    Line::from(vec![
        Span::styled(
            "TAPE RADAR ",
            Style::default()
                .fg(accent(color_mode))
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!("prints {} ", trades.len())),
        Span::styled("buy ", Style::default().fg(success(color_mode))),
        Span::raw(format!("{buy_count}  ")),
        Span::styled("sell ", Style::default().fg(danger(color_mode))),
        Span::raw(format!(
            "{sell_count} net {} public tape",
            format_usd_signed(Some(net_notional))
        )),
    ])
}

fn selected_trades<'a>(
    model: &'a RatatuiFrameModel,
    symbol: &str,
    content_height: usize,
) -> Vec<&'a TradeEvent> {
    let limit = content_height.clamp(1, 12);
    let mut trades = model
        .trades
        .iter()
        .filter(|trade| trade.hl_coin == symbol)
        .collect::<Vec<_>>();
    trades.sort_by(|left, right| {
        right
            .exchange_ts_ms
            .cmp(&left.exchange_ts_ms)
            .then_with(|| right.tid.cmp(&left.tid))
    });
    if trades.len() > limit {
        trades.truncate(limit);
    }
    trades
}

fn trade_pressure_lines(
    trades: &[&TradeEvent],
    compact: bool,
    color_mode: RatatuiColorMode,
) -> Vec<Line<'static>> {
    let buy_notional = trades
        .iter()
        .filter(|trade| trade.side == TradeSide::Buy)
        .map(|trade| trade.notional)
        .sum::<f64>();
    let sell_notional = trades
        .iter()
        .filter(|trade| trade.side == TradeSide::Sell)
        .map(|trade| trade.notional)
        .sum::<f64>();
    let buy_count = trades
        .iter()
        .filter(|trade| trade.side == TradeSide::Buy)
        .count();
    let sell_count = trades
        .iter()
        .filter(|trade| trade.side == TradeSide::Sell)
        .count();
    let total_notional = buy_notional + sell_notional;
    let buy_share = if total_notional > 0.0 {
        buy_notional / total_notional
    } else {
        0.0
    };
    let sell_share = if total_notional > 0.0 {
        sell_notional / total_notional
    } else {
        0.0
    };
    let pressure = if total_notional > 0.0 {
        (buy_notional - sell_notional) / total_notional
    } else {
        0.0
    };
    let bar_width = if compact { 8 } else { 10 };

    let mut lines = vec![
        Line::from(vec![
            Span::styled(
                "TRADE FLOW MODE ",
                Style::default()
                    .fg(accent(color_mode))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("Public trades only"),
        ]),
        Line::from(vec![
            Span::styled("buy pressure ", Style::default().fg(success(color_mode))),
            Span::raw(format!(
                "{} {} / {} prints",
                depth_bar(buy_share, bar_width),
                format_usd(Some(buy_notional)),
                buy_count
            )),
        ]),
    ];

    if compact {
        lines.push(Line::from(format!(
            "sell pressure {} {} / {} prints",
            depth_bar(sell_share, bar_width),
            format_usd(Some(sell_notional)),
            sell_count
        )));
    } else {
        lines.extend([
            Line::from(vec![
                Span::styled("sell pressure ", Style::default().fg(danger(color_mode))),
                Span::raw(format!(
                    "{} {} / {} prints",
                    depth_bar(sell_share, bar_width),
                    format_usd(Some(sell_notional)),
                    sell_count
                )),
            ]),
            Line::from(format!(
                "trade skew {} net {}",
                signed_meter(pressure),
                format_usd_signed(Some(buy_notional - sell_notional))
            )),
        ]);
    }

    lines
}

fn trade_tape_line(
    trade: &TradeEvent,
    compact: bool,
    color_mode: RatatuiColorMode,
) -> Line<'static> {
    let side = trade_side_label(trade.side);
    let time = trade_time_label(trade.exchange_ts_ms);
    if compact {
        return Line::from(vec![
            Span::raw(format!("{time} ")),
            Span::styled(
                format!("{side:<4}"),
                Style::default()
                    .fg(trade_side_color(trade.side, color_mode))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!(
                " {} {}",
                format_plain_number(trade.price),
                format_usd(Some(trade.notional))
            )),
        ]);
    }

    Line::from(vec![
        Span::raw(format!("{time} ")),
        Span::styled(
            format!("{side:<4}"),
            Style::default()
                .fg(trade_side_color(trade.side, color_mode))
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!(
            " px {} size {} notional {}",
            format_plain_number(trade.price),
            format_size(Some(trade.size)),
            format_usd(Some(trade.notional))
        )),
    ])
}

fn trade_side_label(side: TradeSide) -> &'static str {
    match side {
        TradeSide::Buy => "BUY",
        TradeSide::Sell => "SELL",
    }
}

fn trade_time_label(exchange_ts_ms: i64) -> String {
    if exchange_ts_ms <= 0 {
        return "--:--".to_owned();
    }
    let total_secs = exchange_ts_ms.div_euclid(1_000);
    let minute = total_secs.div_euclid(60).rem_euclid(60);
    let second = total_secs.rem_euclid(60);
    format!("{minute:02}:{second:02}")
}

fn trade_side_color(side: TradeSide, color_mode: RatatuiColorMode) -> Color {
    match side {
        TradeSide::Buy => success(color_mode),
        TradeSide::Sell => danger(color_mode),
    }
}

fn tape_leader_line(
    row: &FeatureSnapshot,
    max_abs_flow: f64,
    color_mode: RatatuiColorMode,
) -> Line<'static> {
    let flow = row.signed_notional_flow_30s.unwrap_or(0.0);
    Line::from(vec![
        Span::styled(
            display_symbol(row).to_owned(),
            Style::default()
                .fg(flow_color(flow, color_mode))
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!(
            " {} flow {} OFI {}",
            signed_flow_bar(flow, max_abs_flow, 12),
            format_usd_signed(row.signed_notional_flow_30s),
            format_usd_signed(row.bbo_ofi_proxy_30s)
        )),
    ])
}

fn compact_tape_leader_line(row: &FeatureSnapshot) -> Line<'static> {
    Line::from(format!(
        "{} {} OFI {}",
        display_symbol(row),
        format_usd_signed(row.signed_notional_flow_30s),
        format_usd_signed(row.bbo_ofi_proxy_30s)
    ))
}

fn signed_flow_bar(value: f64, max_abs: f64, width: usize) -> String {
    let half = (width / 2).max(1);
    let ratio = if max_abs.is_finite() && max_abs > 0.0 {
        (value.abs() / max_abs).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let filled = ((ratio * half as f64).round() as usize).min(half);
    if value < 0.0 {
        format!(
            "{}{}|{}",
            "░".repeat(half.saturating_sub(filled)),
            "█".repeat(filled),
            "░".repeat(half)
        )
    } else if value > 0.0 {
        format!(
            "{}|{}{}",
            "░".repeat(half),
            "█".repeat(filled),
            "░".repeat(half.saturating_sub(filled))
        )
    } else {
        format!("{}|{}", "░".repeat(half), "░".repeat(half))
    }
}

fn render_status_bar(
    frame: &mut Frame<'_>,
    area: Rect,
    model: &RatatuiFrameModel,
    color_mode: RatatuiColorMode,
) {
    let status = if area.width < 90 {
        compact_status_bar_line(model)
    } else {
        market_status_bar_line(model, color_mode)
    };
    frame.render_widget(
        Paragraph::new(status)
            .wrap(Wrap { trim: true })
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

fn compact_status_bar_line(model: &RatatuiFrameModel) -> Line<'static> {
    if model.ui_state.focused_pane() == WorkstationPane::Watchlist {
        return Line::from(format!(
            "{} | {} | {} | {} | {} | RO no-wallet",
            compact_health_label(&model.health_status),
            display_state_label(model),
            focused_pane_key_label(model.ui_state.focused_pane(), true),
            compact_mode_label(&model.request, model.rows.len()),
            operational_quality_label(model, true)
        ));
    }

    Line::from(format!(
        "{} | {} | {} | ACTION {} | RO no-wallet",
        display_state_label(model),
        focused_pane_key_label(model.ui_state.focused_pane(), true),
        compact_mode_label(&model.request, model.rows.len()),
        focused_pane_action_label(model.ui_state.focused_pane(), true),
    ))
}

fn market_status_bar_line(
    model: &RatatuiFrameModel,
    color_mode: RatatuiColorMode,
) -> Line<'static> {
    let mut spans = vec![
        Span::raw(format!(
            " {} | {} | No wallet | ",
            compact_health_label(&model.health_status),
            pause_label(model),
        )),
        Span::styled(
            "MARKET TICKER ",
            Style::default()
                .fg(accent(color_mode))
                .add_modifier(Modifier::BOLD),
        ),
    ];
    spans.extend(market_ticker_spans(model, color_mode));
    spans.extend([
        Span::raw(" | "),
        Span::styled(
            format!("{} | ", operational_quality_label(model, false)),
            Style::default()
                .fg(warn(color_mode))
                .add_modifier(Modifier::BOLD),
        ),
    ]);
    spans.extend(risk_strip_spans(model, color_mode));
    spans.extend([Span::raw(" | ACTION STRIP | ")]);
    Line::from(spans)
}

fn risk_strip_spans(model: &RatatuiFrameModel, color_mode: RatatuiColorMode) -> Vec<Span<'static>> {
    let rows = screened_rows(model);
    if rows.is_empty() {
        return vec![
            Span::raw(" | "),
            Span::styled(
                "RISK STRIP ",
                Style::default()
                    .fg(warn(color_mode))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("no rows"),
        ];
    }

    let degraded = rows
        .iter()
        .filter(|row| row.confidence.score < 70 || row.staleness_state != StalenessState::Fresh)
        .count();
    let net_flow = rows
        .iter()
        .filter_map(|row| row.signed_notional_flow_30s)
        .filter(|value| value.is_finite())
        .sum::<f64>();
    let avg_confidence = rows
        .iter()
        .map(|row| row.confidence.score as u64)
        .sum::<u64>() as f64
        / rows.len() as f64;
    let pressure_style = Style::default()
        .fg(flow_color(net_flow, color_mode))
        .add_modifier(Modifier::BOLD);

    vec![
        Span::raw(" | "),
        Span::styled(
            "RISK STRIP ",
            Style::default()
                .fg(warn(color_mode))
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!(
            "conf{:.0} degraded{:02} ",
            avg_confidence,
            degraded.min(99)
        )),
        Span::styled(
            format!("net flow {}", format_usd_signed(Some(net_flow))),
            pressure_style,
        ),
    ]
}

fn focused_pane_key_label(pane: WorkstationPane, compact: bool) -> &'static str {
    match pane {
        WorkstationPane::Watchlist => {
            if compact {
                "watchlist:j/k"
            } else {
                "watchlist j/k rows"
            }
        }
        WorkstationPane::Detail => {
            if compact {
                "detail:tab"
            } else {
                "detail tab views"
            }
        }
        WorkstationPane::Chart => {
            if compact {
                "chart:t"
            } else {
                "chart t window"
            }
        }
        WorkstationPane::Book => {
            if compact {
                "book:tab"
            } else {
                "book tab flow"
            }
        }
        WorkstationPane::Tape => {
            if compact {
                "tape:tab"
            } else {
                "tape tab flow"
            }
        }
        WorkstationPane::Status => {
            if compact {
                "status:?"
            } else {
                "status ? help"
            }
        }
    }
}

fn focused_pane_action_label(pane: WorkstationPane, compact: bool) -> &'static str {
    match pane {
        WorkstationPane::Watchlist => {
            if compact {
                "watch:j/k rows / command"
            } else {
                "watchlist j/k rows | / command"
            }
        }
        WorkstationPane::Detail => {
            if compact {
                "detail:tab views / command"
            } else {
                "detail tab views | / command"
            }
        }
        WorkstationPane::Chart => {
            if compact {
                "chart:t window / command"
            } else {
                "chart t window | / command"
            }
        }
        WorkstationPane::Book => {
            if compact {
                "book:tab flow / command"
            } else {
                "book tab flow | / command"
            }
        }
        WorkstationPane::Tape => {
            if compact {
                "tape:tab flow / command"
            } else {
                "tape tab flow | / command"
            }
        }
        WorkstationPane::Status => {
            if compact {
                "status:? help / command"
            } else {
                "status ? help | / command"
            }
        }
    }
}

fn market_ticker_spans(
    model: &RatatuiFrameModel,
    color_mode: RatatuiColorMode,
) -> Vec<Span<'static>> {
    let rows = screened_rows(model);
    if rows.is_empty() {
        return vec![Span::raw("no rows")];
    }

    let up = rows
        .iter()
        .filter(|row| row.ret_1m.is_some_and(|value| value > 0.0))
        .count();
    let down = rows
        .iter()
        .filter(|row| row.ret_1m.is_some_and(|value| value < 0.0))
        .count();
    let up_leader = market_return_leader(&rows, true);
    let down_leader = market_return_leader(&rows, false);
    let flow_leader = rows
        .iter()
        .filter_map(|row| {
            row.signed_notional_flow_30s
                .filter(|value| value.is_finite())
                .map(|value| (row, value))
        })
        .max_by(|(_, left), (_, right)| {
            left.abs()
                .partial_cmp(&right.abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        });

    let mut spans = vec![
        Span::styled("BREADTH ", Style::default().fg(accent(color_mode))),
        Span::raw(format!("{:02}/{:02}", up.min(99), down.min(99))),
    ];
    if let Some((row, value)) = up_leader {
        spans.push(Span::raw(" | "));
        spans.push(Span::styled(
            format!("UP {} {:+.2}%", display_symbol(row), value * 100.0),
            Style::default()
                .fg(success(color_mode))
                .add_modifier(Modifier::BOLD),
        ));
    }
    if let Some((row, value)) = down_leader {
        spans.push(Span::raw(" | "));
        spans.push(Span::styled(
            format!("DOWN {} {:+.2}%", display_symbol(row), value * 100.0),
            Style::default()
                .fg(danger(color_mode))
                .add_modifier(Modifier::BOLD),
        ));
    }
    if let Some((row, _)) = flow_leader {
        spans.push(Span::raw(" | "));
        spans.push(Span::styled(
            format!(
                "FLOW {} {}",
                display_symbol(row),
                format_usd_signed(row.signed_notional_flow_30s)
            ),
            market_row_style(row, color_mode).add_modifier(Modifier::BOLD),
        ));
    }
    spans
}

fn market_return_leader(
    rows: &[FeatureSnapshot],
    positive: bool,
) -> Option<(&FeatureSnapshot, f64)> {
    rows.iter()
        .filter_map(|row| {
            row.ret_1m
                .filter(|value| value.is_finite())
                .filter(|value| (*value > 0.0) == positive)
                .map(|value| (row, value))
        })
        .max_by(|(_, left), (_, right)| {
            left.abs()
                .partial_cmp(&right.abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
}

fn operational_quality_label(model: &RatatuiFrameModel, compact: bool) -> String {
    let rows = screened_rows(model);
    let tradeable = rows
        .iter()
        .filter(|row| matches!(row.tradeability_state, TradeabilityState::Tradeable))
        .count();
    let degraded = rows
        .iter()
        .filter(|row| row.confidence.score < 70 || row.staleness_state != StalenessState::Fresh)
        .count();
    let stale = rows
        .iter()
        .filter(|row| row.staleness_state != StalenessState::Fresh)
        .count();
    if compact {
        format!("q:T{tradeable}")
    } else {
        format!("QUALITY T{tradeable:02} !{degraded:02} stale{stale:02}")
    }
}

fn compact_health_label(health_status: &str) -> String {
    health_status
        .replace("ws=", "ws")
        .replace("events=", "ev")
        .replace("reconnects=", "r")
        .replace("gaps=", "g")
}

fn display_state_label(model: &RatatuiFrameModel) -> &'static str {
    if model.ui_state.paused() {
        "paused"
    } else {
        "live"
    }
}

fn render_status_panel(
    frame: &mut Frame<'_>,
    area: Rect,
    model: &RatatuiFrameModel,
    color_mode: RatatuiColorMode,
) {
    let rows = screened_rows(model);
    let ws_messages = health_metric(&model.health_status, "ws").unwrap_or(0);
    let market_events = health_metric(&model.health_status, "events").unwrap_or(0);
    let reconnects = health_metric(&model.health_status, "reconnects").unwrap_or(0);
    let gaps = health_metric(&model.health_status, "gaps").unwrap_or(0);
    let ingest_ratio = (market_events as f64 / 500.0).clamp(0.0, 1.0);
    let ws_ratio = (ws_messages as f64 / 500.0).clamp(0.0, 1.0);
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
            Span::raw(format!(
                " terminal color {} --color always",
                color_mode.label()
            )),
        ]),
        Line::from(vec![
            Span::styled(
                "OPS RADAR ",
                Style::default()
                    .fg(accent(color_mode))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!(
                "health {} WS load {}",
                model.health_status,
                depth_bar(ws_ratio, 4)
            )),
        ]),
        Line::from(vec![
            Span::styled(
                "LIVE OPS ",
                Style::default()
                    .fg(accent(color_mode))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!(
                "ws {ws_messages} events {market_events} reconnects {reconnects} gaps {gaps} "
            )),
            Span::styled("EVENT flow ", Style::default().fg(success(color_mode))),
            Span::raw(depth_bar(ingest_ratio, 4)),
            Span::raw(" ingest"),
        ]),
        status_quality_matrix_line(&rows, color_mode),
        Line::from(format!(
            "active {} pane {} palette {} / filter t chart",
            mode_label(&model.request, rows.len()),
            model.ui_state.focused_pane().label(),
            color_mode.palette_label(),
        )),
        Line::from(vec![
            Span::styled(
                "OPS DECK SAFETY GATES ",
                Style::default()
                    .fg(accent(color_mode))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("no orders 1-6 focus p preset s sort space pause"),
        ]),
        Line::from("read-only safety | No wallet | public market data only"),
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

fn status_quality_matrix_line(
    rows: &[FeatureSnapshot],
    color_mode: RatatuiColorMode,
) -> Line<'static> {
    let total = rows.len();
    let tradeable = rows
        .iter()
        .filter(|row| matches!(row.tradeability_state, TradeabilityState::Tradeable))
        .count();
    let degraded = rows
        .iter()
        .filter(|row| row.confidence.score < 70 || row.staleness_state != StalenessState::Fresh)
        .count();
    let stale = rows
        .iter()
        .filter(|row| row.staleness_state != StalenessState::Fresh)
        .count();
    let confidence = if total == 0 {
        0
    } else {
        let sum = rows
            .iter()
            .map(|row| row.confidence.score as u64)
            .sum::<u64>();
        (sum / total as u64).min(100)
    };
    let tradeable_ratio = if total == 0 {
        0.0
    } else {
        tradeable as f64 / total as f64
    };
    let confidence_ratio = confidence as f64 / 100.0;

    Line::from(vec![
        Span::styled(
            "QUALITY MATRIX ",
            Style::default()
                .fg(accent(color_mode))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("tradeable ", Style::default().fg(success(color_mode))),
        Span::raw(format!(
            "{tradeable}/{total} {}  ",
            depth_bar(tradeable_ratio, 4)
        )),
        Span::styled("degraded ", Style::default().fg(warn(color_mode))),
        Span::raw(format!("{degraded}  ")),
        Span::styled("stale ", Style::default().fg(danger(color_mode))),
        Span::raw(format!("{stale}  ")),
        Span::styled("confidence ", Style::default().fg(success(color_mode))),
        Span::raw(format!("{confidence} {}", depth_bar(confidence_ratio, 4))),
    ])
}

fn health_metric(health_status: &str, key: &str) -> Option<u64> {
    let prefix = format!("{key}=");
    health_status.split_whitespace().find_map(|part| {
        part.strip_prefix(&prefix)
            .and_then(|value| value.parse::<u64>().ok())
    })
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

fn compact_mode_label(request: &ScreenRequest, row_count: usize) -> String {
    let sort = request.sort.clone().or_else(|| {
        request
            .preset
            .as_deref()
            .and_then(find_preset)
            .map(|preset| preset.sort.to_owned())
    });
    sort.map_or_else(
        || format!("top{row_count}"),
        |sort| format!("top{row_count} {}", sort.replace(':', " ")),
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

fn notional(px: Option<f64>, qty: Option<f64>) -> Option<f64> {
    match (px, qty) {
        (Some(px), Some(qty)) => Some(px * qty),
        _ => None,
    }
}

fn quote_share(bid_notional: Option<f64>, ask_notional: Option<f64>) -> Option<(f64, f64)> {
    let bid = positive_finite(bid_notional).unwrap_or(0.0);
    let ask = positive_finite(ask_notional).unwrap_or(0.0);
    let total = bid + ask;
    (total > 0.0).then_some((bid / total, ask / total))
}

fn positive_finite(value: Option<f64>) -> Option<f64> {
    value.filter(|value| value.is_finite() && *value > 0.0)
}

fn percent_label(value: f64) -> String {
    format!("{:.0}%", value.clamp(0.0, 1.0) * 100.0)
}

fn depth_bar(value: f64, width: usize) -> String {
    let clamped = value.clamp(0.0, 1.0);
    let filled = ((clamped * width as f64).round() as usize).min(width);
    let empty = width.saturating_sub(filled);
    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
}

fn depth_bar_empty(width: usize) -> String {
    "░".repeat(width)
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

fn push_ansi_fg(output: &mut String, color: Color) {
    match color {
        Color::Reset => output.push_str("\x1b[39m"),
        Color::Black => output.push_str("\x1b[30m"),
        Color::Red => output.push_str("\x1b[31m"),
        Color::Green => output.push_str("\x1b[32m"),
        Color::Yellow => output.push_str("\x1b[33m"),
        Color::Blue => output.push_str("\x1b[34m"),
        Color::Magenta => output.push_str("\x1b[35m"),
        Color::Cyan => output.push_str("\x1b[36m"),
        Color::Gray => output.push_str("\x1b[37m"),
        Color::DarkGray => output.push_str("\x1b[90m"),
        Color::LightRed => output.push_str("\x1b[91m"),
        Color::LightGreen => output.push_str("\x1b[92m"),
        Color::LightYellow => output.push_str("\x1b[93m"),
        Color::LightBlue => output.push_str("\x1b[94m"),
        Color::LightMagenta => output.push_str("\x1b[95m"),
        Color::LightCyan => output.push_str("\x1b[96m"),
        Color::White => output.push_str("\x1b[97m"),
        Color::Rgb(red, green, blue) => {
            output.push_str(&format!("\x1b[38;2;{red};{green};{blue}m"));
        }
        Color::Indexed(index) => {
            output.push_str(&format!("\x1b[38;5;{index}m"));
        }
    }
}

fn push_ansi_bg(output: &mut String, color: Color) {
    match color {
        Color::Reset => output.push_str("\x1b[49m"),
        Color::Black => output.push_str("\x1b[40m"),
        Color::Red => output.push_str("\x1b[41m"),
        Color::Green => output.push_str("\x1b[42m"),
        Color::Yellow => output.push_str("\x1b[43m"),
        Color::Blue => output.push_str("\x1b[44m"),
        Color::Magenta => output.push_str("\x1b[45m"),
        Color::Cyan => output.push_str("\x1b[46m"),
        Color::Gray => output.push_str("\x1b[47m"),
        Color::DarkGray => output.push_str("\x1b[100m"),
        Color::LightRed => output.push_str("\x1b[101m"),
        Color::LightGreen => output.push_str("\x1b[102m"),
        Color::LightYellow => output.push_str("\x1b[103m"),
        Color::LightBlue => output.push_str("\x1b[104m"),
        Color::LightMagenta => output.push_str("\x1b[105m"),
        Color::LightCyan => output.push_str("\x1b[106m"),
        Color::White => output.push_str("\x1b[107m"),
        Color::Rgb(red, green, blue) => {
            output.push_str(&format!("\x1b[48;2;{red};{green};{blue}m"));
        }
        Color::Indexed(index) => {
            output.push_str(&format!("\x1b[48;5;{index}m"));
        }
    }
}

fn accent(color_mode: RatatuiColorMode) -> Color {
    match color_mode {
        RatatuiColorMode::NoColor => Color::White,
        RatatuiColorMode::Auto | RatatuiColorMode::Color => Color::Rgb(0, 229, 255),
    }
}

fn success(color_mode: RatatuiColorMode) -> Color {
    match color_mode {
        RatatuiColorMode::NoColor => Color::White,
        RatatuiColorMode::Auto | RatatuiColorMode::Color => Color::Rgb(0, 255, 154),
    }
}

fn danger(color_mode: RatatuiColorMode) -> Color {
    match color_mode {
        RatatuiColorMode::NoColor => Color::White,
        RatatuiColorMode::Auto | RatatuiColorMode::Color => Color::Rgb(255, 77, 109),
    }
}

fn flow_color(value: f64, color_mode: RatatuiColorMode) -> Color {
    if value > 0.0 {
        success(color_mode)
    } else if value < 0.0 {
        danger(color_mode)
    } else {
        text(color_mode)
    }
}

fn confidence_color(score: u8, color_mode: RatatuiColorMode) -> Color {
    if score >= 85 {
        success(color_mode)
    } else if score >= 70 {
        warn(color_mode)
    } else {
        danger(color_mode)
    }
}

fn warn(color_mode: RatatuiColorMode) -> Color {
    match color_mode {
        RatatuiColorMode::NoColor => Color::White,
        RatatuiColorMode::Auto | RatatuiColorMode::Color => Color::Rgb(255, 214, 102),
    }
}

fn text(color_mode: RatatuiColorMode) -> Color {
    match color_mode {
        RatatuiColorMode::NoColor => Color::White,
        RatatuiColorMode::Auto | RatatuiColorMode::Color => Color::Rgb(198, 208, 222),
    }
}
