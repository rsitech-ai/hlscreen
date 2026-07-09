use hls_core::market_state::{CandleEvent, LiveMarketState, MarketEvent, TradeEvent};
use hls_features::engine::FeatureEngine;
use hls_hyperliquid::{rest::parse_metadata_enrichment_bundle, ws::parser::parse_ws_ndjson};
use hls_screen::ScreenRequest;
use hls_tui::{
    interaction::{WorkstationAction, WorkstationPane, WorkstationUiState},
    ratatui_app::{
        RatatuiColorMode, RatatuiFrameModel, RatatuiViewport, render_ratatui_snapshot_for_test,
    },
};

fn fixture_snapshots() -> Vec<hls_core::market_state::FeatureSnapshot> {
    let events = parse_ws_ndjson(include_str!(
        "../../../tests/fixtures/hyperliquid/ws_mock_live.ndjson"
    ))
    .expect("fixture parses");
    let metadata = parse_metadata_enrichment_bundle(include_str!(
        "../../../tests/fixtures/microstructure/metadata_enrichment.json"
    ))
    .expect("metadata parses");
    let mut state = LiveMarketState::new(["@107".to_owned()]);
    for event in events {
        state.apply(event).expect("event applies");
    }
    let mut snapshots = FeatureEngine::default().snapshots(&state, 1_710_000_066_000);
    for snapshot in &mut snapshots {
        snapshot.metadata = metadata
            .iter()
            .find(|metadata| metadata.feed_identifier == snapshot.symbol)
            .cloned();
    }
    snapshots
}

#[test]
fn cockpit_renders_command_palette_with_validation_error() {
    let snapshots = fixture_snapshots();
    let mut state = WorkstationUiState::default();
    state.apply(WorkstationAction::CycleFilter, snapshots.len());
    for ch in "symbol > 10".chars() {
        state.apply(WorkstationAction::CommandChar(ch), snapshots.len());
    }
    state.set_command_error("type-incompatible comparison".to_owned());
    let model = RatatuiFrameModel::new(
        snapshots,
        "READ-ONLY Hyperliquid spot live screen",
        ScreenRequest::default(),
        state,
    )
    .with_candles(fixture_candles());

    let rendered = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 140,
            height: 40,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders");

    assert!(rendered.contains("COMMAND"));
    assert!(rendered.contains("filter"));
    assert!(rendered.contains("symbol > 10"));
    assert!(rendered.contains("type-incompatible comparison"));
    assert!(rendered.contains("Enter apply"));
}

fn fixture_candles() -> Vec<CandleEvent> {
    parse_ws_ndjson(include_str!(
        "../../../tests/fixtures/hyperliquid/ws_mock_live.ndjson"
    ))
    .expect("fixture parses")
    .into_iter()
    .filter_map(|event| match event {
        MarketEvent::Candle(candle) => Some(candle),
        _ => None,
    })
    .collect()
}

fn fixture_trades() -> Vec<TradeEvent> {
    parse_ws_ndjson(include_str!(
        "../../../tests/fixtures/hyperliquid/ws_mock_live.ndjson"
    ))
    .expect("fixture parses")
    .into_iter()
    .filter_map(|event| match event {
        MarketEvent::Trade(trade) => Some(trade),
        _ => None,
    })
    .collect()
}

