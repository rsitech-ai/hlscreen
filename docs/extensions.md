# Extensions

`hlscreen` v0.1 defines a read-only extension contract. It does not load or
execute extension WASM yet.

The goal of the contract is to let future contributors design row annotation,
score annotation, and health annotation plugins without opening unsafe surfaces
in the host application.

## Current Boundary

Allowed in v0.1:

- Manifest validation.
- Relative `.wasm` artifact paths.
- SHA-256 artifact hashes.
- Bounded memory declarations.
- Input kinds for feature snapshots, screen rows, and health snapshots.
- Output kinds for annotations only.

Not allowed in v0.1:

- Network access.
- Filesystem access.
- Private stream or account data.
- Wallet access.
- Trading, order, cancel, leverage, withdrawal, or exchange-action access.
- Host functions that mutate local state.

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

## Validation

Run the contract tests with:

```bash
cargo test -p hls-core --test extension_contract
```

The host rejects manifests that request network, filesystem, private data, or
trading permissions. It also rejects absolute paths, path traversal, missing
hashes, and empty entrypoint lists.

## Future Runtime Work

A future runtime implementation should still keep this contract as the gate
before any WASM engine is initialized:

1. Validate the manifest.
2. Verify the WASM artifact hash.
3. Provide only immutable, bounded input payloads.
4. Accept only annotation outputs.
5. Refuse host functions that mutate files, network state, screen rules, or
   market-data capture.
