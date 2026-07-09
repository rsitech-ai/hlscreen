# Ratatui standard-wide density pass

## Success criteria
- A 220x56 live fixture render does not visibly clip the top DESK visibility rail or bottom NEON STATE copy.
- Existing pane controls, read-only guardrails, color-mode semantics, and live-data copy remain present.
- Focused Ratatui tests cover the standard-wide adaptive copy path before implementation.

## Failure hypotheses
- The 220-column branch treats the terminal as ultrawide and emits too much descriptive copy.
- Compacting labels globally could weaken the existing ultrawide visual contract.
- Bottom action/status rails may still overrun even after header copy is shortened.

## Candidate approaches
- Add a standard-wide density breakpoint that keeps ultrawide copy at 240+ columns but uses compact labels below that.
- Shorten only terminal-visible labels while preserving existing state and commands.
- If text still clips, move secondary status copy into the market/status rail instead of the action rail.

## Result
- Added a 220-column regression test for compact DESK visibility and NEON status rails.
- Preserved full ultrawide labels at 240+ columns and kept the change display-only.
- Verified with Ratatui cockpit tests, workstation interaction tests, CLI live-TUI tests, live mock integration, build, clippy, and colored fixture smoke.