#[test]
fn wide_cockpit_renders_all_primary_trading_workstation_regions() {
    let model = RatatuiFrameModel::new(
        directional_snapshots(),
        "READ-ONLY Hyperliquid spot live screen",
        ScreenRequest::default(),
        WorkstationUiState::default(),
    )
    .with_status("LIVE", "REC ready", "ws=120 events=300 gaps=0");

    let rendered = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 240,
            height: 48,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders");

    assert!(rendered.contains("WATCHLIST"));
    assert!(rendered.contains("MICROSTRUCTURE"));
    assert!(rendered.contains("CHART"));
    assert!(rendered.contains("TAPE"));
    assert!(rendered.contains("BOOK"));
    assert!(rendered.contains("BID"));
    assert!(rendered.contains("ASK"));
    assert!(rendered.contains("notional"));
    assert!(rendered.contains("imbalance"));
    assert!(rendered.contains("Selected flow"));
    assert!(rendered.contains("Flow leaderboard"));
    assert!(rendered.contains("OFI"));
    assert!(rendered.contains("HYPE/USDC"));
    assert!(rendered.contains("confidence"));
    assert!(rendered.contains("No wallet"));
    assert!(rendered.contains("MARKET"));
    assert!(rendered.contains("HYPE/USDC UP+0.57%"));
    assert!(rendered.contains("DOWN/USDC DN-1.23% -$4.2K"));
    assert!(rendered.contains("STATUS LIVE"));
    assert!(rendered.contains("CONTROLS"));
    assert!(rendered.contains("QUALITY"));
    assert!(rendered.contains("stale00"));
    assert!(rendered.contains("1-6 panes"));
    assert!(rendered.contains("RANK"));
    assert!(rendered.contains("FLOW30"));
    assert!(rendered.contains("DEPTH"));
    assert!(rendered.contains("Q"));
    assert!(rendered.contains("01"));
    assert!(rendered.contains("UP"));
    assert!(rendered.contains("DN"));
}

fn directional_snapshots() -> Vec<hls_core::market_state::FeatureSnapshot> {
    let mut snapshots = fixture_snapshots();
    snapshots[0].ret_1m = Some(0.0057);
    let mut down_row = snapshots[0].clone();
    down_row.symbol = "DOWN/USDC".to_owned();
    down_row.metadata = None;
    down_row.price = Some(12.34);
    down_row.ret_1m = Some(-0.0123);
    down_row.signed_notional_flow_30s = Some(-4_200.0);
    snapshots.push(down_row);
    snapshots
}

fn ten_directional_snapshots() -> Vec<hls_core::market_state::FeatureSnapshot> {
    let mut snapshots = directional_snapshots();
    let seed = snapshots[0].clone();
    for index in snapshots.len()..10 {
        let mut row = seed.clone();
        row.symbol = format!("ROW{}/USDC", index + 1);
        row.metadata = None;
        row.price = Some(10.0 + index as f64);
        row.ret_1m = Some(if index % 2 == 0 { 0.003 } else { -0.003 });
        row.signed_notional_flow_30s = Some(if index % 2 == 0 { 1_000.0 } else { -1_000.0 });
        snapshots.push(row);
    }
    snapshots
}

#[test]
fn medium_cockpit_compacts_market_board_without_truncated_signals() {
    let model = RatatuiFrameModel::new(
        directional_snapshots(),
        "READ-ONLY Hyperliquid spot live screen",
        ScreenRequest::default(),
        WorkstationUiState::default(),
    );

    let rendered = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 120,
            height: 36,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders");

    assert!(rendered.contains("RK"));
    assert!(rendered.contains("FLOW"));
    assert!(rendered.contains("UP+0.57%"));
    assert!(rendered.contains("DN-1.23%"));
}

#[test]
fn narrow_watchlist_scrolls_keyboard_selection_into_view() {
    let snapshots = ten_directional_snapshots();
    let mut state = WorkstationUiState::default();
    state.apply(WorkstationAction::End, snapshots.len());
    let model = RatatuiFrameModel::new(
        snapshots,
        "READ-ONLY Hyperliquid spot live screen",
        ScreenRequest::default(),
        state,
    );

    let rendered = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 72,
            height: 24,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders selected watchlist row");

    assert!(rendered.contains("[FOCUS] WATCHLIST 10/10 VIEW 05-10"));
    assert!(rendered.contains(">10"));
    assert!(rendered.contains("ROW10/USDC"));
    assert!(!rendered.contains("01 HYPE/USDC"));
    assert!(rendered.contains("DETAIL"));
}

