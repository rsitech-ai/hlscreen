# Live Spot Symbol Display Mapping Fix

## Task
- **ID/Title:** 2026-07-08 live spot symbol display mapping fix
- **Date:** 2026-07-08
- **Scope:** Hyperliquid public spot metadata parser, live CLI symbol selection, and TUI display labels

## Plan and Risks
- **Planned approach:** Reproduce with live `spotMeta`, add live-shaped fixtures, fix the metadata boundary, validate with live `hls symbols` and a short live TUI run.
- **Top failure hypotheses:** `spotMeta.universe[].name` is being used as display text; explicit live symbols are not normalized before WebSocket subscription; `spotMetaAndAssetCtxs` contexts are aligned by array position instead of feed ID.
- **Success criteria:** `HYPE/USDC`, `hype-usdc`, and `@107` select the same feed; live TUI shows `HYPE/USDC`; current `UETH/USDC` mapping is visible; tests and release gates pass.

## Reflection
- **Failure modes observed:** HYPE displayed as `@107`, and worse, HYPE/USDC initially showed the wrong mark/mid values because asset contexts were joined by array position.
- **Root cause:** The parser treated Hyperliquid's spot universe `name` as a display name and assumed the asset context array was position-aligned with the universe array. Live payloads require deriving display pairs from token indexes and joining contexts by `coin`.
- **Fix that resolved it:** Derive display names as `base_token.name/quote_token.name`, keep `hl_coin` as `@{spot_index}` except PURR, map explicit live selectors through metadata, and prefer metadata display names in TUI rows.
- **Useful command-level evidence:** `hls symbols --include hype-usdc --top 1` prints `HYPE/USDC @107`; `hls symbols --include ueth-usdc --top 1` prints `UETH/USDC @151`; `hls live --symbols hype-usdc --duration-secs 5 --refresh-secs 5 --tui` rendered `HYPE/USDC` with 0 reconnects and 0 data gaps.

## Reusable Lesson
- **Pattern that worked:** Treat exchange display names and transport IDs as separate fields at the parser boundary, then validate both with live API output and a TUI smoke.
- **Pattern to avoid:** Never infer context alignment from array position when the payload includes an explicit `coin` key.
- **Where to apply next:** Replay metadata reattachment and any future multi-venue symbol mapper.

## Decision
- **Final chosen approach:** Fix the parser boundary and CLI/TUI surfaces without changing raw event storage or replay feed IDs.
- **Commit/rollback decision:** Commit after full gates pass; rollback is safe because the change is read-only metadata parsing and display.
- **Next step / follow-up:** Consider persisting display metadata into replay flows so replay-only TUI can show pairs without extra metadata files.
