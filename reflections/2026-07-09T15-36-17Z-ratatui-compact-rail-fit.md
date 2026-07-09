# Ratatui compact rail fit pass

## Success criteria
- A 100x30 fixture render keeps the header, DESK, controls, status, and action rails readable without clipped words.
- A 160x48 fixture render shows a complete compact risk rail instead of truncating after `deg`.
- Read-only/no-wallet safety text remains visible in compact status/action areas.

## Failure hypotheses
- The medium header includes too much state (`VISUAL`, full pane names, full title) for 100 columns.
- The 132-179 status branch still uses wide risk labels even though the bottom rail is only two terminal lines.
- The action rail consumes width with theme/color diagnostics before the core command keys fit.

## Candidate approaches
- Add a sub-medium copy path for 90-109 columns.
- Reuse compact ticker/quality/risk helpers up to 179 columns.
- Make action/theme diagnostics progressively shorter while preserving command keys and color-mode evidence.

## Result
- Added regression coverage for 100-column and 160-column rail fit.
- Introduced a dense 90-109 column header/DESK/action path and compact 132-179 status/theme rails.
- Kept `RO no-wallet` visible in the sub-medium action rail and preserved full visual/color diagnostics at wider breakpoints.
- Verified cockpit, interaction, CLI live-TUI, live mock, build, clippy, colored fixture smoke, and explicit 100/160 render markers.
