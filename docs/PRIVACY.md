# Privacy

`hlscreen` is designed as a local-first, read-only public market-data tool.

## Data It Reads

- Public Hyperliquid spot metadata from read-only public REST endpoints.
- Public Hyperliquid market-data WebSocket envelopes in bounded live mode.
- Local fixture files and local replay files.

## Data It Writes

Depending on the command and flags, `hlscreen` can write these local operator
artifacts:

- `config.toml` created by `hls init`; current runtime commands still use
  explicit CLI flags, while `hls doctor` validates this file's safety settings.
- A bounded doctor filesystem probe, created and removed under the selected data
  directory.
- `tui-preferences.toml` with display-only layout preferences.
- Compressed raw public WebSocket messages.
- Normalized replay JSONL events.
- A local `hls.sqlite` metadata registry, including recording metadata,
  confidence-parity baselines, and schema-versioned candle-cache tables.
- Local alert history such as `alerts.jsonl` when `--alert-history-file` is used.
- Schema-versioned local analog index JSON when `--write-index` is used.
- Analytical Parquet event/feature datasets and their `schema.json` schema
  manifest when export is requested (the schema manifest is local metadata).

These files are local operator artifacts and should not be committed to git.

## Data It Must Not Collect

`hlscreen` does not need and must not request:

- Private keys.
- Seed phrases.
- Wallet permissions.
- Exchange API secrets.
- Trading credentials.
- Private account streams.
- Order, position, or balance permissions.

## Network Behavior

The implemented public REST metadata commands call Hyperliquid public read-only
endpoints over HTTPS. Bounded live mode connects to public market-data feeds
over WSS; cleartext WS/HTTP is accepted only for literal loopback or localhost
fixtures. Redirects from public REST calls are not followed, and successful
REST bodies are capped at 8 MiB. Fixture-backed commands do not require network
access.

Live mode does not request user-specific streams, account data, wallet permissions, or exchange-action endpoints.

## Public Issues

Do not paste secrets, account addresses, private endpoints, or local raw data captures into public issues.
