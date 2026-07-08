# Ratatui Contextual Pane Actions

## Intent

Make focused panes operational, not just decorative. The advanced TUI spec says keyboard actions should apply to the focused pane. This slice keeps read-only market data untouched while making the existing `Up`/`Down` navigation context-aware for watchlist, detail, and chart panes.

## Success Criteria

- Default focus remains watchlist, preserving row navigation for existing users.
- When detail is focused, `Up`/`Down` change the active detail view and do not move the selected market row.
- When chart is focused, `Up`/`Down` adjust the chart window and do not move the selected market row.
- Command palette input still takes priority over pane navigation.
- Tests and live smoke prove the behavior without changing ingestion, recording, subscriptions, or order-readiness boundaries.

## Failure Hypotheses

- Contextual `Up`/`Down` could make row movement feel unavailable after pane focus changes unless help/status text explains it.
- Chart window cycling needs a previous direction to make `Up` useful.
- Mouse scroll events already map to `Up`/`Down`; after this change they become pane-contextual too, which should match focused-pane semantics.

## Attempt Log

- Added a focused-pane state test that proves watchlist focus moves rows, detail focus cycles views, and chart focus cycles chart windows while preserving selected market row.
- Added a previous chart-window transition so `Up` and `Down` are symmetric in the chart pane.
- Updated Ratatui help text to describe focused-pane actions instead of claiming all arrows always move market rows.
- Validation passed: focused TUI tests, live CLI interaction tests including mouse parity, full workspace tests/build, full workspace clippy, fixture TUI render, and short public top-10 live smoke with 218 WS messages, 472 market events, zero reconnects, and zero gaps.
