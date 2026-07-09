# Ratatui Detail Bid/Ask Balance Slice

## Success Criteria
- The selected-symbol overview detail pane shows a read-only bid/ask balance strip sourced from existing public BBO top-book fields.
- Compact detail views stay dense and do not gain another full-width line.
- Existing expanded quote terminal, book, tape, and workstation tests remain green.

## Failure Hypotheses
- Adding a line could crowd small terminals or push existing detail content below the visible area.
- Recomputing BBO notional separately could drift from the existing book/expanded quote semantics.
- Test assertions could overfit exact fixture dollars instead of the behavioral labels users rely on.

## Candidate Approaches
- Reuse `notional`, `quote_share`, `depth_bar`, and `percent_label` in a new overview-only helper.
- Move the existing expanded quote terminal BID/ASK line into overview, but that would duplicate too much expanded-mode structure.

## Attempt Log
- Starting with a behavior assertion in the overview quote card test, then adding a small helper beside `quote_strip_line`.
- Focused red/green signal: `detail_overview_renders_quote_card` failed on missing `BID/ASK BALANCE`, then passed after adding `quote_balance_line`.
- Broad cockpit signal: the first implementation crowded a 160-column viewport enough to clip the liquidity radar detail. The balance strip now follows the existing pane-level `PAIR SNAPSHOT` breakpoint.
- Final verification passed: fmt, Ratatui cockpit/workstation tests, hls-cli live TUI tests, live mock, build, ANSI fixture smoke, clippy, and diff hygiene.
