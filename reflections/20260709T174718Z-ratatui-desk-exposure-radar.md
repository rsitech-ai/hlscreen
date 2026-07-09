## Task

Add a cross-row desk exposure radar to the expanded Ratatui status pane using only public screened market data.

## Success Criteria

- Expanded status shows long, short, and neutral screen buckets derived from public row returns.
- It shows public notional proxies from top-book depth and highlights the lowest-confidence row.
- It states the boundary as screen exposure only, not sizing, positions, orders, or advice.
- Existing status, adaptive layout, color, and live CLI tests stay green.

## Failure Hypotheses

1. The status pane already has risk lines, but not a compact cross-row exposure lens that reads like an algorithmic desk.
2. Exposure wording could imply trading or position sizing if not explicitly labeled as screen-only.
3. Extra expanded-status lines could overflow smaller focused status panes unless kept concise.

## Candidate Approaches

- Add the radar to expanded status only, beside mission/risk/color diagnostics.
- Add another normal status-bar line.

Chosen approach: expanded status only, because it is discoverable through `h`/`6` plus `z` and does not crowd the default workstation.
