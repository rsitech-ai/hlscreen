# Releasing

This project is pre-1.0. Use this checklist before tagging a public release.

## Pre-Release Checklist

1. Confirm scope and safety.
   - No wallet, private-key, signing, order, cancel, withdrawal, leverage, or exchange-action support.
   - No README, screenshot, or release note implies trading advice or profitability.
2. Run validation.
   ```bash
   cargo fmt --all -- --check
   cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
   cargo test --workspace --all-features --locked
   cargo build --release --workspace --all-features --locked
   RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps --locked
   cargo audit --deny warnings --ignore RUSTSEC-2024-0436
   cargo deny check licenses sources
   scripts/check-third-party-licenses.sh
   scripts/check-release-packaging.sh
   git diff --check
   python3 scripts/generate-screenshots.py --check
   ```
   Install the policy tools with
   `cargo install cargo-deny --version 0.20.2 --locked` when it is not already
   available and
   `cargo install cargo-about --version 0.9.1 --locked --features cli` for the
   deterministic attribution check. CI installs these exact versions.
   The checker fetches only the locked graph first, then runs cargo-about with
   `--offline`; cargo-about 0.9.1 does not support a `no-clearly-defined`
   configuration field. Offline generation is the supported fail-closed mode
   that disables ClearlyDefined and other mutable network enrichment. The
   tradeoff is that a poorly packaged crate can expose less copyright detail
   than network enrichment would recover, so dependency upgrades require a
   manual notice review.
   `scripts/check-release-packaging.sh` runs the static release contract tests,
   the public-readiness scan, and the local artifact smoke described below.
   `RUSTSEC-2024-0436` is a narrow exception for the unmaintained `paste`
   proc-macro currently pulled by Apache Parquet 59.1.0. It is not a known
   vulnerability; every vulnerability and all other warnings remain denied.
   Remove the exception when Parquet drops that transitive dependency.
3. Run fixture smokes.
   ```bash
   tmpdir="$(mktemp -d /tmp/hlscreen-release.XXXXXX)"
   ./target/debug/hls init --data-dir "$tmpdir"
   ./target/debug/hls doctor --data-dir "$tmpdir"
   ./target/debug/hls live --symbols @107 --fixture-file tests/fixtures/hyperliquid/ws_mock_live.ndjson --preset thin_books --once
   ./target/debug/hls live --symbols HYPE/USDC --duration-secs 15 --refresh-secs 5 --tui --record --raw --normalized --run-id release-live --data-dir "$tmpdir"
   ./target/debug/hls record --symbols @107 --fixture-file tests/fixtures/hyperliquid/ws_mock_live.ndjson --raw --normalized --run-id release --data-dir "$tmpdir"
   ./target/debug/hls replay --data-dir "$tmpdir" --run-id release
   ```
4. Update public docs.
   - `README.md`
   - `CHANGELOG.md`
   - `docs/architecture.md`
   - `docs/data-format.md`
   - `docs/feature-definitions.md`
   - screenshots in `docs/assets/screenshots/`
5. Create and review a release PR.
6. Tag only after `main` is green.

## Release Packaging Dry Run

Release packaging is configured with `cargo-dist` in `dist-workspace.toml`.

Install the pinned version:

```bash
cargo install --root /tmp/hlscreen-dist cargo-dist --version 0.32.0 --locked
export PATH="/tmp/hlscreen-dist/bin:$PATH"
```

Preview the release plan:

```bash
dist plan
```

Build local distributable artifacts:

```bash
dist build
```

No release secrets are required for the local dry run or pull-request release
build. Pull requests build and upload candidate artifacts to the workflow run,
but cannot execute the publication job. Tag pushes are the only event that can
publish a GitHub Release.

## Supply-Chain Contract

The release workflow provides:

- fixed supported runner labels and full-commit SHA pins for every action;
- `persist-credentials: false` on every checkout;
- workflow concurrency limits and a pinned pedantic zizmor security gate;
- a cargo-deny policy across every supported release target that rejects
  unapproved licenses, registries, and Git dependencies;
- deterministic third-party dependency attribution generated from `Cargo.lock`
  with pinned cargo-about 0.9.1 and no mutable ClearlyDefined enrichment;
- read-only top-level permissions, with `contents`, `id-token`, and
  `attestations` write permissions scoped only to the tag-only host job;
- release build caching disabled, no dynamic container image, and no
  expression interpolation directly into release shell commands;
- version-pinned cargo-dist 0.32.0 and cargo-auditable 0.7.5 installers;
- a source archive and SHA-256 checksums;
- a CycloneDX XML SBOM generated with `cargo-cyclonedx`;
- dependency metadata embedded in release binaries with `cargo-auditable`;
- GitHub artifact attestations for published tag artifacts.

