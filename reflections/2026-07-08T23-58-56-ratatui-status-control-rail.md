# Ratatui Status Control Rail Reflection

## Intent

Continue the unified `hls live --tui` Ratatui workstation polish by replacing the cramped single-line header with a clearer two-line cockpit rail:

- status line: live/fixture state, recorder state, view, focused pane, density, chart window, and active filter;
- control line: primary keyboard affordances, including row movement, direct pane hotkeys, view cycling, command editors, chart window, help, and quit.

## Success Criteria

- The same unified Ratatui TUI remains the only live `--tui` path.
- No ingestion, recording, screen DSL, private-data, wallet, or order-routing behavior changes.
- Snapshot tests prove the rail renders in the wide cockpit.
- Focused TUI tests, workspace tests, build, fixture smoke, and bounded public live top-10 smoke stay green.

## Failure Hypotheses

- The extra header row could starve narrow layouts and hide important panels.
- Longer control copy could wrap or become unreadable on moderate terminal widths.
- Updating the visible label from `density:` to `dens:` could break tests or expectations.

## Candidate Approaches

- Keep one header line and abbreviate harder. Rejected because it was already clipping on real terminals and hid discoverability.
- Split status and controls into two header lines. Chosen because it preserves cockpit context while making keyboard controls visible.
- Move controls only into help overlay. Rejected because the main workstation should be usable without opening help.

## Result

Implemented the two-line status/control rail in `crates/hls-tui/src/ratatui_app.rs` and covered it in `crates/hls-tui/tests/ratatui_cockpit.rs`.

Validation passed:

- `cargo fmt -p hls-tui --check`
- `cargo test -p hls-tui --test ratatui_cockpit --test interactive_tui`
- `cargo clippy -p hls-tui --all-targets --all-features -- -D warnings`
- `cargo test -p hls-cli --test health_commands doctor_live_json_reports_simulated_health -- --nocapture`
- `cargo test --workspace --all-features`
- `cargo build --workspace --all-features`
- scoped `git diff --check`
- fixture `hls live --symbols @107 ... --once --tui`
- public live top-10 8s smoke: 10 symbols, 40 subscriptions, 241 WS messages, 491 market events, 0 reconnects, 0 data gaps

Workspace-wide clippy is currently blocked by unrelated dirty `hls-store/tests/metadata_registry.rs` backfill-attempt test references to APIs not present in the dirty tree. The scoped TUI clippy gate passed.
