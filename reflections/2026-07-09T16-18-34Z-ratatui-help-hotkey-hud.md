# Ratatui Help Hotkey HUD

## Success Criteria

- Wide help overlay opens with a cockpit-style hotkey HUD, not only a plain list.
- The HUD surfaces symbol, filter, preset, sort, chart-window, pane zoom, and read-only public-data status.
- Colored help overlay uses semantic styling for the HUD and active pane zoom label.
- The read-only safety boundary remains visible after adding the extra overlay line.
- Existing compact help, keyboard routing, live TUI entrypoint, and read-only behavior remain unchanged.

## Failure Hypotheses

1. Adding a new help line can push the final safety boundary out of the popup viewport.
2. ANSI styling splits visible text, so color-mode tests should assert styled tokens directly.
3. Wider help content must not affect compact help behavior on narrow shells.

## Attempt Result

- Added a `HOTKEY HUD` line to the wide help overlay with bracketed command keys and active-pane zoom.
- Increased the wide help popup height so `READ-ONLY public market data only` remains visible.
- Added behavior coverage for the HUD text and color styling.
- Verified full Ratatui snapshots, workstation interaction tests, CLI live TUI tests, live mock integration, formatting, diff hygiene, build, and clippy.