These controls follow GitHub's official
[Actions security hardening guidance](https://docs.github.com/en/code-security/tutorials/secure-your-organization/protect-against-threats)
and cargo-dist's official
[configuration reference](https://axodotdev.github.io/cargo-dist/book/reference/config.html).

GitHub artifact attestations for private repositories require GitHub Enterprise
Cloud. While this repository remains private outside Enterprise Cloud, do not
push a release tag: make the repository public first or explicitly disable the
attestation requirement in a reviewed release change. See the official
[GitHub artifact attestation documentation](https://docs.github.com/en/actions/how-tos/secure-your-work/use-artifact-attestations/use-artifact-attestations)
and [cargo-dist attestation documentation](https://axodotdev.github.io/cargo-dist/book/supplychain-security/attestations/github.html).

## Artifact Checklist

Every release candidate needs:

- one archive per supported target triple;
- a SHA-256 checksum file beside each archive;
- the source archive and CycloneDX SBOM;
- auditable dependency metadata in each Rust binary;
- project `LICENSE`, `THIRD_PARTY_LICENSES.txt`, `THIRD_PARTY_NOTICES.md`,
  README, and changelog included in the archive;
- unpacked binary smoke with `hls --help`;
- read-only safety smoke with `hls doctor`;
- bounded fixture live smoke with `hls live --fixture-file ... --once`;
- explicit release notes stating known limitations and read-only scope.

## Local Artifact Smoke

For local proof before a tag, run:

```bash
cargo build --release --workspace --all-features
scripts/local-release-artifact-smoke.sh
```

The script writes a local archive and checksum under
`target/local-release-smoke/`, verifies the checksum, unpacks the archive, and
runs the binary from the unpacked directory. This is not a published release; it
is install-smoke proof for the reviewed local artifact.

## Release Artifact Status

| Channel | Status | Proof / Next step |
| --- | --- | --- |
| Source build | Implemented | `cargo build --release --workspace --all-features` |
| Pull-request candidate artifacts | Configured; clean-runner proof pending on this PR | Builds four target archives, source, checksums, installers, and SBOM without publishing |
| GitHub Release artifacts | Local artifact/checksum proof implemented; public publication pending | Requires public/Enterprise attestation support and a reviewed `v*` tag workflow |
| Shell installer | Configured by cargo-dist; unpublished | Verify from PR artifact, then from the reviewed tag run |
| PowerShell installer | Configured by cargo-dist; unpublished | Verify from PR artifact, then from the reviewed tag run |
| Provenance | Configured for tag artifacts; unproven | Requires a successful eligible tag workflow and downloaded attestation verification |

Release docs must not claim binary installation is public-ready until a reviewed
artifact and checksum exist on a completed release page.

The GitHub release workflow starts from the pinned `dist` generator. Repository
tooling applies and verifies deterministic post-generation corrections for
least privilege, SBOM upload, concurrency, cache/container safety, versioned
installers, and shell-expression isolation. After editing
`dist-workspace.toml`, run:

```bash
python3 scripts/harden-generated-release-workflow.py --regenerate
python3 scripts/harden-generated-release-workflow.py --check
dist plan
```

`--regenerate` temporarily removes only the exact CI `allow-dirty` line, invokes
the pinned `dist` binary from `PATH`, restores the config in a `finally` path,
and applies the reviewed workflow hardening. `--dist-bin` can name an explicit
pinned binary path.

`allow-dirty = ["ci"]` is intentionally scoped to this reviewed generated-file
exception. The static release contract tests and zizmor fail if permissions,
action pins, SBOM upload, event boundaries, shell interpolation, cache/container
safety, or host-job privileges drift.

## Tagging

```bash
git tag -a v0.1.0 -m "hlscreen v0.1.0"
git push origin v0.1.0
```

## Release Notes

Release notes should include:

- What changed.
- Validation run.
- Known limitations.
- Read-only safety statement.
- Upgrade notes or migration steps.

## Current Known Limitations

- Live WebSocket mode is bounded and read-only. It records reconnect gaps and can append coarse public candle rows, but those rows do not repair missing trades/BBO or remove the gap confidence penalty.
- HTTP behavior is a bounded localhost preview over in-memory state, not a supported long-running daemon or production service deployment.
- Local alert evaluation and bounded TUI history are implemented, but delivery,
  escalation, ownership, and durable retention are not an operational alert engine.
- Current microstructure analytics are research formulas/proxies, not a validated canonical production metric suite.
- Initial normalized-event Parquet export exists with schema manifests, JSONL parity, and DuckDB smoke. Full schema-versioned feature/confidence Parquet datasets are still planned.
- Release packaging is drafted and locally smoke-tested; the first public GitHub release is not proven until a reviewed `v*` tag run succeeds.
- This is not trading advice and does not execute orders.
