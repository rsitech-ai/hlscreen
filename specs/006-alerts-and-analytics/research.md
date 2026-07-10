# Research: Alerts And Analytics

## Plugin Sandbox Runtime

**Decision**: Use Wasmtime directly for the first read-only plugin execution path.

**Rationale**: The current extension contract needs a narrow host-controlled
runtime for row annotations, not a broad plugin ecosystem. Wasmtime exposes the
controls this slice needs directly: engine configuration, import inspection,
fuel metering, memory declarations, typed function calls, and host-managed
input/output buffers. The implementation enables fuel consumption, rejects all
WASM imports, verifies the manifest SHA-256 before compilation, enforces
manifest memory page limits, writes a bounded immutable `FeatureSnapshot`
payload into guest memory, and accepts only bounded row-annotation output.
Wasmtime documents `Config::consume_fuel` as a deterministic way to halt
infinitely executing WebAssembly when fuel runs out, with the store requiring
fuel to be added before execution.

**Alternatives considered**:

- Extism. It remains a reasonable future plugin ecosystem candidate, but its
  manifest/runtime model is broader than the current v0.1 need. `hlscreen`
  wants no host functions, no filesystem, no network, no private data, and no
  mutation in this slice. A direct Wasmtime wrapper keeps that boundary easier
  to audit.
- Native Rust dynamic plugins. Rejected because Rust ABI stability and process
  isolation are a poor fit for untrusted public contributions.
- TUI-only scripted annotations. Rejected because it would not exercise the
  extension contract or sandbox constraints.

**Residual risks**:

- The CLI command is a row-annotation runner, not a complete plugin ecosystem.
- There is no plugin discovery, installation, marketplace, WASI filesystem, host
  callback API, TUI panel plugin API, or live automatic plugin execution.
- Fuel is deterministic instruction limiting; it is not a wall-clock supervisor
  for host calls. The current runtime avoids that class by rejecting all imports
  and not providing WASI or host functions.
- Plugin outputs are local annotations only and must not be treated as trading
  signals, execution instructions, or advice.

References:

- Wasmtime `Config` API: https://docs.wasmtime.dev/api/wasmtime/struct.Config.html
- Wasmtime 38 Rust API: https://docs.rs/wasmtime/38.0.4/wasmtime/
- Extism Manifest API: https://docs.rs/extism/latest/extism/struct.Manifest.html
