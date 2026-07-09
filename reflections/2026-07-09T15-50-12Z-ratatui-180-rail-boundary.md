# Ratatui 180-column rail boundary pass

## Success criteria
- A 180x50 fixture render uses compact DESK, ticker, quality, and risk rails without clipped words.
- Wider 220+ layouts keep their richer visual and command diagnostics.
- The change stays display-only and preserves read-only/no-wallet safety boundaries.

## Failure hypotheses
- The 180-column breakpoint was treated as wide even though the footer and DESK rails did not have enough space.
- Compact status labels were only applied below 180 columns.
- Existing tests encoded full 180-column labels even though the terminal render clipped them.

## Result
- Added a regression test for exact 180-column rail fit.
- Switched 180-column DESK/status rails to the compact standard-wide copy path.
- Updated the 180-column DESK interaction test to assert compact pane labels.
- Verified cockpit, interaction, CLI live-TUI, live mock, build, clippy, colored fixture smoke, and explicit 180-column render markers.
