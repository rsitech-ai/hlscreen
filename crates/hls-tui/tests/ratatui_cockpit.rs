use hls_core::{
    confidence::ConfidenceLevel,
    market_state::{CandleEvent, LiveMarketState, MarketEvent, StalenessState, TradeEvent},
};
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
    assert!(rendered.contains("COMMAND CENTER"));
    assert!(rendered.contains("TARGET filter"));
    assert!(rendered.contains("COMMAND ROUTER"));
    assert!(rendered.contains("live ingestion continues"));
    assert!(rendered.contains("RESULT PREVIEW"));
    assert!(rendered.contains("top HYPE/USDC"));
    assert!(rendered.contains("selected HYPE/USDC"));
    assert!(rendered.contains("last valid screen retained"));
    assert!(rendered.contains("KEYFLOW"));
    assert!(rendered.contains("t timeframe"));
    assert!(rendered.contains("GUARDRAILS"));
    assert!(rendered.contains("display mutation only"));
    assert!(rendered.contains("SCOPE read-only screened rows"));
    assert!(rendered.contains("EXAMPLES"));
    assert!(rendered.contains("filter: spread_bps < 5"));
    assert!(rendered.contains("SAFETY no orders"));
    assert!(rendered.contains("filter"));
    assert!(rendered.contains("symbol > 10"));
    assert!(rendered.contains("type-incompatible comparison"));
    assert!(rendered.contains("Enter apply"));
}

#[test]
fn command_palette_color_mode_renders_semantic_command_deck() {
    let snapshots = fixture_snapshots();
    let mut state = WorkstationUiState::default();
    state.apply(WorkstationAction::CycleFilter, snapshots.len());
    for ch in "spread_bps < 5".chars() {
        state.apply(WorkstationAction::CommandChar(ch), snapshots.len());
    }
    state.set_command_error("invalid filter token".to_owned());
    let model = RatatuiFrameModel::new(
        snapshots,
        "READ-ONLY Hyperliquid spot live screen",
        ScreenRequest::default(),
        state,
    );

    let plain = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 140,
            height: 40,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("plain command deck renders");
    let colored = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 140,
            height: 40,
        },
        RatatuiColorMode::Color,
    )
    .expect("colored command deck renders");

    assert!(!plain.contains("\u{1b}["));
    assert_eq!(
        active_fg_before(&colored, "COMMAND CENTER"),
        Some("\u{1b}[38;2;0;229;255m")
    );
    assert_eq!(
        active_fg_before(&colored, "TARGET"),
        Some("\u{1b}[38;2;255;214;102m")
    );
    assert_eq!(
        active_fg_before(&colored, "INPUT"),
        Some("\u{1b}[38;2;0;255;154m")
    );
    assert_eq!(
        active_fg_before(&colored, "ERROR"),
        Some("\u{1b}[38;2;255;77;109m")
    );
    assert!(colored.contains("COMMAND ROUTER"));
    assert!(colored.contains("SMART SUGGESTIONS"));
    assert!(colored.contains("SAFETY no orders"));
}

fn active_fg_before<'a>(rendered: &'a str, label: &str) -> Option<&'a str> {
    let label_index = rendered.find(label)?;
    let prefix = &rendered[..label_index];
    let fg_index = prefix.rfind("\u{1b}[38;2;")?;
    let fg_end = prefix[fg_index..].find('m')?;
    Some(&prefix[fg_index..fg_index + fg_end + 1])
}

#[test]
fn command_palette_renders_preset_deck_with_active_context() {
    let snapshots = fixture_snapshots();
    let mut state = WorkstationUiState::default();
    state.apply(WorkstationAction::CyclePreset, snapshots.len());
    for ch in "flow_pressure".chars() {
        state.apply(WorkstationAction::CommandChar(ch), snapshots.len());
    }
    let model = RatatuiFrameModel::new(
        snapshots,
        "READ-ONLY Hyperliquid spot live screen",
        ScreenRequest {
            preset: Some("liquidity_resilience".to_owned()),
            where_expr: None,
            sort: Some("score:desc".to_owned()),
        },
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
    .expect("renders preset command deck");

    assert!(rendered.contains("COMMAND CENTER"));
    assert!(rendered.contains("TARGET preset"));
    assert!(rendered.contains("INPUT flow_pressure"));
    assert!(rendered.contains("ACTIVE preset liquidity_resilience"));
    assert!(rendered.contains("sort score:desc"));
    assert!(rendered.contains("visible rows 01"));
    assert!(rendered.contains("PRESET DECK"));
    assert!(rendered.contains("liquidity_resilience"));
    assert!(rendered.contains("flow_pressure"));
    assert!(rendered.contains("metadata_unknown"));
    assert!(rendered.contains("read-only presets"));
}

#[test]
fn command_palette_renders_symbol_jump_deck() {
    let snapshots = fixture_snapshots();
    let mut state = WorkstationUiState::default();
    state.apply(WorkstationAction::OpenSymbolSearch, snapshots.len());
    for ch in "hype".chars() {
        state.apply(WorkstationAction::CommandChar(ch), snapshots.len());
    }
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
    .expect("renders symbol command deck");

    assert!(rendered.contains("COMMAND CENTER"));
    assert!(rendered.contains("TARGET symbol"));
    assert!(rendered.contains("INPUT hype"));
    assert!(rendered.contains("KEYFLOW g symbol"));
    assert!(rendered.contains("symbol: hype"));
    assert!(rendered.contains("Enter jump visible row"));
    assert!(rendered.contains("SYMBOL DECK visible rows only"));
    assert!(rendered.contains("SAFETY no orders"));
}

#[test]
fn command_palette_renders_live_symbol_suggestions() {
    let snapshots = directional_snapshots();
    let mut state = WorkstationUiState::default();
    state.apply(WorkstationAction::OpenSymbolSearch, snapshots.len());
    for ch in "down".chars() {
        state.apply(WorkstationAction::CommandChar(ch), snapshots.len());
    }
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
    .expect("renders symbol suggestions");

    assert!(rendered.contains("SMART SUGGESTIONS"));
    assert!(rendered.contains("symbols DOWN/USDC"));
    assert!(rendered.contains("visible live rows"));
    assert!(rendered.contains("Enter accepts highlighted visible row"));
    assert!(rendered.contains("SAFETY no orders"));
}

#[test]
fn narrow_command_palette_renders_compact_operator_deck() {
    let snapshots = directional_snapshots();
    let mut state = WorkstationUiState::default();
    state.apply(WorkstationAction::OpenSymbolSearch, snapshots.len());
    for ch in "down".chars() {
        state.apply(WorkstationAction::CommandChar(ch), snapshots.len());
    }
    let model = RatatuiFrameModel::new(
        snapshots,
        "READ-ONLY Hyperliquid spot live screen",
        ScreenRequest::default(),
        state,
    );

    let plain = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 72,
            height: 24,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("plain compact command palette renders");
    let colored = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 72,
            height: 24,
        },
        RatatuiColorMode::Color,
    )
    .expect("colored compact command palette renders");

    assert!(!plain.contains("\u{1b}["));
    assert!(plain.contains("COMMAND COMPACT"));
    assert!(plain.contains("symbol > down"));
    assert!(plain.contains("DOWN/USDC"));
    assert!(plain.contains("Enter apply"));
    assert!(plain.contains("Esc cancel"));
    assert!(plain.contains("RO no-wallet"));
    assert!(!plain.contains("COMMAND CENTER"));
    assert_eq!(
        active_fg_before(&colored, "COMMAND COMPACT"),
        Some("\u{1b}[38;2;0;229;255m")
    );
}

#[test]
fn cockpit_header_renders_adaptive_layout_profile() {
    let model = RatatuiFrameModel::new(
        directional_snapshots(),
        "READ-ONLY Hyperliquid spot live screen",
        ScreenRequest::default(),
        WorkstationUiState::default(),
    )
    .with_status("LIVE", "REC ready", "ws=120 events=300 gaps=0");

    let wide = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 240,
            height: 48,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders wide layout profile");
    let medium = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 120,
            height: 40,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders medium layout profile");
    let narrow = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 72,
            height: 24,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders narrow layout profile");

    assert!(wide.contains("layout wide 240x48"));
    assert!(medium.contains("layout medium 120x40"));
    assert!(narrow.contains("layout narrow 72x24"));
}

#[test]
fn cockpit_header_renders_layout_director_across_viewports() {
    let model = RatatuiFrameModel::new(
        directional_snapshots(),
        "READ-ONLY Hyperliquid spot live screen",
        ScreenRequest::default(),
        WorkstationUiState::default(),
    )
    .with_status("LIVE", "REC ready", "ws=120 events=300 gaps=0");

    let wide = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 240,
            height: 48,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders wide layout director");
    let medium = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 120,
            height: 40,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders medium layout director");
    let narrow = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 72,
            height: 24,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders narrow layout director");

    for rendered in [&wide, &medium, &narrow] {
        assert!(rendered.contains("LAYOUT DIRECTOR"));
        assert!(rendered.contains("resize-safe"));
        assert!(rendered.contains("1-6 focus"));
        assert!(rendered.contains("z expand"));
    }
    assert!(wide.contains("visible panes watchlist detail chart book tape status"));
    assert!(wide.contains("hidden panes none"));
    assert!(medium.contains("visible panes watchlist detail chart book tape"));
    assert!(medium.contains("hidden panes status drilldown"));
    assert!(narrow.contains("layout narrow 72x24"));
}

#[test]
fn cockpit_header_renders_terminal_top_command_strip() {
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
    .expect("renders top command strip");

    assert!(rendered.contains("TOP BAR"));
    assert!(rendered.contains("DESK NAV"));
    assert!(rendered.contains("[w/1] WATCH"));
    assert!(rendered.contains("[i/2] DETAIL"));
    assert!(rendered.contains("[c/3] CHART"));
    assert!(rendered.contains("[b/4] BOOK"));
    assert!(rendered.contains("[r/5] TAPE"));
    assert!(rendered.contains("[o/6] OPS"));
    assert!(rendered.contains("SEARCH [/]"));
    assert!(rendered.contains("HELP [?]"));
    assert!(rendered.contains("QUIT [q]"));
    assert!(rendered.contains("EXEC GUARD"));
    assert!(rendered.contains("read-only proxy"));
}

