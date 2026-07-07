# Optional Local HTTP API Contract

The local API is optional for v1 and must remain read-only. It exposes current feature snapshots and health for local tooling. It is not a full web dashboard.

## `GET /health`

Returns current process, connection, subscription, storage, replay, and read-only safety state.

Response fields:

- `status`
- `read_only`
- `connections`
- `subscription_count`
- `last_message_age_ms`
- `lag_ms`
- `writer_backlog`
- `recording`
- `gap_count`

## `GET /symbols`

Returns current symbol metadata and optional current context values.

Query parameters:

- `top`
- `include`
- `exclude`

## `GET /screen`

Returns current feature snapshots after optional filter and sort.

Query parameters:

- `where`
- `preset`
- `sort`
- `limit`

Validation:

- Invalid `where` or `sort` returns a validation error and does not mutate the active TUI rule.

## `GET /symbol/{symbol_or_feed_id}`

Returns one symbol detail snapshot.

Response includes:

- symbol metadata
- price references
- top-of-book
- feature row
- recent trades when available
- staleness and incomplete-window reasons

## Safety Requirements

- No endpoint accepts order parameters.
- No endpoint accepts private keys or wallet credentials.
- No endpoint sends exchange actions.
- Bind to localhost by default.
