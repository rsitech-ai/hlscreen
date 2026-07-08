# CLI Contract

Binary name: `hls`

All commands are read-only with respect to exchanges. No command accepts private keys, wallet addresses for trading, order parameters, leverage, or execution actions.

## `hls init`

Creates or validates a local data directory and default config.

Required behavior:

- Writes a config file only under the requested local path.
- Does not contact Hyperliquid unless `--check-live` is supplied.
- Prints the config path and data directory.

Common options:

- `--data-dir <path>`
- `--force`
- `--check-live`

## `hls doctor`

Checks local and optional live readiness.

Required behavior:

- Always checks config readability and data-dir writability.
- With `--live`, checks public REST and WebSocket reachability using read-only market-data requests.
- Reports read-only safety status.

Common options:

- `--live`
- `--json`

## `hls symbols`

Lists known or live spot markets.

Required behavior:

- Shows display symbol, feed identifier, spot index, canonical status, 24h notional volume, mark, and mid when available.
- Supports top-N and include/exclude filters.

Common options:

- `--top <n>`
- `--include <symbol-or-feed-id>`
- `--exclude <symbol-or-feed-id>`
- `--json`

## `hls live`

Starts the live terminal screener.

Required behavior:

- Applies universe selection, subscription budget validation, and optional recording.
- Shows sortable table, symbol details, health pane, stale-data status, and read-only status.
- Does not block ingestion on rendering or storage.

Common options:

- `--top <n>`
- `--symbols <csv>`
- `--preset <name>`
- `--where <expr>`
- `--sort <field:direction>`
- `--record`
- `--raw`
- `--normalized`
- `--parquet` (planned; currently rejected)
- `--run-id <id>`
- `--data-dir <path>`

## `hls record`

Records selected public market data without opening the TUI.

Required behavior:

- Writes raw frames when `--raw` is enabled.
- Writes normalized replay JSONL when `--normalized` is enabled.
- Updates metadata registry and flushes on shutdown.
- Prints recording run ID.

Common options:

- `--top <n>`
- `--symbols <csv>`
- `--raw`
- `--normalized`
- `--parquet` (planned; currently rejected)
- `--run-id <id>`
- `--data-dir <path>`

## `hls screen`

Runs one screen over current live data or replayed data and prints table output.

Required behavior:

- Accepts preset or custom rule.
- Rejects invalid rules with a clear error.
- Supports JSON and table output.

Common options:

- `--preset <name>`
- `--where <expr>`
- `--sort <field:direction>`
- `--from <timestamp>`
- `--to <timestamp>`
- `--json`

## `hls replay`

Replays local recorded data.

Required behavior:

- Loads local raw or normalized data for the requested range.
- Rebuilds feature snapshots.
- Reports data gaps and incomplete windows.

Common options:

- `--run-id <id>`
- `--symbols <csv>`
- `--speed <factor>`
- `--preset <name>`
- `--where <expr>`
- `--data-dir <path>`

## `hls inspect`

Shows symbol-level details from current live state or recorded data.

Required behavior:

- Shows price references, top-of-book, flow buckets, returns, volatility, z-scores, and recent trades.
- Clearly marks missing or stale fields.

Common options:

- `<symbol-or-feed-id>`
- `--from <timestamp>`
- `--to <timestamp>`
- `--json`
