# Ratatui Live TUI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace `hls live --tui` with one unified, full-screen, adaptive Ratatui trading workstation that preserves existing read-only market-data features.

**Architecture:** Keep ingestion, screening, feature generation, metadata enrichment, recording, and safety boundaries unchanged. Add a Ratatui presentation/runtime layer in `hls-tui`; wire `hls-cli` live mode to render through that layer while keeping non-TTY fixture and command output deterministic.

**Tech Stack:** Rust 2024, `ratatui`, existing `crossterm`, existing `hls-core`, `hls-screen`, `hls-features`, and `hls-cli`.

## Global Constraints

- Public market data only; no wallets, private streams, signing, orders, execution routes, or profitability claims.
- Display pause must pause rendering only; live ingestion and recording must continue.
- The TUI must be resize-aware and must not depend on fixed terminal width.
- Existing screen presets, filters, sorting, confidence, resilience, tradeability, metadata, why-ranked scoring, health counters, and recording status remain visible.
- `NO_COLOR` and `TERM=dumb` must disable color.
- Non-TTY fixture/screenshot/golden paths must remain deterministic.

---

### Task 1: Add Ratatui Cockpit Renderer Surface

**Files:**
- Modify: `Cargo.toml`
- Modify: `crates/hls-tui/Cargo.toml`
- Create: `crates/hls-tui/src/ratatui_app.rs`
- Modify: `crates/hls-tui/src/lib.rs`
- Test: `crates/hls-tui/tests/ratatui_cockpit.rs`

**Interfaces:**
- Consumes: `hls_core::market_state::FeatureSnapshot`, `hls_screen::ScreenRequest`, `hls_tui::interaction::WorkstationUiState`.
- Produces: `RatatuiFrameModel`, `RatatuiViewport`, `RatatuiColorMode`, `render_ratatui_snapshot_for_test(...) -> hls_core::HlsResult<String>`.

- [x] **Step 1: Write failing tests**

Create `crates/hls-tui/tests/ratatui_cockpit.rs` with tests that build fixture snapshots and assert:
- wide view contains `WATCHLIST`, `MICROSTRUCTURE`, `CHART`, `TAPE`, `BOOK`, `HYPE/USDC`, `confidence`, `No wallet`;
- narrow view contains `WATCHLIST` and `DETAIL` and omits `TAPE`;
- color mode emits ANSI escapes and no-color mode does not.

Run:

```bash
cargo test -p hls-tui --test ratatui_cockpit
```

Expected: fail because `ratatui_app` does not exist.

- [x] **Step 2: Add dependencies and minimal renderer**

Add workspace dependency:

```toml
ratatui = "0.29"
```

Add `ratatui.workspace = true` to `crates/hls-tui/Cargo.toml`.

Implement `crates/hls-tui/src/ratatui_app.rs`:
- a viewport struct `{ width: u16, height: u16 }`;
- color mode enum `{ Auto, Color, NoColor }`;
- frame model containing rows, title, screen request, UI state, health/recording summary strings;
- `render_ratatui_snapshot_for_test` using `ratatui::backend::TestBackend` and `Terminal::draw`.

- [x] **Step 3: Verify Task 1**

Run:

```bash
cargo test -p hls-tui --test ratatui_cockpit
cargo test -p hls-tui --test interactive_tui --test main_table_golden
```

Expected: all pass.

### Task 2: Wire Live Runtime To Ratatui Terminal

**Files:**
- Modify: `crates/hls-cli/src/commands/live.rs`
- Modify: `crates/hls-tui/src/ratatui_app.rs`
- Test: `crates/hls-cli/tests/live_mock.rs`

**Interfaces:**
- Consumes: `RatatuiFrameModel`.
- Produces: live `--tui` progress frames rendered with `ratatui::Terminal<CrosstermBackend<Stderr>>`.

- [x] **Step 1: Write failing CLI test**

Extend `live_mock.rs` with a fixture-backed `--once --tui` assertion that the output includes `WATCHLIST`, `MICROSTRUCTURE`, `CHART`, and read-only caveat text.

Run:

```bash
cargo test -p hls-cli --test live_mock
```

Expected: fail until `--tui` once/final path uses the Ratatui renderer.

- [x] **Step 2: Implement live Ratatui path**

In `live.rs`:
- keep raw mode and keyboard polling;
- for live progress, draw a Ratatui frame to stderr instead of writing the old string table;
- for fixture `--once --tui` and final live summary, print a TestBackend snapshot only when stdout output is needed;
- keep non-`--tui` output on the old deterministic renderer.

