# Ratatui Selected Quote Rail Slice

## Success Criteria
- Wide Ratatui layouts surface the selected symbol in the header with bid/ask share, spread, top-book depth, and flow context.
- The rail uses existing public BBO fields only and keeps the read-only/no-orders boundary explicit.
- Medium and narrow layouts keep their current compact header behavior.

## Failure Hypotheses
- Adding a header line could clip existing header controls if the wide header height is not adjusted.
- The rail could duplicate detail/book content without improving first-glance operator context.
- Tests could pass on text presence while silently crowding the main cockpit body.

## Candidate Approaches
- Add a wide-only `SELECTED QUOTE` rail in `render_header`, reusing the existing BBO notional/share helpers.
- Fold selected quote data into `MARKET PULSE`, but that would mix global market state with selected-pair state.

## Attempt Log
- Starting with a wide-header behavior test, then adding a dedicated selected quote rail only when the wide header has enough height.
- Focused red/green signal: `cockpit_header_renders_selected_quote_rail` failed on missing `SELECTED QUOTE`, then passed after adding the rail and increasing only the wide header height.
- Broad cockpit signal: applying the extra header row to all wide layouts clipped 160-column book content. The header height is now conditional on the same `>=220` breakpoint that renders the selected quote rail.
- Final verification passed: fmt, Ratatui cockpit/workstation tests, hls-cli live TUI tests, live mock, build, ANSI fixture smoke, clippy, and diff hygiene.