#[test]
fn cockpit_header_renders_selected_quote_rail() {
    let model = RatatuiFrameModel::new(
        fixture_snapshots(),
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
    .expect("renders selected quote rail");

    assert!(rendered.contains("SELECTED QUOTE"));
    assert!(rendered.contains("HYPE/USDC"));
    assert!(rendered.contains("bid share"));
    assert!(rendered.contains("ask share"));
    assert!(rendered.contains("spread 57.1bps"));
    assert!(rendered.contains("top book $"));
    assert!(rendered.contains("public BBO read-only"));
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

fn directional_chart_candles(symbol: &str) -> Vec<CandleEvent> {
    vec![
        CandleEvent {
            recv_ts_ns: 1,
            open_ts_ms: 1_710_000_000_000,
            close_ts_ms: 1_710_000_059_999,
            hl_coin: symbol.to_owned(),
            interval: "1m".to_owned(),
            open: 10.0,
            high: 12.0,
            low: 9.8,
            close: 12.0,
            volume_base: 100.0,
            trade_count: 10,
        },
        CandleEvent {
            recv_ts_ns: 2,
            open_ts_ms: 1_710_000_060_000,
            close_ts_ms: 1_710_000_119_999,
            hl_coin: symbol.to_owned(),
            interval: "1m".to_owned(),
            open: 12.0,
            high: 12.2,
            low: 10.0,
            close: 10.0,
            volume_base: 160.0,
            trade_count: 12,
        },
    ]
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
    assert!(rendered.contains("ALGO SCAN"));
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
    assert!(rendered.contains("TICKER"));
    assert!(rendered.contains("UP HYPE/USDC +0.57%"));
    assert!(rendered.contains("DOWN DOWN/USDC -1.23%"));
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

#[test]
fn wide_status_bar_renders_dynamic_market_ticker_rail() {
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
    .expect("renders wide status ticker");

    assert!(rendered.contains("TICKER"));
    assert!(rendered.contains("UP HYPE/USDC +0.57%"));
    assert!(rendered.contains("DOWN DOWN/USDC -1.23%"));
    assert!(rendered.contains("FLOW DOWN/USDC -$4.2K"));
    assert!(rendered.contains("BREADTH 01/01"));
    assert!(rendered.contains("RISK STRIP"));
    assert!(rendered.contains("conf"));
    assert!(rendered.contains("degraded00"));
    assert!(rendered.contains("net flow -$4.2K"));
    assert!(rendered.contains("ACTION STRIP"));
    assert!(rendered.contains("No wallet"));
}

#[test]
fn wide_status_bar_renders_quality_alert_for_worst_row() {
    let mut snapshots = ten_directional_snapshots();
    snapshots[1].confidence.score = 44;
    snapshots[1].confidence.level = ConfidenceLevel::Low;
    snapshots[1].staleness_state = StalenessState::Stale;
    snapshots[1].updated_ms_ago = Some(3_400);
    snapshots[2].confidence.score = 68;
    snapshots[2].updated_ms_ago = Some(900);
    let model = RatatuiFrameModel::new(
        snapshots,
        "READ-ONLY Hyperliquid spot live screen",
        ScreenRequest::default(),
        WorkstationUiState::default(),
    )
    .with_status("LIVE", "REC ready", "ws=120 events=300 gaps=0");

    let plain = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 240,
            height: 48,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("plain status quality alert renders");
    let colored = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 240,
            height: 48,
        },
        RatatuiColorMode::Color,
    )
    .expect("colored status quality alert renders");

    assert!(!plain.contains("\u{1b}["));
    assert!(plain.contains("QUALITY ALERT"));
    assert!(plain.contains("conf44"));
    assert!(plain.contains("age 3.4s"));
    assert!(plain.contains("stale"));
    assert_eq!(
        active_fg_before(&colored, "QUALITY ALERT"),
        Some("\u{1b}[38;2;255;77;109m")
    );
    assert_eq!(
        active_fg_before(&colored, "conf44"),
        Some("\u{1b}[38;2;255;77;109m")
    );
}

#[test]
fn wide_status_bar_renders_action_key_rail() {
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
    .expect("renders wide action key rail");

    assert!(rendered.contains("ACTION STRIP"));
    assert!(rendered.contains("j/k row"));
    assert!(rendered.contains("ent detail"));
    assert!(rendered.contains("tab view"));
    assert!(rendered.contains("g symbol"));
    assert!(rendered.contains("z zoom"));
    assert!(rendered.contains("/ filter"));
    assert!(rendered.contains("p preset"));
    assert!(rendered.contains("s sort"));
    assert!(rendered.contains("t win"));
    assert!(rendered.contains("? help"));
    assert!(rendered.contains("q quit"));
    assert!(rendered.contains("THEME plain"));
    assert!(rendered.contains("COLOR plain fallback"));
    assert!(rendered.contains("No wallet"));
    assert!(rendered.contains("RISK STRIP"));
}

#[test]
fn medium_status_bar_compacts_action_and_theme_rails() {
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
            width: 120,
            height: 40,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders medium compact action rail");

    assert!(rendered.contains("ACTION STRIP"));
    assert!(rendered.contains("j/k ent tab g"));
    assert!(rendered.contains("z zoom"));
    assert!(rendered.contains("/ p s t ? q"));
    assert!(rendered.contains("THEME plain"));
    assert!(rendered.contains("COLOR plain fallback"));
    assert!(rendered.contains("--color always"));
    assert!(rendered.contains("No wallet"));
    assert!(rendered.contains("TICKER"));
}

#[test]
fn medium_status_bar_surfaces_compact_quality_alert() {
    let mut snapshots = ten_directional_snapshots();
    snapshots[1].confidence.score = 44;
    snapshots[1].confidence.level = ConfidenceLevel::Low;
    snapshots[1].staleness_state = StalenessState::Stale;
    snapshots[1].updated_ms_ago = Some(3_400);
    let model = RatatuiFrameModel::new(
        snapshots,
        "READ-ONLY Hyperliquid spot live screen",
        ScreenRequest::default(),
        WorkstationUiState::default(),
    )
    .with_status("LIVE", "REC ready", "ws=120 events=300 gaps=0");

    let plain = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 120,
            height: 40,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("plain medium status quality alert renders");
    let colored = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 120,
            height: 40,
        },
        RatatuiColorMode::Color,
    )
    .expect("colored medium status quality alert renders");

    assert!(!plain.contains("\u{1b}["));
    assert!(plain.contains("QALERT"));
    assert!(plain.contains("conf44"));
    assert!(plain.contains("TICKER"));
    assert!(plain.contains("ACTION STRIP"));
    assert!(plain.contains("No wallet"));
    assert_eq!(
        active_fg_before(&colored, "QALERT"),
        Some("\u{1b}[38;2;255;77;109m")
    );
    assert_eq!(
        active_fg_before(&colored, "conf44"),
        Some("\u{1b}[38;2;255;77;109m")
    );
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
    assert!(rendered.contains("HT"));
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

    assert!(rendered.contains("[FOCUS] WATCHLIST 10/10 ALGO SCAN VIEW 05-10"));
    assert!(rendered.contains(">10"));
    assert!(rendered.contains("ROW10/USDC"));
    assert!(!rendered.contains("01 HYPE/USDC"));
    assert!(rendered.contains("DETAIL"));
}

#[test]
fn watchlist_keeps_symbol_jump_selected_after_live_rank_reorder() {
    let mut snapshots = directional_snapshots();
    let mut state = WorkstationUiState::default();
    state.select_symbol("DOWN/USDC", 1, snapshots.len());
    snapshots.swap(0, 1);
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
    .expect("renders symbol-pinned selection after reorder");

    assert!(rendered.contains("[FOCUS] WATCHLIST 1/2"));
    assert!(rendered.contains(">01"));
    assert!(rendered.contains("DOWN/USDC"));
    assert!(!rendered.contains("[FOCUS] WATCHLIST 2/2"));
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
    assert!(rendered.contains("HEAT"));
    assert!(rendered.contains("BIAS"));
    assert!(rendered.contains("SPR"));
    assert!(rendered.contains("57.1"));
    assert!(rendered.contains("MOM+"));
    assert!(rendered.contains("██"));
    assert!(rendered.contains("13"));
}

#[test]
fn market_board_quality_view_renders_pair_quality_columns() {
    let snapshots = directional_snapshots();
    let mut state = WorkstationUiState::default();
    state.apply(WorkstationAction::NextView, snapshots.len());
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
            width: 240,
            height: 48,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders quality market board");

    assert!(rendered.contains("view:quality"));
    assert!(rendered.contains("QUALITY SCAN"));
    assert!(rendered.contains("CONF"));
    assert!(rendered.contains("FRESH"));
    assert!(rendered.contains("TRADE"));
    assert!(rendered.contains("RISK"));
    assert!(rendered.contains("DEPTH"));
    assert!(rendered.contains("HYPE/USDC"));
    assert!(rendered.contains("fresh"));
}

#[test]
fn market_board_explain_view_renders_ranking_reason_columns() {
    let snapshots = directional_snapshots();
    let mut state = WorkstationUiState::default();
    for _ in 0..4 {
        state.apply(WorkstationAction::NextView, snapshots.len());
    }
    let model = RatatuiFrameModel::new(
        snapshots,
        "READ-ONLY Hyperliquid spot live screen",
        ScreenRequest::default(),
        state,
    );

    let rendered = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 240,
            height: 48,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders explain market board");

    assert!(rendered.contains("view:explain"));
    assert!(rendered.contains("EXPLAIN SCAN"));
    assert!(rendered.contains("LIQ"));
    assert!(rendered.contains("MOM"));
    assert!(rendered.contains("MEAN"));
    assert!(rendered.contains("WHY"));
    assert!(rendered.contains("HYPE/USDC"));
    assert!(rendered.contains("mom+"));
}

