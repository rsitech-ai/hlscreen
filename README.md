# hlscreen

`hlscreen` is a read-only Rust workspace for Hyperliquid spot market-data screening and local recording.

Current implemented slice:

- Cargo workspace with `hls-core`, `hls-hyperliquid`, `hls-cli`, and placeholder crates for later store/features/screen/TUI/server work.
- Validated config loading and read-only safety guardrails.
- Hyperliquid public REST metadata parsing for `spotMeta` and `spotMetaAndAssetCtxs`.
- Fixture-backed `hls init`, `hls doctor`, and `hls symbols`.

Out of scope for v1:

- Wallet connection
- Private keys
- Order placement
- Leverage or execution controls
- Predictive signals or profitability claims

## Build And Test

```bash
cargo build --workspace
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Local Smoke

```bash
./target/debug/hls init --data-dir /tmp/hlscreen-smoke
./target/debug/hls doctor --data-dir /tmp/hlscreen-smoke
./target/debug/hls symbols --top 2 --asset-contexts-file tests/fixtures/hyperliquid/spot_meta_and_asset_ctxs.json
```

The fixture-backed `symbols` command must print `READ-ONLY` and preserve both display symbols and Hyperliquid feed identifiers such as `HYPE/USDC` and `@107`.