#[test]
fn market_board_renders_score_and_bias_columns() {
    let model = RatatuiFrameModel::new(
        fixture_snapshots(),
        "READ-ONLY Hyperliquid spot live screen",
        ScreenRequest::default(),
        WorkstationUiState::default(),
    );

    let rendered = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 240,
            height: 48,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders scored market board");

    assert!(rendered.contains("SIG"));
    assert!(rendered.contains("EDGE"));
    assert!(rendered.contains("BIAS"));
    assert!(rendered.contains("SPR"));
    assert!(rendered.contains("57.1"));
    assert!(rendered.contains("MOM+"));
    assert!(rendered.contains("██"));
    assert!(rendered.contains("13"));
}

#[test]
fn market_board_renders_directional_edge_pulses() {
    let model = RatatuiFrameModel::new(
        directional_snapshots(),
        "READ-ONLY Hyperliquid spot live screen",
        ScreenRequest::default(),
        WorkstationUiState::default(),
    );

    let rendered = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 240,
            height: 48,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders directional market board");

    assert!(rendered.contains("EDGE"));
    assert!(rendered.contains("▲"));
    assert!(rendered.contains("▼"));
    assert!(rendered.contains("UP+0.57%"));
    assert!(rendered.contains("DN-1.23%"));
}

#[test]
fn cockpit_header_renders_market_internals_rail() {
    let model = RatatuiFrameModel::new(
        directional_snapshots(),
        "READ-ONLY Hyperliquid spot live screen",
        ScreenRequest::default(),
        WorkstationUiState::default(),
    )
    .with_status("LIVE", "REC ready", "ws=120 events=300 gaps=0");

    let rendered = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 160,
            height: 40,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders internals rail");

    assert!(rendered.contains("INTERNALS"));
    assert!(rendered.contains("rows 02"));
    assert!(rendered.contains("up 01"));
    assert!(rendered.contains("down 01"));
    assert!(rendered.contains("flow -$4.2K"));
    assert!(rendered.contains("depth $490"));
    assert!(rendered.contains("tradeable"));
}

#[test]
fn detail_panel_renders_score_factor_stack() {
    let model = RatatuiFrameModel::new(
        fixture_snapshots(),
        "READ-ONLY Hyperliquid spot live screen",
        ScreenRequest::default(),
        WorkstationUiState::default(),
    );

    let rendered = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 160,
            height: 48,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders factor stack");

    assert!(rendered.contains("FACTOR STACK"));
    assert!(rendered.contains("score raw"));
    assert!(rendered.contains("adj"));
    assert!(rendered.contains("mean"));
    assert!(rendered.contains("mom"));
    assert!(rendered.contains("spread"));
}

#[test]
fn detail_panel_renders_liquidity_radar() {
    let model = RatatuiFrameModel::new(
        fixture_snapshots(),
        "READ-ONLY Hyperliquid spot live screen",
        ScreenRequest::default(),
        WorkstationUiState::default(),
    );

    let rendered = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 160,
            height: 48,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders liquidity radar");

    assert!(rendered.contains("LIQUIDITY RADAR"));
    assert!(rendered.contains("spread cost"));
    assert!(rendered.contains("depth"));
    assert!(rendered.contains("imbalance"));
    assert!(rendered.contains("flow"));
    assert!(rendered.contains("Public BBO/flow only"));
    assert!(rendered.contains("█"));
}

#[test]
fn detail_panel_renders_interactive_view_tab_rail() {
    let snapshots = fixture_snapshots();
    let mut state = WorkstationUiState::default();
    state.apply(WorkstationAction::NextView, snapshots.len());
    let model = RatatuiFrameModel::new(
        snapshots,
        "READ-ONLY Hyperliquid spot live screen",
        ScreenRequest::default(),
        state,
    );

    let rendered = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 160,
            height: 48,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders detail view tabs");

    assert!(rendered.contains("VIEWS overview [flow] quality metadata explain"));
    assert!(rendered.contains("Flow tape"));
    assert!(rendered.contains("FLOW LADDER"));
    assert!(rendered.contains("pressure"));
    assert!(rendered.contains("imbalance"));
    assert!(rendered.contains("friction spr"));
    assert!(rendered.contains("Public BBO/trade context only"));
    assert!(rendered.contains("display heuristic, not advice"));
}