#[test]
fn wide_market_board_renders_public_candle_sparkline_lane() {
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
            width: 240,
            height: 48,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders wide market board with sparklines");

    assert!(rendered.contains("1m spark"));
    assert!(rendered.contains("SPK"));
    assert!(rendered.contains("HYPE/USDC"));
    assert!(
        ["▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"]
            .iter()
            .any(|glyph| rendered.contains(glyph))
    );
}

#[test]
fn wide_watchlist_renders_selected_row_router_strip() {
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
            width: 240,
            height: 48,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders selected row router");

    assert!(rendered.contains("ROW ROUTER"));
    assert!(rendered.contains("selected HYPE/USDC"));
    assert!(rendered.contains("spr 57.1bps"));
    assert!(rendered.contains("flow -$35"));
    assert!(rendered.contains("trade unknown"));
    assert!(rendered.contains("quality Q"));
    assert!(rendered.contains("j/k move"));
    assert!(rendered.contains("tab detail"));
    assert!(rendered.contains("ROW ACTION MAP"));
    assert!(rendered.contains("enter detail"));
    assert!(rendered.contains("c/3 chart"));
    assert!(rendered.contains("b/4 book"));
    assert!(rendered.contains("r/5 tape"));
    assert!(rendered.contains("o/6 ops"));
    assert!(rendered.contains("/ filter"));
    assert!(rendered.contains("z expand"));
    assert!(rendered.contains("display only"));
    assert!(rendered.contains("read-only row context"));
}

#[test]
fn wide_watchlist_renders_dynamic_scanner_rail() {
    let mut snapshots = directional_snapshots();
    snapshots[0].tob_depth_usd = Some(245.0);
    snapshots[1].tob_depth_usd = Some(8_800.0);
    let model = RatatuiFrameModel::new(
        snapshots,
        "READ-ONLY Hyperliquid spot live screen",
        ScreenRequest::default(),
        WorkstationUiState::default(),
    )
    .with_candles(fixture_candles());

    let rendered = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 240,
            height: 48,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders scanner rail");

    assert!(rendered.contains("SCANNER RAIL"));
    assert!(rendered.contains("selected HYPE/USDC"));
    assert!(rendered.contains("mover DOWN/USDC DN-1.23%"));
    assert!(rendered.contains("flow DOWN/USDC -$4.2K"));
    assert!(rendered.contains("depth DOWN/USDC $8.8K"));
    assert!(rendered.contains("read-only scan"));
}

#[test]
fn expanded_watchlist_renders_market_heatmap_deck() {
    let mut snapshots = directional_snapshots();
    snapshots[0].tob_depth_usd = Some(245.0);
    snapshots[1].tob_depth_usd = Some(8_800.0);
    let mut state = WorkstationUiState::default();
    state.apply(
        WorkstationAction::FocusPane(WorkstationPane::Watchlist),
        snapshots.len(),
    );
    state.apply(WorkstationAction::TogglePaneZoom, snapshots.len());
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
            width: 180,
            height: 48,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders expanded watchlist heatmap deck");

    assert!(rendered.contains("EXPANDED watchlist"));
    assert!(rendered.contains("MARKET HEATMAP"));
    assert!(rendered.contains("breadth 01/01"));
    assert!(rendered.contains("heat ██░░"));
    assert!(rendered.contains("top mover DOWN/USDC"));
    assert!(rendered.contains("top flow DOWN/USDC -$4.2K"));
    assert!(rendered.contains("read-only scan"));
}

#[test]
fn expanded_watchlist_renders_command_center_deck() {
    let mut snapshots = directional_snapshots();
    snapshots[0].tob_depth_usd = Some(245.0);
    snapshots[1].tob_depth_usd = Some(8_800.0);
    snapshots[1].confidence.score = 55;
    snapshots[1].confidence.level = ConfidenceLevel::Low;
    let mut state = WorkstationUiState::default();
    state.apply(
        WorkstationAction::FocusPane(WorkstationPane::Watchlist),
        snapshots.len(),
    );
    state.apply(WorkstationAction::TogglePaneZoom, snapshots.len());
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
            width: 190,
            height: 52,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders expanded watchlist command center");

    assert!(rendered.contains("EXPANDED watchlist"));
    assert!(rendered.contains("WATCHLIST COMMAND CENTER"));
    assert!(rendered.contains("selected HYPE/USDC"));
    assert!(rendered.contains("visible 02"));
    assert!(rendered.contains("tradeable"));
    assert!(rendered.contains("degraded"));
    assert!(rendered.contains("hotkeys j/k ent tab w/i/c/b/r/o"));
    assert!(rendered.contains("leaders mover DOWN/USDC"));
    assert!(rendered.contains("flow DOWN/USDC -$4.2K"));
    assert!(rendered.contains("depth DOWN/USDC $8.8K"));
    assert!(rendered.contains("read-only scanner"));
    assert!(rendered.contains("no wallet"));
    assert!(rendered.contains("no orders"));
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
fn watchlist_color_mode_keeps_selected_row_semantic_cell_colors() {
    let model = RatatuiFrameModel::new(
        directional_snapshots(),
        "READ-ONLY Hyperliquid spot live screen",
        ScreenRequest::default(),
        WorkstationUiState::default(),
    );

    let plain = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 240,
            height: 48,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("plain watchlist renders");
    let colored = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 240,
            height: 48,
        },
        RatatuiColorMode::Color,
    )
    .expect("colored watchlist renders");

    assert!(!plain.contains("\u{1b}["));
    assert!(plain.contains(">01"));
    assert!(plain.contains("UP+0.57%"));
    assert!(plain.contains("DN-1.23%"));
    assert!(colored.contains("\u{1b}[48;2;0;95;73m"));
    assert!(colored.contains("\u{1b}[38;2;0;255;154mUP+0.57"));
    assert!(colored.contains("\u{1b}[38;2;255;77;109m-$35"));
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
    assert!(rendered.contains("MARKET PULSE"));
    assert!(rendered.contains("breadth 01/01"));
    assert!(rendered.contains("regime mixed"));
    assert!(rendered.contains("pulse ██░░"));
    assert!(rendered.contains("move DOWN/USDC DN-1.23%"));
    assert!(rendered.contains("flow DOWN/USDC -$4.2K"));
    assert!(rendered.contains("public rows"));
}

#[test]
fn market_pulse_uses_display_symbols_for_metadata_backed_rows() {
    let mut snapshots = fixture_snapshots();
    snapshots[0].ret_1m = Some(0.01);
    snapshots[0].signed_notional_flow_30s = Some(12_345.0);
    let model = RatatuiFrameModel::new(
        snapshots,
        "READ-ONLY Hyperliquid spot live screen",
        ScreenRequest::default(),
        WorkstationUiState::default(),
    );

    let rendered = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 180,
            height: 40,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders market pulse display symbols");

    assert!(rendered.contains("MARKET PULSE"));
    assert!(rendered.contains("move HYPE/USDC"));
    assert!(rendered.contains("flow HYPE/USDC"));
    assert!(!rendered.contains("move @107"));
    assert!(!rendered.contains("flow @107"));
}

#[test]
fn market_pulse_renders_pipeline_freshness_hud() {
    let mut snapshots = directional_snapshots();
    snapshots[0].updated_ms_ago = Some(120);
    snapshots[1].updated_ms_ago = Some(2_400);
    snapshots[1].staleness_state = StalenessState::Stale;
    snapshots[1].confidence.score = 55;
    snapshots[1].confidence.level = ConfidenceLevel::Low;
    let model = RatatuiFrameModel::new(
        snapshots,
        "READ-ONLY Hyperliquid spot live screen",
        ScreenRequest::default(),
        WorkstationUiState::default(),
    )
    .with_status("LIVE", "REC ready", "ws=235 events=485 reconnects=2 gaps=1");

    let plain = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 240,
            height: 48,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("plain market pulse pipeline HUD renders");
    let colored = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 240,
            height: 48,
        },
        RatatuiColorMode::Color,
    )
    .expect("colored market pulse pipeline HUD renders");

    assert!(!plain.contains("\u{1b}["));
    assert!(plain.contains("MARKET PULSE"));
    assert!(plain.contains("PIPELINE"));
    assert!(plain.contains("p95 2.4s"));
    assert!(plain.contains("re 2"));
    assert!(plain.contains("gaps 1"));
    assert_eq!(
        active_fg_before(&colored, "PIPELINE"),
        Some("\u{1b}[38;2;0;229;255m")
    );
    assert_eq!(
        active_fg_before(&colored, "p95 2.4s"),
        Some("\u{1b}[38;2;255;77;109m")
    );
}

