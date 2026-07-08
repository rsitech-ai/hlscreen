# Contract: Read-Only Extension Surface

## Purpose

Define the safe boundary for future custom features or panels before any plugin runtime is added.

## Contract Version

Initial version: `hls-extension-v0`

This is a design contract. A runtime is not required in the first implementation slice.

## Input Shape

```json
{
  "contract_version": "hls-extension-v0",
  "row": {
    "symbol": "@107",
    "price": 68.0,
    "spread_bps": 4.2
  },
  "confidence": {
    "state": "trusted",
    "score": 0.96,
    "reasons": []
  },
  "recent_trades": [],
  "recent_bbo": [],
  "metadata": {},
  "capabilities": {
    "network": false,
    "filesystem": false,
    "private_account": false,
    "state_mutation": false,
    "execution": false
  }
}
```

## Output Shape

```json
{
  "contract_version": "hls-extension-v0",
  "panel_lines": [
    "custom panel output"
  ],
  "feature_fields": {
    "custom_score": 12.3
  },
  "warnings": [],
  "requested_capabilities": []
}
```

## Safety Rules

- Extensions are read-only.
- Extensions do not receive credentials, private account data, wallet data, order data, or signing capabilities.
- Extensions cannot place orders or mutate core market state.
- Network and filesystem are disabled by default.
- Outputs must be bounded in bytes, field count, and render lines.
- Unknown output fields are ignored or rejected according to versioned schema rules.
- Runtime failures must degrade the extension output without breaking core live ingestion.

## Validation

- Contract tests must load fixture input and validate accepted output.
- Tests must reject capability requests for network, filesystem, private account data, state mutation, and execution.
- Tests must reject oversized outputs.
- Replay must produce deterministic extension output for deterministic input if a runtime is later added.

## Future Runtime Criteria

A WASM/plugin runtime can be considered only after:
- the contract above is tested
- capability denial is enforced in tests
- execution timeouts are enforced
- output size limits are enforced
- docs explain plugin trust boundaries
