# Contributing

Thanks for considering a contribution to `hlscreen`.

This project is trading-adjacent infrastructure, so the contribution bar is intentionally higher than a demo CLI. Changes should preserve read-only behavior, clear failure modes, deterministic tests, and honest language about what the tool does.

## Ground Rules

- Keep `hlscreen` read-only. Do not add wallet, private-key, signing, order, cancel, withdrawal, leverage, or execution surfaces.
- Do not present scores, presets, or screen rows as trading recommendations or profitability claims.
- Keep public Hyperliquid API assumptions documented and test-covered.
- Prefer small focused PRs with tests and a clear rollback path.
- Do not commit secrets, private endpoints, account identifiers, local `.hls/` data, raw captures, or generated databases.

## Local Setup

```bash
git clone https://github.com/s1korrrr/hlscreen.git
cd hlscreen
cargo build --workspace --all-features --locked
```

The repository is pinned by `rust-toolchain.toml` and currently targets Rust 1.88 or newer.

## Validation Before Opening A PR

Run the full local gate before asking for review:

```bash
scripts/check.sh pr
```

`scripts/check.sh fast` is the iteration loop: formatting, locked workspace
check and tests, and diff hygiene. The default `scripts/check.sh` mode is `pr`,
which adds full clippy, release, rustdoc, deterministic screenshot, and release
packaging validation. Maintainers use `scripts/check.sh release` before tagging;
it adds the pinned advisory, dependency-policy, attribution, and workflow audits.

For changes touching screenshots or CLI output, regenerate screenshots:

```bash
python3 scripts/generate-screenshots.py
```

For storage or replay work, run a fixture smoke:

```bash
tmpdir="$(mktemp -d /tmp/hlscreen-smoke.XXXXXX)"
./target/debug/hls record --symbols @107 --fixture-file tests/fixtures/hyperliquid/ws_mock_live.ndjson --raw --normalized --run-id smoke --data-dir "$tmpdir"
./target/debug/hls replay --data-dir "$tmpdir" --run-id smoke
```

## Pull Request Checklist

- The diff is focused and has no unrelated refactors.
- New behavior has tests that would fail without the change.
- Public docs are updated when behavior or commands change.
- `README.md` and screenshot assets still match the actual CLI output.
- The read-only/security boundary is unchanged or explicitly tightened.
- Known limitations are documented rather than hidden.

## Issue Triage

Please include exact commands, expected behavior, actual behavior, platform, Rust version, and whether the path used live public REST or only fixtures.

Security issues should follow `SECURITY.md`, not public issue comments.