#[test]
fn cockpit_header_renders_interactive_desk_tab_rail() {
    let snapshots = directional_snapshots();
    let mut state = WorkstationUiState::default();
    state.apply(
        WorkstationAction::FocusPane(WorkstationPane::Chart),
        snapshots.len(),
    );
    state.apply(WorkstationAction::NextView, snapshots.len());
    state.apply(WorkstationAction::ToggleDensity, snapshots.len());
    let model = RatatuiFrameModel::new(
        snapshots,
        "READ-ONLY Hyperliquid spot live screen",
        ScreenRequest::default(),
        state,
    );

    let rendered = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 180,
            height: 48,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders interactive desk tab rail");

    assert!(rendered.contains("DESK"));
    assert!(rendered.contains("WATCHLIST 1"));
    assert!(rendered.contains("DETAIL 2"));
    assert!(rendered.contains("[CHART 3]"));
    assert!(rendered.contains("BOOK 4"));
    assert!(rendered.contains("TAPE 5"));
    assert!(rendered.contains("OPS 6"));
    assert!(rendered.contains("view flow"));
    assert!(rendered.contains("density dense"));
    assert!(rendered.contains("EXEC GUARD"));
    assert!(rendered.contains("read-only"));
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
fn detail_panel_renders_selected_pair_alpha_risk_stack() {
    let snapshots = fixture_snapshots();
    let mut state = WorkstationUiState::default();
    state.apply(
        WorkstationAction::FocusPane(WorkstationPane::Detail),
        snapshots.len(),
    );
    state.apply(WorkstationAction::TogglePaneZoom, snapshots.len());
    let model = RatatuiFrameModel::new(
        snapshots,
        "READ-ONLY Hyperliquid spot live screen",
        ScreenRequest::default(),
        state,
    );

    let rendered = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 180,
            height: 48,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders selected-pair alpha risk stack");

    assert!(rendered.contains("EXPANDED detail"));
    assert!(rendered.contains("ALPHA STACK"));
    assert!(rendered.contains("signal"));
    assert!(rendered.contains("cost"));
    assert!(rendered.contains("risk"));
    assert!(rendered.contains("SCREEN ONLY"));
    assert!(rendered.contains("no orders"));
}

#[test]
fn expanded_detail_renders_quote_terminal_deck() {
    let snapshots = fixture_snapshots();
    let mut state = WorkstationUiState::default();
    state.apply(
        WorkstationAction::FocusPane(WorkstationPane::Detail),
        snapshots.len(),
    );
    state.apply(WorkstationAction::TogglePaneZoom, snapshots.len());
    let model = RatatuiFrameModel::new(
        snapshots,
        "READ-ONLY Hyperliquid spot live screen",
        ScreenRequest::default(),
        state,
    );

    let rendered = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 190,
            height: 50,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders expanded quote terminal");

    assert!(rendered.contains("EXPANDED detail"));
    assert!(rendered.contains("QUOTE TERMINAL"));
    assert!(rendered.contains("instrument HYPE/USDC"));
    assert!(rendered.contains("BID 34.9000"));
    assert!(rendered.contains("ASK 35.1000"));
    assert!(rendered.contains("spread 57.1bps"));
    assert!(rendered.contains("top book $"));
    assert!(rendered.contains("FLOW"));
    assert!(rendered.contains("CONF 100"));
    assert!(rendered.contains("public BBO/trades only"));
    assert!(rendered.contains("no orders"));
    assert!(rendered.contains("not advice"));
}

#[test]
fn expanded_detail_renders_instrument_dossier() {
    let snapshots = fixture_snapshots();
    let mut state = WorkstationUiState::default();
    state.apply(
        WorkstationAction::FocusPane(WorkstationPane::Detail),
        snapshots.len(),
    );
    state.apply(WorkstationAction::TogglePaneZoom, snapshots.len());
    let model = RatatuiFrameModel::new(
        snapshots,
        "READ-ONLY Hyperliquid spot live screen",
        ScreenRequest::default(),
        state,
    );

    let rendered = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 190,
            height: 52,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders expanded instrument dossier");

    assert!(rendered.contains("EXPANDED detail"));
    assert!(rendered.contains("INSTRUMENT DOSSIER"));
    assert!(rendered.contains("public metadata"));
    assert!(rendered.contains("cohort"));
    assert!(rendered.contains("tags"));
    assert!(rendered.contains("listing"));
    assert!(rendered.contains("seeded"));
    assert!(rendered.contains("source"));
    assert!(rendered.contains("feed id"));
    assert!(rendered.contains("confidence 100"));
    assert!(rendered.contains("freshness fresh"));
    assert!(rendered.contains("no wallet"));
    assert!(rendered.contains("no orders"));
}

#[test]
fn detail_explain_view_renders_why_ranked_deck() {
    let snapshots = fixture_snapshots();
    let mut state = WorkstationUiState::default();
    for _ in 0..4 {
        state.apply(WorkstationAction::NextView, snapshots.len());
    }
    let model = RatatuiFrameModel::new(
        snapshots,
        "READ-ONLY Hyperliquid spot live screen",
        ScreenRequest::default(),
        state,
    );

    let rendered = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 180,
            height: 48,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders why-ranked deck");

    assert!(rendered.contains("WHY RANKED"));
    assert!(rendered.contains("score explanation"));
    assert!(rendered.contains("SCORE adjusted"));
    assert!(rendered.contains("raw"));
    assert!(rendered.contains("confidence penalty"));
    assert!(rendered.contains("COMPONENTS"));
    assert!(rendered.contains("unavailable evidence"));
    assert!(rendered.contains("BBO/top-of-book proxy only"));
    assert!(rendered.contains("screen heuristic, not advice"));
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
    assert!(rendered.contains("QUOTE STRIP"));
    assert!(rendered.contains("bid 34.9000"));
    assert!(rendered.contains("ask 35.1000"));
    assert!(rendered.contains("mid 35.0000"));
    assert!(rendered.contains("read-only quote"));
    assert!(rendered.contains("spread cost"));
    assert!(rendered.contains("depth"));
    assert!(rendered.contains("imbalance"));
    assert!(rendered.contains("flow"));
    assert!(rendered.contains("Public BBO/flow only"));
    assert!(rendered.contains("█"));
}

#[test]
fn wide_detail_panel_renders_selected_pair_snapshot() {
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
    .expect("renders selected pair snapshot");

    assert!(rendered.contains("PAIR SNAPSHOT"));
    assert!(rendered.contains("trade unknown"));
    assert!(rendered.contains("resilience unknown"));
    assert!(rendered.contains("freshness fresh"));
    assert!(rendered.contains("conf 100"));
    assert!(rendered.contains("read-only selected pair"));
}

#[test]
fn detail_overview_renders_quote_card() {
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
    .expect("renders detail quote card");

    assert!(rendered.contains("QUOTE CARD"));
    assert!(rendered.contains("QUOTE STRIP"));
    assert!(rendered.contains("HYPE/USDC"));
    assert!(rendered.contains("mid 35.0000"));
    assert!(rendered.contains("spread 57.1 bps"));
    assert!(rendered.contains("PAIR SNAPSHOT"));
    assert!(rendered.contains("freshness fresh"));
    assert!(rendered.contains("read-only quote"));
    assert!(rendered.contains("BID/ASK BALANCE"));
    assert!(rendered.contains("bid share"));
    assert!(rendered.contains("ask share"));
    assert!(rendered.contains("public BBO only"));
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
    assert!(rendered.contains("j/k ent /pstzh? q"));
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
    assert!(rendered.contains("ACTION chart:t window"));
    assert!(rendered.contains("/ command"));
    assert!(rendered.contains("RO no-wallet"));
}

#[test]
fn narrow_status_bar_surfaces_quality_alert_outside_watchlist_focus() {
    let mut snapshots = fixture_snapshots();
    snapshots[0].confidence.score = 44;
    snapshots[0].confidence.level = ConfidenceLevel::Low;
    snapshots[0].staleness_state = StalenessState::Stale;
    snapshots[0].updated_ms_ago = Some(3_400);
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

    let plain = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 72,
            height: 24,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("plain compact quality rail renders");
    let colored = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 72,
            height: 24,
        },
        RatatuiColorMode::Color,
    )
    .expect("colored compact quality rail renders");

    assert!(!plain.contains("\u{1b}["));
    assert!(plain.contains("ws235 ev485 r0 g0"));
    assert!(plain.contains("QALERT"));
    assert!(plain.contains("conf44"));
    assert!(plain.contains("chart:t"));
    assert!(plain.contains("RO no-wallet"));
    assert_eq!(
        active_fg_before(&colored, "QALERT"),
        Some("\u{1b}[38;2;255;77;109m")
    );
    assert_eq!(
        active_fg_before(&colored, "conf44"),
        Some("\u{1b}[38;2;255;77;109m")
    );
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
    assert!(rendered.contains("w/i/c/b/r/o"));
    assert!(rendered.contains("j/k ent /pstzh? q"));
    assert!(rendered.contains("[FOCUS] BOOK"));
}

#[test]
fn wide_cockpit_expands_focused_chart_pane() {
    let snapshots = fixture_snapshots();
    let mut state = WorkstationUiState::default();
    state.apply(
        WorkstationAction::FocusPane(WorkstationPane::Chart),
        snapshots.len(),
    );
    state.apply(WorkstationAction::TogglePaneZoom, snapshots.len());
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
    .expect("renders expanded chart pane");

    assert!(rendered.contains("zoom:chart"));
    assert!(rendered.contains("EXPANDED chart"));
    assert!(rendered.contains("z grid"));
    assert!(rendered.contains("CANDLES"));
    assert!(rendered.contains("WINDOWS"));
}

#[test]
fn expanded_chart_renders_public_intelligence_deck() {
    let snapshots = fixture_snapshots();
    let mut state = WorkstationUiState::default();
    state.apply(
        WorkstationAction::FocusPane(WorkstationPane::Chart),
        snapshots.len(),
    );
    state.apply(WorkstationAction::TogglePaneZoom, snapshots.len());
    let model = RatatuiFrameModel::new(
        snapshots,
        "READ-ONLY Hyperliquid spot live screen",
        ScreenRequest::default(),
        state,
    )
    .with_candles(fixture_candles())
    .with_trades(fixture_trades());

    let rendered = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 180,
            height: 52,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders expanded chart intelligence deck");

    assert!(rendered.contains("EXPANDED chart"));
    assert!(rendered.contains("CHART INTEL"));
    assert!(rendered.contains("trend"));
    assert!(rendered.contains("range pos"));
    assert!(rendered.contains("vol pulse"));
    assert!(rendered.contains("public candles + prints only"));
    assert!(rendered.contains("no orders"));
}

#[test]
fn expanded_chart_renders_semantic_zoom_deck() {
    let snapshots = fixture_snapshots();
    let mut state = WorkstationUiState::default();
    state.apply(
        WorkstationAction::FocusPane(WorkstationPane::Chart),
        snapshots.len(),
    );
    state.apply(WorkstationAction::TogglePaneZoom, snapshots.len());
    let model = RatatuiFrameModel::new(
        snapshots,
        "READ-ONLY Hyperliquid spot live screen",
        ScreenRequest::default(),
        state,
    )
    .with_candles(fixture_candles());

    let plain = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 120,
            height: 36,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("plain expanded zoom deck renders");
    let colored = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 120,
            height: 36,
        },
        RatatuiColorMode::Color,
    )
    .expect("colored expanded zoom deck renders");

    assert!(!plain.contains("\u{1b}["));
    assert!(plain.contains("ZOOM DECK"));
    assert!(plain.contains("EXPANDED chart"));
    assert!(plain.contains("z grid"));
    assert!(plain.contains("1-6 focus"));
    assert!(plain.contains("/ command"));
    assert!(plain.contains("READ-ONLY"));
    assert_eq!(
        active_fg_before(&colored, "ZOOM DECK"),
        Some("\u{1b}[38;2;0;229;255m")
    );
    assert_eq!(
        active_fg_before(&colored, "EXPANDED chart"),
        Some("\u{1b}[38;2;255;209;102m")
    );
}

