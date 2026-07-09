# Ratatui Command Palette HUD Reflection

## Success criteria
- Wide command palette reads as an operator HUD, not a plain form.
- Existing command targets, suggestions, validation, and read-only boundaries remain intact.
- Compact command palette remains stable for narrow terminals.

## Failure hypotheses
- Adding another line could crowd the popup and hide lower command context.
- Safety copy could imply trading execution instead of read-only screen mutation.
- Color assertions could become brittle if labels share nearby ANSI spans.

## Candidate approaches
- Add a single top HUD rail to the wide command palette with command keys, target, live status, and no-order guardrail.
- Replace the existing command title with a denser HUD title.

## Attempt 1
- Red test added for `COMMAND HUD`, key chips, and `RO no orders`.
- Signal: `ratatui_cockpit` fails only the new HUD expectations; existing behavior remains green.

## Attempt 2
- First renderer pass added a new top line, but it crowded the fixed-height popup and pushed the lower safety rails out.
- Revised approach folds HUD controls into the existing title line to preserve palette height.

## Closeout
- `ratatui_cockpit`: 111 passed.
- `workstation_interaction`: 11 passed.
- `hls-cli live_tui`: 18 passed.
- `live_mock`: 3 passed.
- `cargo fmt --check`, `git diff --check`, `cargo build -p hls-cli`, and strict clippy passed.