- [x] **Step 3: Verify Task 2**

Run:

```bash
cargo test -p hls-cli --test live_mock
cargo test -p hls-tui --test ratatui_cockpit
```

Expected: all pass.

### Task 3: Preserve Existing Keyboard State And Resize Behavior

**Files:**
- Modify: `crates/hls-tui/src/interaction.rs`
- Modify: `crates/hls-tui/src/ratatui_app.rs`
- Test: `crates/hls-tui/tests/ratatui_cockpit.rs`

**Interfaces:**
- Produces: existing view cycling, density, help, pause, selected row, narrow/wide layout selection, and visible keyboard legend.

- [x] **Step 1: Write failing interaction tests**

Add assertions that `Tab`/`Shift+Tab` view cycling is represented in the rendered title/legend, that pause/help/density state is visible, and that `RatatuiViewport { width: 70, height: 24 }` uses a compact single-column layout.

- [x] **Step 2: Implement existing state in Ratatui**

Render `WorkstationUiState` view, density, help, pause, and selected row in the Ratatui cockpit without changing the existing input contract.

- [x] **Step 3: Verify Task 3**

Run:

```bash
cargo test -p hls-tui --test interactive_tui --test ratatui_cockpit
```

Expected: all pass.

### Task 4: End-To-End Validation

**Files:**
- Modify: `README.md`
- Modify: `memory/2026-07-08.md`
- Create or update: `reflections/<timestamp>-ratatui-live-tui.md`

**Interfaces:**
- Produces: documented live command and verification evidence.

- [x] **Step 1: Run full validation**

Run:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --all-features
scripts/check-release-packaging.sh
python3 scripts/generate-screenshots.py --check
git diff --check
```

- [x] **Step 2: Run a bounded public smoke**

Run:

```bash
./target/debug/hls live --top 10 --duration-secs 10 --refresh-secs 2 --tui
```

Result: exited 0 with 10 symbols, 40 subscriptions, 275 WS messages, 525 market events, 0 reconnects, and 0 data gaps. The non-interactive 80-column harness collapsed to the narrow watchlist/detail layout as expected.

- [x] **Step 3: Document closeout**

Update README and memory with the new TUI behavior, command, key bindings, and validation evidence.

## Follow-Up Scope

- [x] Carry true candle/OHLC history into the live presentation model before rendering real candlesticks.
- [x] Add deterministic in-TUI controls for filter, preset, sort, and chart window.
- [x] Add a free-text in-TUI command palette/editor for arbitrary screen DSL filters with validation before mutation.
- [x] Add explicit keyboard pane focus state after command editing exists.
- [x] Add first pane-specific actions for watchlist row movement, detail view cycling, and chart-window cycling.
- [x] Keep book/tape reachable in adaptive layouts: medium terminals show compact book/tape panels, and narrow terminals render focused hidden panes as drilldowns.
- [x] Add direct `1`-`6` pane hotkeys for watchlist, detail, chart, book, tape, and status.
- [x] Split the Ratatui header into a two-line status/control rail so live mode, focused pane, density, chart window, filter, and primary keyboard controls stay readable on smaller terminals.
- [x] Upgrade the watchlist into a denser market board with rank, UP/DN movement, signed flow, top-book depth, and quality badges.
- [x] Make the market board width-aware so 120-column terminals use compact columns without clipping movement signals.
- [x] Render focused status as a real operational drilldown with stream, recorder, health, layout, controls, and read-only safety context.
- [x] Add explicit `--color auto|always|never` control so terminal/theme behavior is reproducible across shells.
- [x] Add a market internals rail with screened row count, breadth, tradeable count, stale count, aggregate signed flow, and aggregate top-book depth.
- [x] Add bid/ask share and public top-book notional bars to the Ratatui book pane.
- [x] Add adaptive tape flow-pulse and net-pressure bars while preserving the public-flow safety label at 120 columns.
- [x] Add price-axis labels and a public 1m candle footer to the Ratatui candle chart.
- [x] Add a visual score/factor stack to the Ratatui detail and explain panes.
- [x] Add selected-symbol public recent trades to the Ratatui tape pane while preserving no-fills/no-private-stream safety copy.
- [x] Add row-level `SIG` score and `BIAS` leading-factor columns to the wide Ratatui market board.
- Consider deeper order-book/tape interactions and persisted TUI preferences after the focused-pane state is stable.