#[test]
fn expanded_chart_renders_tactical_matrix() {
    let snapshots = directional_snapshots();
    let mut state = WorkstationUiState::default();
    state.apply(
        WorkstationAction::FocusPane(WorkstationPane::Chart),
        snapshots.len(),
    );
    state.apply(WorkstationAction::TogglePaneZoom, snapshots.len());
    state.apply(WorkstationAction::CycleChartWindow, snapshots.len());
    let model = RatatuiFrameModel::new(
        snapshots,
        "READ-ONLY Hyperliquid spot live screen",
        ScreenRequest::default(),
        state,
    )
    .with_candles(directional_chart_candles("@107"))
    .with_trades(fixture_trades());

    let rendered = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 190,
            height: 54,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders expanded chart tactical matrix");

    assert!(rendered.contains("EXPANDED chart"));
    assert!(rendered.contains("TACTICAL MATRIX"));
    assert!(rendered.contains("window 30m"));
    assert!(rendered.contains("regime"));
    assert!(rendered.contains("trend"));
    assert!(rendered.contains("volatility"));
    assert!(rendered.contains("liquidity gate"));
    assert!(rendered.contains("flow gate"));
    assert!(rendered.contains("confidence 100"));
    assert!(rendered.contains("public candles/BBO/trades only"));
    assert!(rendered.contains("no orders"));
    assert!(rendered.contains("not advice"));
}

#[test]
fn narrow_cockpit_expands_focused_book_pane() {
    let snapshots = fixture_snapshots();
    let mut state = WorkstationUiState::default();
    state.apply(
        WorkstationAction::FocusPane(WorkstationPane::Book),
        snapshots.len(),
    );
    state.apply(WorkstationAction::TogglePaneZoom, snapshots.len());
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
    .expect("renders expanded book pane");

    assert!(rendered.contains("z:book"));
    assert!(rendered.contains("EXPANDED book"));
    assert!(rendered.contains("z grid"));
    assert!(rendered.contains("BOOK"));
    assert!(rendered.contains("BID"));
    assert!(rendered.contains("ASK"));
}

#[test]
fn expanded_book_renders_depth_map_drilldown() {
    let snapshots = fixture_snapshots();
    let mut state = WorkstationUiState::default();
    state.apply(
        WorkstationAction::FocusPane(WorkstationPane::Book),
        snapshots.len(),
    );
    state.apply(WorkstationAction::TogglePaneZoom, snapshots.len());
    let model = RatatuiFrameModel::new(
        snapshots,
        "READ-ONLY Hyperliquid spot live screen",
        ScreenRequest::default(),
        state,
    );

    let rendered = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 180,
            height: 48,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders expanded book depth map");

    assert!(rendered.contains("EXPANDED book"));
    assert!(rendered.contains("DEPTH MAP"));
    assert!(rendered.contains("bid wall"));
    assert!(rendered.contains("ask wall"));
    assert!(rendered.contains("queue skew"));
    assert!(rendered.contains("spread gate"));
    assert!(rendered.contains("public top-book only"));
    assert!(rendered.contains("no orders"));
}

#[test]
fn expanded_book_renders_liquidity_wall_monitor() {
    let snapshots = fixture_snapshots();
    let mut state = WorkstationUiState::default();
    state.apply(
        WorkstationAction::FocusPane(WorkstationPane::Book),
        snapshots.len(),
    );
    state.apply(WorkstationAction::TogglePaneZoom, snapshots.len());
    let model = RatatuiFrameModel::new(
        snapshots,
        "READ-ONLY Hyperliquid spot live screen",
        ScreenRequest::default(),
        state,
    );

    let rendered = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 190,
            height: 52,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders expanded book liquidity wall monitor");

    assert!(rendered.contains("EXPANDED book"));
    assert!(rendered.contains("LIQUIDITY WALL"));
    assert!(rendered.contains("bid share"));
    assert!(rendered.contains("ask share"));
    assert!(rendered.contains("spread 57.1bps"));
    assert!(rendered.contains("OFI"));
    assert!(rendered.contains("micro edge"));
    assert!(rendered.contains("public BBO only"));
    assert!(rendered.contains("no L2 reconstruction"));
    assert!(rendered.contains("no orders"));
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
fn medium_cockpit_renders_lower_pane_keyboard_router() {
    let model = RatatuiFrameModel::new(
        fixture_snapshots(),
        "READ-ONLY Hyperliquid spot live screen",
        ScreenRequest::default(),
        WorkstationUiState::default(),
    )
    .with_candles(fixture_candles())
    .with_trades(fixture_trades());

    let plain = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 120,
            height: 36,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("plain medium lower pane router renders");
    let colored = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 120,
            height: 36,
        },
        RatatuiColorMode::Color,
    )
    .expect("colored medium lower pane router renders");

    assert!(!plain.contains("\u{1b}["));
    assert!(plain.contains("ADAPTIVE DESK"));
    assert!(plain.contains("4 book"));
    assert!(plain.contains("5 tape"));
    assert!(plain.contains("public BBO/trades only"));
    assert!(plain.contains("z zoom"));
    assert_eq!(
        active_fg_before(&colored, "ADAPTIVE DESK"),
        Some("\u{1b}[38;2;0;229;255m")
    );
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
    assert!(rendered.contains("DEPTH CONSOLE"));
    assert!(rendered.contains("bid pressure"));
    assert!(rendered.contains("ask pressure"));
    assert!(rendered.contains("BID notional"));
    assert!(rendered.contains("ASK notional"));
    assert!(rendered.contains("BBO depth proxy"));
    assert!(rendered.contains("BOOK proxy only"));
}

#[test]
fn book_pane_renders_pressure_tape() {
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
            width: 160,
            height: 48,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders book pressure tape");

    assert!(rendered.contains("PRESSURE TAPE"));
    assert!(rendered.contains("BOOK SNAP"));
    assert!(rendered.contains("bid share"));
    assert!(rendered.contains("ask share"));
    assert!(rendered.contains("queue skew"));
    assert!(rendered.contains("read-only top-book"));
}

#[test]
fn book_pane_renders_read_only_bbo_ladder() {
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
            width: 320,
            height: 48,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders bbo ladder");

    assert!(rendered.contains("BBO LADDER"));
    assert!(rendered.contains("bid 34.9000"));
    assert!(rendered.contains("mid 35.0000"));
    assert!(rendered.contains("ask 35.1000"));
    assert!(rendered.contains("spr 57.1bps"));
    assert!(rendered.contains("read-only BBO"));
    assert!(rendered.contains("MICROPRICE"));
    assert!(rendered.contains("queue skew"));
    assert!(rendered.contains("read-only top-book model"));
}

#[test]
fn book_pane_color_mode_renders_semantic_depth_lens() {
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

    let plain = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 320,
            height: 48,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("plain book depth lens renders");
    let colored = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 320,
            height: 48,
        },
        RatatuiColorMode::Color,
    )
    .expect("colored book depth lens renders");

    assert!(!plain.contains("\u{1b}["));
    assert!(plain.contains("DEPTH LENS"));
    assert!(plain.contains("read-only top-book"));
    assert!(colored.contains("DEPTH LENS"));
    assert!(colored.contains("\u{1b}[38;2;0;255;154mbid █"));
    assert!(colored.contains("\u{1b}[38;2;255;77;109mask █"));
}

#[test]
fn book_pane_renders_execution_quality_band() {
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
            width: 320,
            height: 48,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders execution quality band");

    assert!(rendered.contains("[FOCUS] BOOK"));
    assert!(rendered.contains("EXEC QUALITY"));
    assert!(rendered.contains("spread 57.1bps"));
    assert!(rendered.contains("depth $245"));
    assert!(rendered.contains("edge"));
    assert!(rendered.contains("trade unknown"));
    assert!(rendered.contains("read-only"));
}