#[test]
fn narrow_cockpit_collapses_to_watchlist_and_detail_without_tape() {
    let model = RatatuiFrameModel::new(
        fixture_snapshots(),
        "READ-ONLY Hyperliquid spot live screen",
        ScreenRequest::default(),
        WorkstationUiState::default(),
    )
    .with_status("LIVE", "REC ready", "ws=235 events=485 reconnects=0 gaps=0");

    let rendered = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 72,
            height: 24,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders");

    assert!(rendered.contains("WATCHLIST"));
    assert!(rendered.contains("DETAIL"));
    assert!(rendered.contains("v:overview p:watchlist d:balanced c:15m"));
    assert!(rendered.contains("j/k 1-6 tab / p s t ? q"));
    assert!(rendered.contains("INT rows"));
    assert!(rendered.contains(" dn "));
    assert!(rendered.contains(" tr "));
    assert!(rendered.contains(" heat "));
    assert!(rendered.contains(" dp "));
    assert!(rendered.contains("ws235 ev485 r0 g0"));
    assert!(rendered.contains("live | watchlist:j/k | top1"));
    assert!(rendered.contains("q:T"));
    assert!(rendered.contains("RO no-wallet"));
    assert!(!rendered.contains("top-10 by screen rank"));
    assert!(rendered.contains("BBO bid"));
    assert!(rendered.contains("ask"));
    assert!(rendered.contains("FACTORS"));
    assert!(!rendered.contains("FACTOR STACK"));
    assert!(rendered.contains("HYPE/USDC"));
    assert!(!rendered.contains("TAPE"));
}

#[test]
fn narrow_status_bar_renders_contextual_focus_keys() {
    let snapshots = fixture_snapshots();
    let mut state = WorkstationUiState::default();
    state.apply(
        WorkstationAction::FocusPane(WorkstationPane::Chart),
        snapshots.len(),
    );
    let model = RatatuiFrameModel::new(
        snapshots,
        "READ-ONLY Hyperliquid spot live screen",
        ScreenRequest::default(),
        state,
    )
    .with_status("LIVE", "REC ready", "ws=235 events=485 reconnects=0 gaps=0");

    let rendered = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 72,
            height: 24,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders focused chart status hint");

    assert!(rendered.contains("live | chart:t | top1"));
    assert!(rendered.contains("RO no-wallet"));
}

#[test]
fn header_renders_keyboard_pane_hotkey_rail() {
    let snapshots = fixture_snapshots();
    let mut state = WorkstationUiState::default();
    state.apply(
        WorkstationAction::FocusPane(WorkstationPane::Book),
        snapshots.len(),
    );
    let model = RatatuiFrameModel::new(
        snapshots,
        "READ-ONLY Hyperliquid spot live screen",
        ScreenRequest::default(),
        state,
    );

    let rendered = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 72,
            height: 24,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders pane hotkey rail");

    assert!(rendered.contains("CONTROLS 1W 2D 3C [4B] 5T 6S"));
    assert!(rendered.contains("j/k 1-6 tab / p s t ? q"));
    assert!(rendered.contains("[FOCUS] BOOK"));
}

