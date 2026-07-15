# Test Fixture Policy

Committed fixtures are small, reviewable test inputs and expected outputs. They
must be safe to publish and must not be treated as live production captures.

## Classifications

- **Synthetic and minimized fixtures:** `hyperliquid/` contains hand-built or
  minimized public REST and WebSocket payload shapes. Microstructure scenario
  inputs under `microstructure/` are deterministic synthetic cases designed to
  exercise one bounded behavior.
- **Derived output fixtures:** expected schema manifests, benchmark results,
  scoring rows, metadata enrichments, and Wasm validation cases under
  `microstructure/` are deterministic outputs or expectations derived from the
  synthetic inputs.
- **Validation-report fixtures:** `operations/` contains deliberately valid or
  invalid synthetic report documents used to test fail-closed report
  validation. They are not claims about a live run.

## Prohibited Content

Never commit credentials, real accounts or wallets, private streams,
unredacted user data, private endpoints, API tokens, signing material, or raw
local captures. Public payload examples must be minimized to the fields needed
by the test and reviewed before commit. When provenance is uncertain, do not
commit the file.