#[test]
fn wide_book_pane_renders_queue_share_snap() {
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
            width: 240,
            height: 48,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders wide book snap");

    assert!(rendered.contains("[FOCUS] BOOK"));
    assert!(rendered.contains("BOOK SNAP"));
    assert!(rendered.contains("share bid"));
    assert!(rendered.contains("ask share"));
    assert!(rendered.contains("queue map"));
    assert!(rendered.contains("read-only top-book"));
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
fn book_pane_quality_view_renders_read_only_liquidity_evidence() {
    let snapshots = fixture_snapshots();
    let mut state = WorkstationUiState::default();
    state.apply(
        WorkstationAction::FocusPane(WorkstationPane::Book),
        snapshots.len(),
    );
    state.apply(WorkstationAction::NextView, snapshots.len());
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
            width: 240,
            height: 48,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders book quality mode");

    assert!(rendered.contains("view:quality"));
    assert!(rendered.contains("[FOCUS] BOOK"));
    assert!(rendered.contains("BOOK QUALITY MODE"));
    assert!(rendered.contains("confidence"));
    assert!(rendered.contains("freshness"));
    assert!(rendered.contains("tradeability"));
    assert!(rendered.contains("resilience"));
    assert!(rendered.contains("spread gate"));
    assert!(rendered.contains("depth gate"));
    assert!(rendered.contains("queue evidence"));
    assert!(rendered.contains("read-only BBO evidence"));
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
            width: 240,
            height: 48,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders flow tape");

    assert!(rendered.contains("[FOCUS] TAPE"));
    assert!(rendered.contains("TAPE RAIL"));
    assert!(rendered.contains("Selected flow"));
    assert!(rendered.contains("FLOW pulse"));
    assert!(rendered.contains("net pressure"));
    assert!(rendered.contains("FLOW SPECTRUM"));
    assert!(rendered.contains("buy pressure"));
    assert!(rendered.contains("sell pressure"));
    assert!(rendered.contains("read-only public flow"));
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
    assert!(rendered.contains("TAPE RADAR"));
    assert!(rendered.contains("prints 2"));
    assert!(rendered.contains("buy 1"));
    assert!(rendered.contains("sell 1"));
    assert!(rendered.contains("public tape"));
    assert!(rendered.contains("TAPE VELOCITY"));
    assert!(rendered.contains("prints/s 0.03"));
    assert!(rendered.contains("notional/s $2"));
    assert!(rendered.contains("max $70"));
    assert!(rendered.contains("public"));
    assert!(rendered.contains("LAST TRADE HUD"));
    assert!(rendered.contains("latest SELL"));
    assert!(rendered.contains("px 35.2000"));
    assert!(rendered.contains("size 1"));
    assert!(rendered.contains("notional $35"));
    assert!(rendered.contains("tid 12"));
    assert!(rendered.contains("public trades only"));
    assert!(rendered.contains("PUBLIC TRADES"));
    assert!(rendered.contains("BUY"));
    assert!(rendered.contains("SELL"));
    assert!(rendered.contains("notional"));
    assert!(rendered.contains("Public trades only | no fills"));
}

#[test]
fn expanded_tape_renders_time_and_sales_board() {
    let snapshots = fixture_snapshots();
    let mut state = WorkstationUiState::default();
    state.apply(
        WorkstationAction::FocusPane(WorkstationPane::Tape),
        snapshots.len(),
    );
    state.apply(WorkstationAction::TogglePaneZoom, snapshots.len());
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
            width: 180,
            height: 48,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders expanded time and sales board");

    assert!(rendered.contains("EXPANDED tape"));
    assert!(rendered.contains("TIME & SALES"));
    assert!(rendered.contains("burst"));
    assert!(rendered.contains("side mix"));
    assert!(rendered.contains("largest $70"));
    assert!(rendered.contains("public prints only"));
    assert!(rendered.contains("no fills"));
}

#[test]
fn expanded_tape_renders_public_print_ladder() {
    let snapshots = fixture_snapshots();
    let mut state = WorkstationUiState::default();
    state.apply(
        WorkstationAction::FocusPane(WorkstationPane::Tape),
        snapshots.len(),
    );
    state.apply(WorkstationAction::TogglePaneZoom, snapshots.len());
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
            width: 190,
            height: 52,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders expanded public print ladder");

    assert!(rendered.contains("EXPANDED tape"));
    assert!(rendered.contains("PUBLIC PRINT LADDER"));
    assert!(rendered.contains("price levels"));
    assert!(rendered.contains("buy level"));
    assert!(rendered.contains("sell level"));
    assert!(rendered.contains("largest print"));
    assert!(rendered.contains("toxicity proxy"));
    assert!(rendered.contains("public trades only"));
    assert!(rendered.contains("no fills"));
    assert!(rendered.contains("no orders"));
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
    assert!(rendered.contains("TRADE FLOW MODE Public trades"));
    assert!(rendered.contains("only | no fills"));
}

#[test]
fn tape_pane_quality_view_renders_public_print_diagnostics() {
    let snapshots = fixture_snapshots();
    let mut state = WorkstationUiState::default();
    state.apply(
        WorkstationAction::FocusPane(WorkstationPane::Tape),
        snapshots.len(),
    );
    state.apply(WorkstationAction::NextView, snapshots.len());
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
    .expect("renders public print diagnostics");

    assert!(rendered.contains("view:quality"));
    assert!(rendered.contains("[FOCUS] TAPE"));
    assert!(rendered.contains("TAPE QUALITY MODE"));
    assert!(rendered.contains("public print diagnostics"));
    assert!(rendered.contains("prints 2"));
    assert!(rendered.contains("buy 1"));
    assert!(rendered.contains("sell 1"));
    assert!(rendered.contains("confidence"));
    assert!(rendered.contains("freshness"));
    assert!(rendered.contains("flow gate"));
    assert!(rendered.contains("tradeability"));
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
    assert!(rendered.contains("OPS RADAR"));
    assert!(rendered.contains("WS load"));
    assert!(rendered.contains("EVENT flow"));
    assert!(rendered.contains("QUALITY MATRIX"));
    assert!(rendered.contains("tradeable"));
    assert!(rendered.contains("degraded"));
    assert!(rendered.contains("stale"));
    assert!(rendered.contains("confidence"));
    assert!(rendered.contains("SAFETY GATES"));
    assert!(rendered.contains("no orders"));
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
    assert!(rendered.contains("ent detail h health"));
    assert!(rendered.contains("No wallet"));
    assert!(!rendered.contains("[FOCUS] DETAIL"));
}

#[test]
fn wide_status_focus_renders_market_regime_board() {
    let mut snapshots = directional_snapshots();
    snapshots[0].tob_depth_usd = Some(1_200.0);
    snapshots[1].tob_depth_usd = Some(8_800.0);
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
            width: 240,
            height: 48,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders wide status regime board");

    assert!(rendered.contains("[FOCUS] STATUS"));
    assert!(rendered.contains("REGIME BOARD"));
    assert!(rendered.contains("portfolio scan read-only"));
    assert!(rendered.contains("regime mixed"));
    assert!(rendered.contains("breadth 01/01"));
    assert!(rendered.contains("heat ██░░"));
    assert!(rendered.contains("net flow -$4.2K"));
    assert!(rendered.contains("depth $10.0K"));
    assert!(rendered.contains("avg conf 100"));
    assert!(rendered.contains("status ? help"));
    assert!(rendered.contains("No wallet"));
}

#[test]
fn expanded_status_renders_ops_command_center() {
    let mut snapshots = directional_snapshots();
    snapshots[0].tob_depth_usd = Some(1_200.0);
    snapshots[1].confidence.score = 55;
    snapshots[1].confidence.level = ConfidenceLevel::Low;
    let mut state = WorkstationUiState::default();
    state.apply(
        WorkstationAction::FocusPane(WorkstationPane::Status),
        snapshots.len(),
    );
    state.apply(WorkstationAction::TogglePaneZoom, snapshots.len());
    let model = RatatuiFrameModel::new(
        snapshots,
        "READ-ONLY Hyperliquid spot live screen",
        ScreenRequest::default(),
        state,
    )
    .with_status("LIVE", "REC ready", "ws=235 events=485 reconnects=2 gaps=1");

    let rendered = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 180,
            height: 48,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders expanded ops command center");

    assert!(rendered.contains("EXPANDED status"));
    assert!(rendered.contains("OPS COMMAND CENTER"));
    assert!(rendered.contains("ingest gate"));
    assert!(rendered.contains("recorder REC ready"));
    assert!(rendered.contains("telemetry ws 235 events 485"));
    assert!(rendered.contains("risk gate"));
    assert!(rendered.contains("degraded 01"));
    assert!(rendered.contains("No wallet"));
    assert!(rendered.contains("no orders"));
}

#[test]
fn expanded_status_renders_portfolio_risk_terminal() {
    let mut snapshots = ten_directional_snapshots();
    snapshots[0].tob_depth_usd = Some(2_000.0);
    snapshots[1].tob_depth_usd = Some(8_000.0);
    snapshots[1].confidence.score = 55;
    snapshots[1].confidence.level = ConfidenceLevel::Low;
    snapshots[1].staleness_state = StalenessState::Stale;
    let mut state = WorkstationUiState::default();
    state.apply(
        WorkstationAction::FocusPane(WorkstationPane::Status),
        snapshots.len(),
    );
    state.apply(WorkstationAction::TogglePaneZoom, snapshots.len());
    let model = RatatuiFrameModel::new(
        snapshots,
        "READ-ONLY Hyperliquid spot live screen",
        ScreenRequest::default(),
        state,
    )
    .with_status("LIVE", "REC ready", "ws=235 events=485 reconnects=1 gaps=0");

    let rendered = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 190,
            height: 52,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders expanded portfolio risk terminal");

    assert!(rendered.contains("EXPANDED status"));
    assert!(rendered.contains("PORTFOLIO RISK TERMINAL"));
    assert!(rendered.contains("screen exposure only"));
    assert!(rendered.contains("no positions"));
    assert!(rendered.contains("no orders"));
    assert!(rendered.contains("up screens"));
    assert!(rendered.contains("down screens"));
    assert!(rendered.contains("flow skew"));
    assert!(rendered.contains("concentration top"));
    assert!(rendered.contains("public top-book depth proxy"));
    assert!(rendered.contains("risk stack degraded"));
    assert!(rendered.contains("stale 01"));
    assert!(rendered.contains("not advice"));
}

#[test]
fn expanded_status_renders_color_lab_diagnostics() {
    let snapshots = fixture_snapshots();
    let mut state = WorkstationUiState::default();
    state.apply(
        WorkstationAction::FocusPane(WorkstationPane::Status),
        snapshots.len(),
    );
    state.apply(WorkstationAction::TogglePaneZoom, snapshots.len());
    let model = RatatuiFrameModel::new(
        snapshots,
        "READ-ONLY Hyperliquid spot live screen",
        ScreenRequest::default(),
        state,
    )
    .with_status("LIVE", "REC ready", "ws=235 events=485 reconnects=0 gaps=0");

    let plain = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 170,
            height: 42,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders no-color status lab");
    let colored = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 170,
            height: 42,
        },
        RatatuiColorMode::Color,
    )
    .expect("renders colored status lab");

    assert!(!plain.contains("\u{1b}["));
    assert!(plain.contains("COLOR LAB"));
    assert!(plain.contains("mode no-color"));
    assert!(plain.contains("path plain fallback"));
    assert!(plain.contains("fix --color always"));
    assert!(plain.contains("NO_COLOR"));
    assert!(plain.contains("TERM=xterm-256color"));
    assert!(plain.contains("truecolor"));
    assert!(plain.contains("public data only"));
    assert!(colored.contains("COLOR LAB"));
    assert!(colored.contains("mode color"));
    assert!(colored.contains("path ansi-neon active"));
    assert!(colored.contains("\u{1b}[38;2;0;255;154m▲"));
    assert!(colored.contains("\u{1b}[38;2;255;77;109m▼"));
}