#[test]
fn medium_cockpit_keeps_book_and_tape_visible() {
    let model = RatatuiFrameModel::new(
        fixture_snapshots(),
        "READ-ONLY Hyperliquid spot live screen",
        ScreenRequest::default(),
        WorkstationUiState::default(),
    )
    .with_candles(fixture_candles());

    let rendered = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 160,
            height: 48,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders");

    assert!(rendered.contains("WATCHLIST"));
    assert!(rendered.contains("MICROSTRUCTURE"));
    assert!(rendered.contains("CANDLES"));
    assert!(rendered.contains("BOOK"));
    assert!(rendered.contains("TAPE"));
    assert!(rendered.contains("BID"));
    assert!(rendered.contains("Selected flow"));
}

#[test]
fn medium_cockpit_keeps_compact_tape_flow_and_safety_visible() {
    let model = RatatuiFrameModel::new(
        fixture_snapshots(),
        "READ-ONLY Hyperliquid spot live screen",
        ScreenRequest::default(),
        WorkstationUiState::default(),
    )
    .with_candles(fixture_candles());

    let rendered = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 120,
            height: 36,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders compact tape");

    assert!(rendered.contains("TAPE"));
    assert!(rendered.contains("FLOW pulse"));
    assert!(rendered.contains("Tape proxy only"));
}

#[test]
fn book_pane_renders_bid_ask_share_and_notional_bars() {
    let snapshots = fixture_snapshots();
    let mut state = WorkstationUiState::default();
    state.apply(
        WorkstationAction::FocusPane(WorkstationPane::Book),
        snapshots.len(),
    );
    let model = RatatuiFrameModel::new(
        snapshots,
        "READ-ONLY Hyperliquid spot live screen",
        ScreenRequest::default(),
        state,
    );

    let rendered = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 120,
            height: 36,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders book depth");

    assert!(rendered.contains("[FOCUS] BOOK"));
    assert!(rendered.contains("share bid"));
    assert!(rendered.contains("ask"));
    assert!(rendered.contains("BID notional"));
    assert!(rendered.contains("ASK notional"));
    assert!(rendered.contains("BOOK proxy only"));
}

#[test]
fn book_pane_flow_view_renders_depth_flow_mode() {
    let snapshots = fixture_snapshots();
    let mut state = WorkstationUiState::default();
    state.apply(
        WorkstationAction::FocusPane(WorkstationPane::Book),
        snapshots.len(),
    );
    state.apply(WorkstationAction::NextView, snapshots.len());
    let model = RatatuiFrameModel::new(
        snapshots,
        "READ-ONLY Hyperliquid spot live screen",
        ScreenRequest::default(),
        state,
    );

    let rendered = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 120,
            height: 36,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders book flow mode");

    assert!(rendered.contains("view:flow"));
    assert!(rendered.contains("[FOCUS] BOOK"));
    assert!(rendered.contains("BOOK FLOW MODE"));
    assert!(rendered.contains("depth skew"));
    assert!(rendered.contains("spread gate"));
    assert!(rendered.contains("Public top-book only"));
}

#[test]
fn tape_pane_renders_flow_pulse_and_net_pressure_bars() {
    let snapshots = directional_snapshots();
    let mut state = WorkstationUiState::default();
    state.apply(
        WorkstationAction::FocusPane(WorkstationPane::Tape),
        snapshots.len(),
    );
    let model = RatatuiFrameModel::new(
        snapshots,
        "READ-ONLY Hyperliquid spot live screen",
        ScreenRequest::default(),
        state,
    );

    let rendered = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 160,
            height: 48,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders flow tape");

    assert!(rendered.contains("[FOCUS] TAPE"));
    assert!(rendered.contains("FLOW pulse"));
    assert!(rendered.contains("net pressure"));
    assert!(rendered.contains("Tape proxy only"));
    assert!(rendered.contains("HYPE/USDC"));
    assert!(rendered.contains("DOWN/USDC"));
}

