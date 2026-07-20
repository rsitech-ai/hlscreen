# Support

`hlscreen` is currently an early open-source Rust project for read-only Hyperliquid spot screening and local recording.

## Before Asking For Help

Run:

```bash
cargo --version
rustc --version --verbose
scripts/check.sh fast
./target/debug/hls doctor --live --json
```

Include your OS/version, CPU architecture, terminal and shell versions, the
exact command, and whether it ran in a real TTY. If the problem is
fixture-backed, include the exact fixture command. If the problem is live REST
metadata, include whether `hls symbols --top 5` works from your network. Redact
local paths, account identifiers, private endpoints, and secrets.

## Where To Ask

- Questions and usage help: use the [Discussions Q&A category](https://github.com/rsitech-ai/hlscreen/discussions/categories/q-a).
- Bugs: open a GitHub issue with the bug report template. Reproducible defects belong in Issues, not Discussions.
- Feature requests: open a GitHub issue with the feature request template.
- Security issues: follow `SECURITY.md`, not public issues.

## What This Project Does Not Provide

- Trading advice.
- Profitability claims.
- Execution support.
- Wallet or key-management support.
- Exchange account troubleshooting.