#[test]
fn wide_status_focus_renders_cross_pair_signal_matrix() {
    let mut snapshots = ten_directional_snapshots();
    snapshots[0].tob_depth_usd = Some(1_200.0);
    snapshots[1].tob_depth_usd = Some(8_800.0);
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
            width: 240,
            height: 48,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders wide status signal matrix");

    assert!(rendered.contains("[FOCUS] STATUS"));
    assert!(rendered.contains("SIGNAL MATRIX"));
    assert!(rendered.contains("HYPE:"));
    assert!(rendered.contains("DOWN:"));
    assert!(rendered.contains("L"));
    assert!(rendered.contains("F+"));
    assert!(rendered.contains("F-"));
    assert!(rendered.contains("+2"));
    assert!(rendered.contains("No wallet"));
}

#[test]
fn wide_status_focus_renders_latency_strip() {
    let mut snapshots = directional_snapshots();
    snapshots[0].updated_ms_ago = Some(120);
    snapshots[1].updated_ms_ago = Some(2_400);
    snapshots[1].confidence.score = 55;
    snapshots[1].confidence.level = ConfidenceLevel::Low;
    snapshots[1].staleness_state = StalenessState::Stale;
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
    .with_status("LIVE", "REC ready", "ws=235 events=485 reconnects=2 gaps=1");

    let rendered = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 240,
            height: 48,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders wide status latency strip");

    assert!(rendered.contains("[FOCUS] STATUS"));
    assert!(rendered.contains("LATENCY STRIP"));
    assert!(rendered.contains("p95 row age 2.4s"));
    assert!(rendered.contains("low confidence 01"));
    assert!(rendered.contains("stale 01"));
    assert!(rendered.contains("reconnects 2"));
    assert!(rendered.contains("gaps 1"));
    assert!(rendered.contains("read-only local processing"));
}

#[test]
fn status_focus_renders_data_quality_watch_for_degraded_rows() {
    let mut snapshots = ten_directional_snapshots();
    snapshots[1].confidence.score = 42;
    snapshots[1].confidence.level = ConfidenceLevel::Low;
    snapshots[1].staleness_state = StalenessState::Stale;
    snapshots[1].updated_ms_ago = Some(3_200);
    snapshots[2].confidence.score = 68;
    snapshots[2].staleness_state = StalenessState::Fresh;
    snapshots[2].updated_ms_ago = Some(900);
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
    .with_status("LIVE", "REC ready", "ws=235 events=485 reconnects=2 gaps=1");

    let plain = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 240,
            height: 48,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("plain status quality watch renders");
    let colored = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 240,
            height: 48,
        },
        RatatuiColorMode::Color,
    )
    .expect("colored status quality watch renders");

    assert!(!plain.contains("\u{1b}["));
    assert!(plain.contains("DATA QUALITY WATCH"));
    assert!(plain.contains("stale"));
    assert!(plain.contains("conf42"));
    assert!(plain.contains("age 3.2s"));
    assert!(plain.contains("public rows only"));
    assert_eq!(
        active_fg_before(&colored, "DATA QUALITY WATCH"),
        Some("\u{1b}[38;2;0;229;255m")
    );
    assert_eq!(
        active_fg_before(&colored, "conf42"),
        Some("\u{1b}[38;2;255;77;109m")
    );
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
    assert!(colored.contains("\u{1b}[48;2;8;14;22m"));
    assert!(colored.contains("\u{1b}[48;2;16;24;36m"));
    assert!(colored.contains("WATCHLIST"));
}

#[test]
fn cockpit_color_mode_uses_pane_accented_borders() {
    let model = RatatuiFrameModel::new(
        fixture_snapshots(),
        "READ-ONLY Hyperliquid spot live screen",
        ScreenRequest::default(),
        WorkstationUiState::default(),
    )
    .with_candles(fixture_candles())
    .with_trades(fixture_trades());

    let plain = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 220,
            height: 52,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("plain wide cockpit renders");
    let colored = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 220,
            height: 52,
        },
        RatatuiColorMode::Color,
    )
    .expect("colored wide cockpit renders");

    assert!(!plain.contains("\u{1b}["));
    assert!(colored.contains("\u{1b}[38;2;0;229;255m"));
    assert!(colored.contains("\u{1b}[38;2;0;255;154m"));
    assert!(colored.contains("\u{1b}[38;2;255;209;102m"));
    assert!(colored.contains("\u{1b}[38;2;255;77;109m"));
    assert!(colored.contains("\u{1b}[38;2;168;85;247m"));
    assert!(colored.contains("\u{1b}[38;2;96;165;250m"));
    assert!(colored.contains("WATCHLIST"));
    assert!(colored.contains("MICROSTRUCTURE"));
    assert!(colored.contains("CANDLES"));
    assert!(colored.contains("BOOK"));
    assert!(colored.contains("TAPE"));
}

#[test]
fn wide_status_bar_renders_theme_calibration_rail() {
    let model = RatatuiFrameModel::new(
        fixture_snapshots(),
        "READ-ONLY Hyperliquid spot live screen",
        ScreenRequest::default(),
        WorkstationUiState::default(),
    );

    let plain = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 240,
            height: 48,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("plain wide cockpit renders");
    let colored = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 240,
            height: 48,
        },
        RatatuiColorMode::Color,
    )
    .expect("colored wide cockpit renders");

    assert!(!plain.contains("\u{1b}["));
    assert!(plain.contains("THEME plain"));
    assert!(plain.contains("COLOR plain fallback"));
    assert!(plain.contains("--color always"));
    assert!(colored.contains("THEME"));
    assert!(colored.contains("ansi"));
    assert!(colored.contains("COLOR ansi-neon active"));
    assert!(colored.contains("--color always"));
    assert!(colored.contains("\u{1b}[38;2;0;255;154m▲"));
    assert!(colored.contains("\u{1b}[38;2;255;77;109m▼"));
}

#[test]
fn wide_status_bar_renders_neon_market_state_strip() {
    let model = RatatuiFrameModel::new(
        directional_snapshots(),
        "READ-ONLY Hyperliquid spot live screen",
        ScreenRequest::default(),
        WorkstationUiState::default(),
    );

    let plain = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 240,
            height: 48,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("plain wide cockpit renders neon state");
    let colored = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 240,
            height: 48,
        },
        RatatuiColorMode::Color,
    )
    .expect("colored wide cockpit renders neon state");

    assert!(!plain.contains("\u{1b}["));
    assert!(plain.contains("NEON STATE"));
    assert!(plain.contains("regime mixed"));
    assert!(plain.contains("heat ██░░"));
    assert!(plain.contains("breadth 01/01"));
    assert!(plain.contains("read-only signal cockpit"));
    assert!(colored.contains("NEON STATE"));
    assert!(colored.contains("\u{1b}[38;2;0;229;255mNEON STATE"));
}

#[test]
fn cockpit_chart_colorizes_directional_candles_in_color_mode() {
    let snapshots = fixture_snapshots();
    let candles = directional_chart_candles(&snapshots[0].symbol);
    let model = RatatuiFrameModel::new(
        snapshots,
        "READ-ONLY Hyperliquid spot live screen",
        ScreenRequest::default(),
        WorkstationUiState::default(),
    )
    .with_candles(candles);

    let plain = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 160,
            height: 48,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("plain chart renders");
    let colored = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 160,
            height: 48,
        },
        RatatuiColorMode::Color,
    )
    .expect("colored chart renders");

    assert!(!plain.contains("\u{1b}["));
    assert!(plain.contains("█"));
    assert!(plain.contains("▓"));
    assert!(colored.contains("\u{1b}[38;2;0;255;154m█"));
    assert!(colored.contains("\u{1b}[38;2;255;77;109m▓"));
}

#[test]
fn cockpit_chart_renders_latest_candle_hud() {
    let snapshots = fixture_snapshots();
    let candles = directional_chart_candles(&snapshots[0].symbol);
    let model = RatatuiFrameModel::new(
        snapshots,
        "READ-ONLY Hyperliquid spot live screen",
        ScreenRequest::default(),
        WorkstationUiState::default(),
    )
    .with_candles(candles);

    let rendered = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 160,
            height: 48,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders latest candle hud");

    assert!(rendered.contains("CANDLE HUD"));
    assert!(rendered.contains("latest DOWN"));
    assert!(rendered.contains("body -2.0000"));
    assert!(rendered.contains("range 18.33%"));
    assert!(rendered.contains("vol 160"));
    assert!(rendered.contains("trades 12"));
    assert!(rendered.contains("public OHLCV"));
}

#[test]
fn wide_chart_renders_public_volume_profile_rail() {
    let snapshots = fixture_snapshots();
    let candles = directional_chart_candles(&snapshots[0].symbol);
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
    .with_candles(candles);

    let rendered = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 180,
            height: 52,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders chart volume profile rail");

    assert!(rendered.contains("[FOCUS] CANDLES"));
    assert!(rendered.contains("PROFILE RAIL"));
    assert!(rendered.contains("POC 10.0000"));
    assert!(rendered.contains("VWAP 10.7692"));
    assert!(rendered.contains("profile"));
    assert!(rendered.contains("volume 260"));
    assert!(rendered.contains("public POC"));
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
    assert!(rendered.contains("KEY MATRIX"));
    assert!(rendered.contains("PANES 1W 2D 3C 4B 5T 6S"));
    assert!(rendered.contains("mnemonic focus w/i/c/b/r/o"));
    assert!(rendered.contains("z zoom/grid"));
    assert!(rendered.contains("z pane zoom"));
    assert!(rendered.contains("enter detail"));
    assert!(rendered.contains("h health/status"));
    assert!(rendered.contains("MARKET OPS g symbol / filter p preset s sort"));
    assert!(rendered.contains("STATE view flow | pane chart | density dense"));
    assert!(rendered.contains("PALETTE DIAGNOSTIC"));
    assert!(rendered.contains("mode no-color"));
    assert!(rendered.contains("COLOR PATH plain fallback"));
    assert!(rendered.contains("truecolor ANSI"));
    assert!(rendered.contains("force --color always"));
    assert!(rendered.contains("If the cockpit is black/white"));
    assert!(rendered.contains("avoid --color never"));
    assert!(rendered.contains("READ-ONLY public market data only"));
    assert!(rendered.contains("[ / ]"));
    assert!(rendered.contains("1-6 panes"));
    assert!(rendered.contains("g symbol"));
    assert!(rendered.contains("/ filter"));
    assert!(!rendered.contains("reserved"));
}

