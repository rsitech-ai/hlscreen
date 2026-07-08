use hls_core::market_state::LiveMarketState;
use hls_features::engine::FeatureEngine;
use hls_hyperliquid::ws::parser::parse_ws_ndjson;
use hls_screen::ScreenRequest;
use hls_tui::{
    app::{RenderOptions, render_screened_table_with_options, render_screened_table_with_state},
    interaction::{WorkstationAction, WorkstationUiState, WorkstationView},
};

fn fixture_snapshots() -> Vec<hls_core::market_state::FeatureSnapshot> {
    let events = parse_ws_ndjson(include_str!(
        "../../../tests/fixtures/hyperliquid/ws_mock_live.ndjson"
    ))
    .expect("fixture parses");
    let mut state = LiveMarketState::new(["@107".to_owned()]);
    for event in events {
        state.apply(event).expect("event applies");
    }
    let mut snapshots = FeatureEngine::default().snapshots(&state, 1_710_000_066_000);
    let mut second = snapshots[0].clone();
    second.symbol = "PURR/USDC".to_owned();
    second.price = Some(0.4200);
    snapshots.push(second);
    snapshots
}

#[test]
fn workstation_state_handles_keyboard_actions() {
    let mut state = WorkstationUiState::default();

    state.apply(WorkstationAction::Down, 3);
    assert_eq!(state.selected_index(3), Some(1));

    state.apply(WorkstationAction::PageDown, 3);
    assert_eq!(state.selected_index(3), Some(2));

    state.apply(WorkstationAction::NextView, 3);
    assert_eq!(state.view(), WorkstationView::Flow);

    state.apply(WorkstationAction::ToggleDensity, 3);
    assert_eq!(state.density().label(), "dense");

    state.apply(WorkstationAction::ToggleHelp, 3);
    assert!(state.help_open());

    state.apply(WorkstationAction::TogglePause, 3);
    assert!(state.paused());

    state.apply(WorkstationAction::Quit, 3);
    assert!(state.quit_requested());
}

#[test]
fn interactive_renderer_marks_focused_row_and_view() {
    let snapshots = fixture_snapshots();
    let mut state = WorkstationUiState::default();
    state.apply(WorkstationAction::Down, snapshots.len());
    state.apply(WorkstationAction::NextView, snapshots.len());

    let table = render_screened_table_with_state(
        &snapshots,
        "READ-ONLY Hyperliquid spot live screen",
        &ScreenRequest::default(),
        &state,
    )
    .expect("renders");

    assert!(table.contains("UI ACTIVE"));
    assert!(table.contains("ui: flow · row 2/2"));
    assert!(table.contains("keys arrows/jk"));
    assert!(table.contains("│ ▶ PURR/USDC"));
    assert!(table.contains("Selected: PURR/USDC  | view flow"));
    assert!(table.contains("Flow tape"));
    assert!(table.contains("adverse proxy"));
}

#[test]
fn interactive_renderer_shows_help_overlay_without_mocking_data() {
    let snapshots = fixture_snapshots();
    let mut state = WorkstationUiState::default();
    state.apply(WorkstationAction::ToggleHelp, snapshots.len());
    state.apply(WorkstationAction::NextView, snapshots.len());
    state.apply(WorkstationAction::NextView, snapshots.len());

    let table = render_screened_table_with_state(
        &snapshots,
        "READ-ONLY Hyperliquid spot live screen",
        &ScreenRequest::default(),
        &state,
    )
    .expect("renders");

    assert!(table.contains("command deck: ↑/↓ row"));
    assert!(table.contains("display only: controls change focus"));
    assert!(table.contains("Selected: @107  | view quality"));
    assert!(table.contains("Confidence     level high"));
    assert!(table.contains("No wallet, no private streams, no order routes"));
}

#[test]
fn interactive_renderer_can_fit_narrow_terminals_without_wrapping() {
    let mut snapshots = fixture_snapshots();
    for index in 0..10 {
        let mut row = snapshots[0].clone();
        row.symbol = format!("PAIR{index}/USDC");
        row.price = Some(10.0 + f64::from(index));
        row.spread_bps = Some(2.0 + f64::from(index));
        row.tob_imbalance = Some(-0.5 + f64::from(index) / 10.0);
        snapshots.push(row);
    }

    let mut state = WorkstationUiState::default();
    state.apply(WorkstationAction::ToggleHelp, snapshots.len());
    let table = render_screened_table_with_options(
        &snapshots,
        "READ-ONLY Hyperliquid spot live screen",
        &ScreenRequest::default(),
        Some(&state),
        RenderOptions::for_width(88),
    )
    .expect("renders");

    assert!(table.contains("keys j/k"));
    assert!(table.contains("Selected:"));
    for line in table.lines() {
        assert!(
            line.chars().count() <= 88,
            "line exceeds terminal width: {} chars: {line}",
            line.chars().count()
        );
    }
}

#[test]
fn live_renderer_uses_conservative_width_even_when_terminal_reports_wide() {
    let snapshots = fixture_snapshots();
    let state = WorkstationUiState::default();
    let table = render_screened_table_with_options(
        &snapshots,
        "READ-ONLY Hyperliquid spot live screen",
        &ScreenRequest::default(),
        Some(&state),
        RenderOptions::for_live_terminal_width(180),
    )
    .expect("renders");

    assert!(table.contains("│ symbol"));
    assert!(table.contains("│ spr "));
    assert!(table.contains("│ liq "));
    assert!(!table.contains("sprbp"));
    assert!(!table.contains("amihud"));
    for line in table.lines() {
        assert!(
            line.chars().count() <= 96,
            "line exceeds conservative live width: {} chars: {line}",
            line.chars().count()
        );
    }
}
