# Ratatui Row Intent Deck Reflection

## Success criteria
- Selected watchlist rows expose an operator-style intent deck.
- The intent deck routes only to read-only views: detail, chart, book, and tape.
- The deck explicitly says there is no order route and no wallet.
- Existing row router, action map, scanner rail, and read-only row context remain visible.

## Failure hypotheses
- Extra row-router copy could crowd the fixed-height watchlist command deck.
- Intent wording could imply execution instead of inspection.
- Styling could regress no-color snapshots.

## Candidate approaches
- Add one compact `INTENT DECK` line between row summary and action map.
- Replace the existing action map with intent wording.

## Attempt 1
- Added behavior assertions for `INTENT DECK`, read-only routes, no order route, and no wallet.
- Signal: `ratatui_cockpit` fails only the new intent-deck expectation; existing behavior remains green.

## Attempt 2
- A single extra line crowded scanner/command-center details in the fixed-height router.
- Increased row-router height where space exists and split intent into two short lines so safety and routes fit inside the watchlist column.
- Signal: `ratatui_cockpit` passes all 111 tests.

## Closeout
- `ratatui_cockpit`: 111 passed.
- `workstation_interaction`: 11 passed.
- `hls-cli live_tui`: 18 passed.
- `live_mock`: 3 passed.
- `cargo fmt --check`, `git diff --check`, `cargo build -p hls-cli`, and strict clippy passed.
