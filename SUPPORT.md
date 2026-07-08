# Support

`hlscreen` is currently an early open-source Rust project for read-only Hyperliquid spot screening and local recording.

## Before Asking For Help

Run:

```bash
cargo --version
cargo test --workspace --all-features
./target/debug/hls doctor --live --json
```

If the problem is fixture-backed, include the exact fixture command. If the problem is live REST metadata, include whether `hls symbols --top 5` works from your network.

## Where To Ask

- Bugs: open a GitHub issue with the bug report template.
- Feature requests: open a GitHub issue with the feature request template.
- Security issues: follow `SECURITY.md`, not public issues.

## What This Project Does Not Provide

- Trading advice.
- Profitability claims.
- Execution support.
- Wallet or key-management support.
- Exchange account troubleshooting.