#[test]
fn tape_pane_renders_public_recent_trades_when_available() {
    let snapshots = fixture_snapshots();
    let mut state = WorkstationUiState::default();
    state.apply(
        WorkstationAction::FocusPane(WorkstationPane::Tape),
        snapshots.len(),
    );
    let model = RatatuiFrameModel::new(
        snapshots,
        "READ-ONLY Hyperliquid spot live screen",
        ScreenRequest::default(),
        state,
    )
    .with_trades(fixture_trades());

    let rendered = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 160,
            height: 48,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders public trade tape");

    assert!(rendered.contains("[FOCUS] TAPE"));
    assert!(rendered.contains("PUBLIC TRADES"));
    assert!(rendered.contains("BUY"));
    assert!(rendered.contains("SELL"));
    assert!(rendered.contains("notional"));
    assert!(rendered.contains("Public trades only | no fills"));
}

#[test]
fn tape_pane_flow_view_renders_public_trade_pressure_mode() {
    let snapshots = fixture_snapshots();
    let mut state = WorkstationUiState::default();
    state.apply(
        WorkstationAction::FocusPane(WorkstationPane::Tape),
        snapshots.len(),
    );
    state.apply(WorkstationAction::NextView, snapshots.len());
    let model = RatatuiFrameModel::new(
        snapshots,
        "READ-ONLY Hyperliquid spot live screen",
        ScreenRequest::default(),
        state,
    )
    .with_trades(fixture_trades());

    let rendered = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 160,
            height: 48,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders public trade pressure mode");

    assert!(rendered.contains("view:flow"));
    assert!(rendered.contains("[FOCUS] TAPE"));
    assert!(rendered.contains("TRADE FLOW MODE"));
    assert!(rendered.contains("buy pressure"));
    assert!(rendered.contains("sell pressure"));
    assert!(rendered.contains("Public trades only | no fills"));
}

#[test]
fn narrow_cockpit_renders_focused_hidden_pane_as_drilldown() {
    let snapshots = fixture_snapshots();
    let mut state = WorkstationUiState::default();
    state.apply(
        WorkstationAction::FocusPane(WorkstationPane::Book),
        snapshots.len(),
    );
    let model = RatatuiFrameModel::new(
        snapshots.clone(),
        "READ-ONLY Hyperliquid spot live screen",
        ScreenRequest::default(),
        state,
    );

    let rendered_book = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 72,
            height: 24,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders book drilldown");

    assert!(rendered_book.contains("[FOCUS] BOOK"));
    assert!(rendered_book.contains("BID"));
    assert!(rendered_book.contains("imbalance"));

    let mut state = WorkstationUiState::default();
    state.apply(
        WorkstationAction::FocusPane(WorkstationPane::Tape),
        snapshots.len(),
    );
    let model = RatatuiFrameModel::new(
        snapshots,
        "READ-ONLY Hyperliquid spot live screen",
        ScreenRequest::default(),
        state,
    );

    let rendered_tape = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 72,
            height: 24,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders tape drilldown");

    assert!(rendered_tape.contains("[FOCUS] TAPE"));
    assert!(rendered_tape.contains("Selected flow"));
    assert!(rendered_tape.contains("Flow leaderboard"));
}

