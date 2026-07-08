# Privacy

`hlscreen` is designed as a local-first, read-only public market-data tool.

## Data It Reads

- Public Hyperliquid spot metadata from read-only public REST endpoints.
- Public Hyperliquid market-data WebSocket envelopes when live network mode is implemented.
- Local fixture files and local replay files.

## Data It Writes

When recording is enabled, `hlscreen` writes local files under the configured data directory:

- Compressed raw public WebSocket messages.
- Normalized replay JSONL events.
- A local SQLite metadata registry.

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

The implemented public REST metadata command calls Hyperliquid public read-only endpoints. Fixture-backed commands do not require network access.

Real live WebSocket mode is intentionally not implemented in the current slice. When added, it should connect only to public market-data subscriptions unless the project scope is explicitly changed.

## Public Issues

Do not paste secrets, account addresses, private endpoints, or local raw data captures into public issues.
