## Task
- **ID/Title:** Ratatui persisted display preferences
- **Date:** 2026-07-09
- **Scope:** multi-file

## Plan and Risks
- **Planned approach:** Add a small display-only preferences value to `hls-tui`, serialize it from `hls-cli` under the configured live data directory, and restore it when the live Ratatui UI starts.
- **Top failure hypotheses:** Invalid preference files could break live market-data startup; persistence could accidentally write during deterministic fixture/smoke paths; adding state accessors could expose mutable TUI internals instead of a narrow stable contract.
- **Success criteria:** View, density, and chart window round-trip through local TOML; malformed or unknown preferences fall back to defaults; live TUI starts from persisted preferences without touching wallet/order paths; focused tests and full workspace checks pass.

## Candidate Attempts
| Candidate | Summary | Outcome | Signals | Why selected / rejected |
|---|---|---|---|---|
| A | Persist full `WorkstationUiState` directly. | Rejected | Includes transient selected row, command text, help, pause, quit. | Too much state and higher chance of restoring stale runtime UI state. |
| B | Persist a narrow `WorkstationUiPreferences` value. | Selected | Captures only view, density, and chart window. | Matches user-visible customization without leaking transient controls. |

## Reflection
- **Failure modes observed:** The first patch attempt targeted the wrong insertion point in `interaction.rs`; the README update initially duplicated the REST-backfill not-implemented line.
- **Root cause:** The TUI state module has chart-window and action definitions between density and state, so broad patch context was too optimistic; the README replacement patch reused the adjacent bullet instead of deleting only the persisted-preferences bullet.
- **Fix that resolved it:** Reapplied narrower patches around `WorkstationDensity`, `WorkstationAction`, and `WorkstationUiState`, then removed the duplicated README bullet and added an explicit preferences-path note.
- **What improved score/quality:** Persisting only `WorkstationUiPreferences` keeps durable state to view, row density, and chart window while malformed/unknown local preference files fall back to defaults instead of blocking live market-data startup.
- **Useful command-level evidence:** `cargo test -p hls-cli live_tui_preferences -- --nocapture`; `cargo test -p hls-tui && cargo test -p hls-cli live_tui`; `cargo test -p hls-cli --test live_mock && cargo run -p hls-cli -- live --fixture-file tests/fixtures/hyperliquid/ws_mock_live.ndjson --once --tui --color always --data-dir /tmp/hlscreen-ratatui-prefs-smoke`; `cargo test --workspace`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- **Branch comparison insight (if multiple attempts):** Not applicable.

## Reusable Lesson
- **Pattern that worked:** Use a narrow public preferences value at the TUI boundary, then keep TOML load/save in the CLI command layer where `data_dir` is known.
- **Pattern to avoid:** Persisting transient interaction flags as durable preferences.
- **Where to apply next:** Theme/palette and command preset persistence if future TUI slices need it.

## Decision
- **Final chosen approach:** Narrow display-only preference persistence.
- **Commit/rollback decision:** Commit and push after green focused tests, deterministic fixture smoke, full workspace tests, and clippy.
- **Next step / follow-up:** Consider persisting explicit palette/theme calibration next if terminal color drift remains a user-facing issue.