#[test]
fn narrow_cockpit_renders_status_focus_as_operational_drilldown() {
    let snapshots = fixture_snapshots();
    let mut state = WorkstationUiState::default();
    state.apply(
        WorkstationAction::FocusPane(WorkstationPane::Status),
        snapshots.len(),
    );
    let model = RatatuiFrameModel::new(
        snapshots,
        "READ-ONLY Hyperliquid spot live screen",
        ScreenRequest::default(),
        state,
    )
    .with_status("LIVE", "REC ready", "ws=235 events=485 reconnects=0 gaps=0");

    let rendered = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 72,
            height: 24,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders status drilldown");

    assert!(rendered.contains("[FOCUS] STATUS"));
    assert!(rendered.contains("stream LIVE"));
    assert!(rendered.contains("recorder REC ready"));
    assert!(rendered.contains("ws=235 events=485 reconnects=0 gaps=0"));
    assert!(rendered.contains("LIVE OPS"));
    assert!(rendered.contains("ws 235"));
    assert!(rendered.contains("events 485"));
    assert!(rendered.contains("reconnects 0"));
    assert!(rendered.contains("gaps 0"));
    assert!(rendered.contains("ingest"));
    assert!(rendered.contains("pane status"));
    assert!(rendered.contains("terminal color no-color"));
    assert!(rendered.contains("palette plain"));
    assert!(rendered.contains("--color always"));
    assert!(rendered.contains("OPS DECK"));
    assert!(rendered.contains("1-6 focus"));
    assert!(rendered.contains("/ filter"));
    assert!(rendered.contains("p preset"));
    assert!(rendered.contains("s sort"));
    assert!(rendered.contains("t chart"));
    assert!(rendered.contains("space pause"));
    assert!(rendered.contains("active top-1 by screen rank"));
    assert!(rendered.contains("read-only safety"));
    assert!(rendered.contains("No wallet"));
    assert!(!rendered.contains("[FOCUS] DETAIL"));
}

#[test]
fn cockpit_color_mode_is_explicit_and_does_not_pollute_no_color_snapshots() {
    let model = RatatuiFrameModel::new(
        fixture_snapshots(),
        "READ-ONLY Hyperliquid spot live screen",
        ScreenRequest::default(),
        WorkstationUiState::default(),
    );

    let plain = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 120,
            height: 36,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("plain renders");
    let colored = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 120,
            height: 36,
        },
        RatatuiColorMode::Color,
    )
    .expect("colored renders");

    assert!(!plain.contains("\u{1b}["));
    assert!(colored.contains("\u{1b}["));
    assert!(colored.contains("\u{1b}[38;2;"));
    assert!(colored.contains("\u{1b}[48;2;"));
    assert!(colored.contains("WATCHLIST"));
}

#[test]
fn cockpit_reflects_keyboard_view_pause_density_and_help_state() {
    let snapshots = fixture_snapshots();
    let mut state = WorkstationUiState::default();
    state.apply(WorkstationAction::NextView, snapshots.len());
    state.apply(WorkstationAction::NextPane, snapshots.len());
    state.apply(WorkstationAction::NextPane, snapshots.len());
    state.apply(WorkstationAction::ToggleDensity, snapshots.len());
    state.apply(WorkstationAction::TogglePause, snapshots.len());
    state.apply(WorkstationAction::ToggleHelp, snapshots.len());
    state.apply(WorkstationAction::CycleChartWindow, snapshots.len());
    let model = RatatuiFrameModel::new(
        snapshots,
        "READ-ONLY Hyperliquid spot live screen",
        ScreenRequest::default(),
        state,
    );

    let rendered = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 140,
            height: 40,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders");

    assert!(rendered.contains("view:flow"));
    assert!(rendered.contains("pane:chart"));
    assert!(rendered.contains("dens:dense"));
    assert!(rendered.contains("chart:30m"));
    assert!(rendered.contains("focus chart"));
    assert!(rendered.contains("display paused"));
    assert!(rendered.contains("HELP"));
    assert!(rendered.contains("Command Deck"));
    assert!(rendered.contains("[ / ]"));
    assert!(rendered.contains("1-6 panes"));
    assert!(rendered.contains("/ filter"));
    assert!(!rendered.contains("reserved"));
}

#[test]
fn cockpit_chart_uses_real_candle_ohlc_and_volume_when_available() {
    let model = RatatuiFrameModel::new(
        fixture_snapshots(),
        "READ-ONLY Hyperliquid spot live screen",
        ScreenRequest::default(),
        WorkstationUiState::default(),
    )
    .with_candles(fixture_candles());

    let rendered = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 160,
            height: 48,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders");

    assert!(rendered.contains("CANDLES 1m"));
    assert!(rendered.contains("O 34.5000"));
    assert!(rendered.contains("H 35.2000"));
    assert!(rendered.contains("L 34.4000"));
    assert!(rendered.contains("C 35.0000"));
    assert!(rendered.contains("VOL 1200"));
    assert!(rendered.contains("MOVE +0.5000"));
    assert!(rendered.contains("RANGE 2.32%"));
}

