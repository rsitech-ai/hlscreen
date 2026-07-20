# Releasing

This project is pre-1.0. Use this checklist before tagging a public release.

## Pre-Release Checklist

1. Confirm scope and safety.
   - No wallet, private-key, signing, order, cancel, withdrawal, leverage, or exchange-action support.
   - No README, screenshot, or release note implies trading advice or profitability.
2. Run validation.
   ```bash
   scripts/check.sh release
   ```
   The release mode includes the complete pull-request gate, then runs the
   pinned advisory, dependency-policy, attribution, and GitHub Actions audits.
   For a faster iteration loop use `scripts/check.sh fast`; before review use
   `scripts/check.sh pr` (also the default when no mode is supplied).
   Install the policy tools with
   `cargo install cargo-audit --version 0.22.2 --locked`,
   `cargo install cargo-deny --version 0.20.2 --locked` when it is not already
   available and
   `cargo install cargo-about --version 0.9.1 --locked --features cli` for the
   deterministic attribution check. Install
   [`uv`](https://docs.astral.sh/uv/getting-started/installation/) to run the
   pinned `zizmor@1.26.1` workflow audit. That audit requires network access on
   its first run to resolve the exact pinned package; CI uses the same command.
   CI installs the exact Cargo tool versions above.
   The checker fetches only the locked graph first, then runs cargo-about with
   `--offline`; cargo-about 0.9.1 does not support a `no-clearly-defined`
   configuration field. Offline generation is the supported fail-closed mode
   that disables ClearlyDefined and other mutable network enrichment. The
   tradeoff is that a poorly packaged crate can expose less copyright detail
   than network enrichment would recover, so dependency upgrades require a
   manual notice review.
   `scripts/check-release-packaging.sh` runs the static release contract tests,
   the public-readiness scan, and the local artifact smoke described below.
   The release gate enforces
   `cargo audit --deny warnings --ignore RUSTSEC-2024-0436` after verifying the
   installed cargo-audit version.
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

GitHub Releases is the only supported binary distribution channel. Pull-request
and workflow artifacts are review candidates with limited retention, not public
releases or supported download channels. Do not redistribute workflow artifacts as releases. Source builds from a reviewed commit remain available under the
repository's Apache-2.0 license, but a binary is supported only after its archive,
checksum, SBOM, and provenance are published together on the corresponding
GitHub Release page.

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
- explicit per-job timeouts for release planning, native builds, global
  packaging, hosting, and announcement;
- version-locked Cargo registry builds for cargo-dist 0.32.0,
  cargo-auditable 0.7.5, and cargo-cyclonedx 0.5.5; Cargo verifies registry
  package checksums before compilation;
- a source archive and SHA-256 checksums;
- a CycloneDX XML SBOM generated with `cargo-cyclonedx`;
- dependency metadata embedded in release binaries with `cargo-auditable`;
- native-runner archive validation before upload: checksum, required notices,
  unpacked `--help`, doctor, fixture-live, and `cargo audit bin` metadata smoke;
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
- project `LICENSE`, `NOTICE`, `THIRD_PARTY_LICENSES.txt`, `THIRD_PARTY_NOTICES.md`,
  README, and changelog included in the archive;
- unpacked binary smoke with `hls --help`;
- read-only safety smoke with `hls doctor`;
- bounded fixture live smoke with `hls live --fixture-file ... --once`;
- explicit release notes stating known limitations and read-only scope.

The private-candidate surface gate downloads the exact workflow artifacts and
fails closed on unsafe archive paths, checksum mismatches, malformed manifests
or SBOM XML, missing installers/notices, and missing native validation steps.
Artifact names alone are not release evidence.

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
| Pull-request candidate artifacts | Configured; hosted proof blocked by GitHub Actions billing | Builds are defined for four targets but have not executed on the current hosted account |
| GitHub Release artifacts | Apple Silicon macOS archive and checksum published for `v0.1.0`; unsigned and unnotarized | Other configured targets remain source-build only until native-runner package proof exists; Developer ID Application signing is not currently proven |
| Shell installer | Configured by cargo-dist; unpublished | Do not advertise until a hosted release build verifies it |
| PowerShell installer | Configured by cargo-dist; unpublished | Do not advertise until a hosted Windows release build verifies it |
| Provenance | Configured for tag artifacts; unproven | Requires a successful eligible hosted tag workflow and downloaded attestation verification |

Release docs must not claim binary installation is public-ready until a reviewed
artifact and checksum exist on a completed release page.

The GitHub release workflow starts from the pinned `dist` generator. Repository
tooling applies and verifies deterministic post-generation corrections for
least privilege, SBOM upload, concurrency, cache/container safety, versioned
tool installation, and shell-expression isolation. After editing
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
- Schema-versioned normalized-event and feature/confidence Parquet export exists
  with manifests and DuckDB smoke; replay currently supports normalized-event
  Parquet only.
- The `v0.1.0` Apple Silicon macOS archive is locally built and smoke-tested;
  hosted multi-platform artifacts, installers, SBOM publication, and provenance
  remain blocked until GitHub Actions jobs can execute successfully.
- This is not trading advice and does not execute orders.
