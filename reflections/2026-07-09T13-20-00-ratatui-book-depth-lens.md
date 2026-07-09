# Ratatui Book Depth Lens

## Success criteria
- The book pane exposes a compact read-only depth lens near the top book, with bid and ask pressure bars visible without expanding the pane.
- Color mode renders bid/ask pressure segments with separate semantic colors.
- No-color snapshots remain readable and ANSI-free.

## Failure hypotheses
1. The wide default book pane may not have enough vertical space for another line without crowding existing evidence.
2. A styled label alone could satisfy a weak color test, so assertions must anchor on colored bar segments.
3. The lens must keep the current top-book-only safety language clear and avoid implying executable L2/order routing.

## Candidate approaches
- Insert a single `DEPTH LENS` line after the BBO ladder/microprice lines in non-compact book layouts.
- Reuse existing quote-share and notional helpers, with styled bid/ask bar spans rather than new market data.

## Attempt log
- Starting with a rendered color/no-color test for the focused wide book pane.
- Red test failed on the missing `DEPTH LENS` line.
- Added a top-book-only depth lens after the BBO ladder/microprice lines with green bid pressure and red ask pressure spans.
- Targeted rendered test passed after asserting the ANSI stream around colored bid/ask bar segments.
- Fixture smoke initially showed the conservative 160-column render hid the wide-only lens, so the lens now renders in every non-compact book pane while the BBO ladder remains wide-only.
