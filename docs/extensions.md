# Extensions

`hlscreen` v0.1 defines a read-only extension contract and a bounded row
annotation runtime for local use.

The goal of the contract is to let future contributors design row annotation,
score annotation, and health annotation plugins without opening unsafe surfaces
in the host application. The current runtime executes only
`feature_snapshot -> row_annotations` entrypoints through `hls extension`;
live integration, score, health, TUI-panel, and plugin discovery surfaces remain
future work.

## Current Boundary

Allowed in v0.1:

- Manifest validation.
- Relative `.wasm` artifact paths.
- SHA-256 artifact hashes.
- Bounded memory declarations.
- Bounded Wasmtime execution for row annotations.
- Host-managed immutable `FeatureSnapshot` input.
- Bounded JSON row-annotation output.
- Input kinds for feature snapshots, screen rows, and health snapshots.
- Output kinds for annotations only. Only row annotations execute today.

Not allowed in v0.1:

- Network access.
- Filesystem access.
- Private stream or account data.
- Wallet access.
- Trading, order, cancel, leverage, withdrawal, or exchange-action access.
- Host functions that mutate local state.
- WASM imports or WASI capabilities.

This mirrors the conservative part of common WASM plugin manifest systems:
runtime capabilities must be explicit. For this project, the first contract is
stricter than a general plugin manifest because the workstation is public-data
only.

## Manifest Shape

```json
{
  "schema_version": 1,
  "name": "gap-labeler",
  "version": "0.1.0",
  "description": "Annotates rows with local replay-gap context.",
  "wasm": {
    "path": "extensions/gap-labeler.wasm",
    "sha256": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "memory_max_pages": 16
  },
  "permissions": {
    "read_only": true,
    "network": false,
    "filesystem": false,
    "private_data": false,
    "trading": false,
    "allowed_hosts": [],
    "allowed_paths": []
  },
  "entrypoints": [
    {
      "name": "annotate_row",
      "input": "feature_snapshot",
      "output": "row_annotations"
    }
  ]
}
```

## Invocation

Run a local row-annotation plugin against fixture or replayed public data:

```bash
./target/debug/hls extension \
  --manifest /path/to/plugin-manifest.json \
  --fixture-file tests/fixtures/hyperliquid/ws_mock_live.ndjson \
  --symbol @107 \
  --json
```

For replayed data, replace `--fixture-file` with `--data-dir <dir> --run-id
<run-id>`. The command validates the manifest, verifies the WASM SHA-256,
rejects unsafe permissions before loading the artifact, executes without host
imports, and emits annotations only.

Live rendering does not currently load extensions.

## Validation

Run the contract tests with:

```bash
cargo test -p hls-core --test extension_contract
cargo test -p hls-cli --test extension_command
cargo test -p hls-cli --test live_mock live_once_executes_safe_plugin_annotations
```

The host rejects manifests that request network, filesystem, private data, or
trading permissions. It also rejects absolute paths, path traversal, missing
hashes, empty entrypoint lists, WASM import declarations, hash mismatches,
oversized input/output, and unsafe annotation wording.

## Future Work

- Plugin discovery and packaging.
- TUI panel plugins.
- Score and health annotation runtime paths.
- Wall-clock supervision beyond deterministic fuel bounds.
- Any host callback API, if ever added, after a separate security design.