#[test]
fn cockpit_chart_renders_price_axis_and_public_candle_footer() {
    let snapshots = fixture_snapshots();
    let mut state = WorkstationUiState::default();
    state.apply(
        WorkstationAction::FocusPane(WorkstationPane::Chart),
        snapshots.len(),
    );
    let model = RatatuiFrameModel::new(
        snapshots,
        "READ-ONLY Hyperliquid spot live screen",
        ScreenRequest::default(),
        state,
    )
    .with_candles(fixture_candles());

    let rendered = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 160,
            height: 48,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders chart with price axis");

    assert!(rendered.contains("[FOCUS] CANDLES"));
    assert!(rendered.contains("px axis"));
    assert!(rendered.contains("candles"));
    assert!(rendered.contains("window"));
    assert!(rendered.contains("Public 1m candles only"));
}

#[test]
fn cockpit_chart_renders_selected_pair_edge_hud() {
    let snapshots = directional_snapshots();
    let mut state = WorkstationUiState::default();
    state.apply(
        WorkstationAction::FocusPane(WorkstationPane::Chart),
        snapshots.len(),
    );
    let model = RatatuiFrameModel::new(
        snapshots,
        "READ-ONLY Hyperliquid spot live screen",
        ScreenRequest::default(),
        state,
    )
    .with_candles(fixture_candles());

    let rendered = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 160,
            height: 48,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders chart edge hud");

    assert!(rendered.contains("EDGE HUD"));
    assert!(rendered.contains("trade unknown"));
    assert!(rendered.contains("conf 100"));
    assert!(rendered.contains("spr 57.1bps"));
    assert!(rendered.contains("risk unknown"));
    assert!(rendered.contains("LIQ"));
    assert!(rendered.contains("flow -$35"));
    assert!(rendered.contains("depth $245"));
    assert!(rendered.contains("imb -0.15"));
    assert!(rendered.contains("score 2"));
}

#[test]
fn cockpit_chart_renders_interactive_window_tab_rail() {
    let snapshots = fixture_snapshots();
    let mut state = WorkstationUiState::default();
    state.apply(
        WorkstationAction::FocusPane(WorkstationPane::Chart),
        snapshots.len(),
    );
    state.apply(WorkstationAction::CycleChartWindow, snapshots.len());
    let model = RatatuiFrameModel::new(
        snapshots,
        "READ-ONLY Hyperliquid spot live screen",
        ScreenRequest::default(),
        state,
    )
    .with_candles(fixture_candles());

    let rendered = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 160,
            height: 48,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders chart with window tabs");

    assert!(rendered.contains("WINDOWS 1m 5m 15m [30m] 60m"));
    assert!(rendered.contains("chart:30m"));
}

#[test]
fn narrow_chart_focus_renders_compact_window_controls() {
    let snapshots = fixture_snapshots();
    let mut state = WorkstationUiState::default();
    state.apply(
        WorkstationAction::FocusPane(WorkstationPane::Chart),
        snapshots.len(),
    );
    let model = RatatuiFrameModel::new(
        snapshots,
        "READ-ONLY Hyperliquid spot live screen",
        ScreenRequest::default(),
        state,
    )
    .with_candles(fixture_candles());

    let rendered = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 72,
            height: 24,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders compact chart controls");

    assert!(rendered.contains("[FOCUS] CANDLES"));
    assert!(rendered.contains("WIN 1 5 [15] 30 60"));
    assert!(rendered.contains("t:window"));
    assert!(rendered.contains("EDGE HUD"));
    assert!(rendered.contains("LIQ"));
    assert!(rendered.contains("live | chart:t | top1"));
    assert!(rendered.contains("RO no-wallet"));
}