#[test]
fn help_overlay_color_mode_renders_operator_keyboard_map() {
    let snapshots = fixture_snapshots();
    let mut state = WorkstationUiState::default();
    state.apply(
        WorkstationAction::FocusPane(WorkstationPane::Chart),
        snapshots.len(),
    );
    state.apply(WorkstationAction::ToggleHelp, snapshots.len());
    let model = RatatuiFrameModel::new(
        snapshots,
        "READ-ONLY Hyperliquid spot live screen",
        ScreenRequest::default(),
        state,
    );

    let plain = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 150,
            height: 44,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("plain help overlay renders");
    let colored = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 150,
            height: 44,
        },
        RatatuiColorMode::Color,
    )
    .expect("colored help overlay renders");

    assert!(!plain.contains("\u{1b}["));
    assert!(plain.contains("OPERATOR KEYBOARD MAP"));
    assert!(plain.contains("NAVIGATION"));
    assert!(plain.contains("MARKET COMMANDS"));
    assert!(plain.contains("LAYOUT"));
    assert!(plain.contains("COLOR SUPPORT"));
    assert!(plain.contains("CAPITAL BOUNDARY"));

    assert_eq!(
        active_fg_before(&colored, "OPERATOR KEYBOARD MAP"),
        Some("\u{1b}[38;2;0;229;255m")
    );
    assert_eq!(
        active_fg_before(&colored, "NAVIGATION"),
        Some("\u{1b}[38;2;255;214;102m")
    );
    assert_eq!(
        active_fg_before(&colored, "MARKET COMMANDS"),
        Some("\u{1b}[38;2;0;255;154m")
    );
    assert_eq!(
        active_fg_before(&colored, "CAPITAL BOUNDARY"),
        Some("\u{1b}[38;2;255;77;109m")
    );
    assert!(colored.contains("active pane chart"));
    assert!(colored.contains("public market data only"));
}

#[test]
fn narrow_help_overlay_renders_compact_operator_map() {
    let snapshots = fixture_snapshots();
    let mut state = WorkstationUiState::default();
    state.apply(
        WorkstationAction::FocusPane(WorkstationPane::Book),
        snapshots.len(),
    );
    state.apply(WorkstationAction::ToggleHelp, snapshots.len());
    let model = RatatuiFrameModel::new(
        snapshots,
        "READ-ONLY Hyperliquid spot live screen",
        ScreenRequest::default(),
        state,
    );

    let plain = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 72,
            height: 24,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("plain compact help overlay renders");
    let colored = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 72,
            height: 24,
        },
        RatatuiColorMode::Color,
    )
    .expect("colored compact help overlay renders");

    assert!(!plain.contains("\u{1b}["));
    assert!(plain.contains("HELP COMPACT"));
    assert!(plain.contains("pane book"));
    assert!(plain.contains("j/k rows"));
    assert!(plain.contains("1-6 panes"));
    assert!(plain.contains("w/i/c/b/r/o focus"));
    assert!(plain.contains("g symbol"));
    assert!(plain.contains("/ filter"));
    assert!(plain.contains("z zoom"));
    assert!(plain.contains("color no-color"));
    assert!(plain.contains("--color always"));
    assert!(plain.contains("READ-ONLY public market data only"));
    assert!(!plain.contains("OPERATOR KEYBOARD MAP"));
    assert_eq!(
        active_fg_before(&colored, "HELP COMPACT"),
        Some("\u{1b}[38;2;0;229;255m")
    );
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
    assert!(rendered.contains("VOL LANE"));
    assert!(rendered.contains("max 1200"));
    assert!(rendered.contains("last 1200"));
    assert!(rendered.contains("MOVE +0.5000"));
    assert!(rendered.contains("RANGE 2.32%"));
}

#[test]
fn cockpit_chart_renders_bootstrap_lens_before_public_candles_arrive() {
    let model = RatatuiFrameModel::new(
        fixture_snapshots(),
        "READ-ONLY Hyperliquid spot live screen",
        ScreenRequest::default(),
        WorkstationUiState::default(),
    );

    let plain = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 160,
            height: 48,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("plain chart bootstrap renders");
    let colored = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 160,
            height: 48,
        },
        RatatuiColorMode::Color,
    )
    .expect("colored chart bootstrap renders");

    assert!(!plain.contains("\u{1b}["));
    assert!(plain.contains("CHART BOOTSTRAP"));
    assert!(plain.contains("public 1m feed pending"));
    assert!(plain.contains("No synthetic candles are rendered"));
    assert!(colored.contains("CHART BOOTSTRAP"));
    assert!(colored.contains("\u{1b}[38;2;255;77;109mpx "));
    assert!(colored.contains("\u{1b}[38;2;0;255;154mbid "));
    assert!(colored.contains("\u{1b}[38;2;255;77;109mask "));
    assert!(colored.contains("\u{1b}[38;2;255;77;109mflow "));
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
    assert!(rendered.contains("VOL LANE"));
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
    assert!(rendered.contains("REGIME"));
    assert!(rendered.contains("MOMENTUM"));
    assert!(rendered.contains("MICRO"));
    assert!(rendered.contains("spread gate wide"));
    assert!(rendered.contains("no execution"));
    assert!(rendered.contains("flow -$35"));
    assert!(rendered.contains("depth $245"));
    assert!(rendered.contains("imb -0.15"));
    assert!(rendered.contains("score 2"));
}

#[test]
fn focused_chart_renders_strategy_hud_without_execution_language() {
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
    .with_candles(directional_chart_candles("@107"));

    let plain = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 160,
            height: 48,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("plain strategy HUD renders");
    let colored = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 160,
            height: 48,
        },
        RatatuiColorMode::Color,
    )
    .expect("colored strategy HUD renders");

    assert!(!plain.contains("\u{1b}["));
    assert!(plain.contains("STRATEGY HUD"));
    assert!(plain.contains("bias"));
    assert!(plain.contains("signal"));
    assert!(plain.contains("liquidity"));
    assert!(plain.contains("flow gate"));
    assert!(plain.contains("confidence 100"));
    assert!(plain.contains("watch only"));
    assert!(plain.contains("no orders"));
    assert!(plain.contains("not advice"));
    assert!(!plain.contains("buy now"));
    assert!(!plain.contains("sell now"));
    assert_eq!(
        active_fg_before(&colored, "STRATEGY HUD"),
        Some("\u{1b}[38;2;0;229;255m")
    );
}

#[test]
fn wide_chart_renders_selected_pair_public_prints_strip() {
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
    .with_candles(fixture_candles())
    .with_trades(fixture_trades());

    let rendered = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 220,
            height: 52,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders chart prints strip");

    assert!(rendered.contains("PRINTS STRIP"));
    assert!(rendered.contains("public time-and-sales"));
    assert!(rendered.contains("prints 2"));
    assert!(rendered.contains("buy 1"));
    assert!(rendered.contains("sell 1"));
    assert!(rendered.contains("last SELL"));
    assert!(rendered.contains("35.2000"));
    assert!(rendered.contains("no fills"));
}

#[test]
fn wide_chart_renders_public_print_markers_on_candles() {
    let snapshots = fixture_snapshots();
    let candles = directional_chart_candles(&snapshots[0].symbol);
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
    .with_candles(candles)
    .with_trades(fixture_trades());

    let rendered = render_ratatui_snapshot_for_test(
        &model,
        RatatuiViewport {
            width: 220,
            height: 52,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders chart print markers");

    assert!(rendered.contains("[FOCUS] CANDLES"));
    assert!(rendered.contains("PRINT MARKERS"));
    assert!(rendered.contains("BS buy 1 sell 1"));
    assert!(rendered.contains("net +$35"));
    assert!(rendered.contains("public prints no fills"));
}

#[test]
fn wide_chart_renders_selected_pair_order_pressure_lane() {
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
            width: 220,
            height: 52,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders wide chart pressure lane");

    assert!(rendered.contains("ORDER PRESSURE"));
    assert!(rendered.contains("BID"));
    assert!(rendered.contains("ASK"));
    assert!(rendered.contains("bid wall"));
    assert!(rendered.contains("ask wall"));
    assert!(rendered.contains("book skew"));
    assert!(rendered.contains("read-only top-book lens"));
}

#[test]
fn cockpit_chart_renders_session_microstructure_strip() {
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
    .expect("renders chart session strip");

    assert!(rendered.contains("SESSION STRIP"));
    assert!(rendered.contains("RET 1m"));
    assert!(rendered.contains("RV 1m/5m/1h"));
    assert!(rendered.contains("OFI"));
    assert!(rendered.contains("adverse"));
    assert!(rendered.contains("spread"));
    assert!(rendered.contains("age"));
    assert!(rendered.contains("public signal context"));
}

#[test]
fn wide_chart_renders_selected_pair_crosshair_context() {
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
            width: 220,
            height: 52,
        },
        RatatuiColorMode::NoColor,
    )
    .expect("renders chart crosshair context");

    assert!(rendered.contains("CROSSHAIR"));
    assert!(rendered.contains("selected HYPE/USDC"));
    assert!(rendered.contains("range pos"));
    assert!(rendered.contains("session high"));
    assert!(rendered.contains("session low"));
    assert!(rendered.contains("spread 57.1bps"));
    assert!(rendered.contains("confidence 100"));
    assert!(rendered.contains("read-only chart lens"));
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

    assert!(rendered.contains("TIMEFRAME RAIL"));
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
