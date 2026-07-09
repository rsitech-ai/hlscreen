# Ratatui medium density pass

## Success criteria
- A 120x40 fixture render keeps the header filter, layout controls, ticker quality, and risk rails visible without clipped words.
- Existing narrow status micro-copy and wide/ultrawide labels keep their compatibility contracts.
- The change stays display-only: no market-data, ranking, safety, or interaction behavior changes.

## Failure hypotheses
- Medium status was still using wide labels for filter, ticker, quality, and risk.
- Shortening global compact helpers could regress narrow terminal behavior.
- Removing too much copy could hide the read-only/no-wallet safety boundary.

## Result
- Added a 120-column regression test for fit-to-width header and status rails.
- Introduced medium-only compact filter, ticker, quality, risk, and health labels while preserving `No wallet`.
- Verified the full Ratatui cockpit, workstation interaction, CLI live-TUI, live mock, build, clippy, colored fixture smoke, and explicit 120-column render markers.
