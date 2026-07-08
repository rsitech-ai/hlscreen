use hls_core::market_state::{CandleEvent, LiveMarketState, MarketEvent};
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
            width: 200,
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
    assert!(rendered.contains("STATUS LIVE"));
    assert!(rendered.contains("CONTROLS"));
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
fn narrow_cockpit_collapses_to_watchlist_and_detail_without_tape() {
    let model = RatatuiFrameModel::new(
        fixture_snapshots(),
        "READ-ONLY Hyperliquid spot live screen",
        ScreenRequest::default(),
        WorkstationUiState::default(),
    );

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
    assert!(rendered.contains("HYPE/USDC"));
    assert!(!rendered.contains("TAPE"));
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
            width: 120,
            height: 36,
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
    assert!(rendered.contains("pane status"));
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
}
