# Open Source Readiness Checklist

This file tracks the public-release package for `hlscreen`.

## Repository Health

- [x] MIT license file.
- [x] README with screenshots, safety boundary, install/build commands, and docs index.
- [x] Contribution guide.
- [x] Code of conduct.
- [x] Security policy.
- [x] Support policy.
- [x] Changelog.
- [x] Release checklist.
- [x] Deployment limitations document that does not claim a production service path.
- [x] Privacy note.
- [x] Threat model.
- [x] Roadmap.
- [x] GitHub issue templates.
- [x] Pull request template.
- [x] CI workflow.
- [x] Dependabot config.
- [x] Deterministic screenshot generator.
- [x] Committed screenshot assets.
- [x] Diagrammed architecture documentation.
- [x] Truthful production-readiness document with explicit non-goals.

## Validation

- [x] GitHub Actions green while the repo is private: post-merge CI run `28965215942` passed on `main` commit `45b9e7c`.
- [x] Current live all-symbol public-data smoke recorded in `docs/reports/2026-07-08-production-readiness-live-refresh.md`.
- [x] Public-readiness scan: `scripts/check-public-readiness.sh`.
- [x] Local release artifact/checksum/install smoke: `scripts/local-release-artifact-smoke.sh`.
- [x] CI release packaging gate includes static release contract tests, public-readiness scan, and local artifact smoke.
- [ ] Release tag created.
- [ ] Release binaries/checksums published.
- [ ] Multi-day supervised soak report published.

## Public Positioning

- `hlscreen` is read-only market-data infrastructure.
- It does not place orders.
- It does not manage wallets or credentials.
- It does not provide financial advice.
- Scores and presets are screening heuristics only.
