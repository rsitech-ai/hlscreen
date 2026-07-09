## Task

Add a public metadata cohort intelligence radar to the expanded Ratatui watchlist.

## Success Criteria

- Expanded watchlist summarizes metadata cohorts across visible rows.
- It highlights selected listing age, tags, seed leader, and unknown metadata count.
- It states the boundary as public metadata only, not advice or execution.
- Existing adaptive watchlist/status tests and live CLI checks stay green.

## Failure Hypotheses

1. Metadata exists per selected detail row but is not summarized where the operator scans the market.
2. Cohort counts can become misleading if missing metadata is hidden rather than counted.
3. Extra watchlist lines can crowd the expanded pane if the deck is too verbose.

## Candidate Approaches

- Add a compact expanded-watchlist-only cohort radar after the heatmap.
- Add metadata columns to the default watchlist.

Chosen approach: expanded-only radar, because default watchlist density is already high and the cohort view is most useful when the operator zooms the scanner.
