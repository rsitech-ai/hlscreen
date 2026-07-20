# Reflection: Production-ready open-source release hardening

## Task

- **ID / title:** `hlscreen` production-ready open-source release hardening
- **Date:** 2026-07-20
- **Scope:** Full repository, inherited `origin/main..e287955` candidate, release
  automation, runtime network boundaries, product truth, evidence, packaging,
  and hosted launch gates.
- **Authority boundary:** Safe local edits, tests, read-only public API use, and
  local commits were authorized. Pushes, PRs, visibility/settings changes,
  ownership transfer, tags, releases, license changes, and history rewrites were
  not authorized.

## Success and Risk

- **Success criteria:** Exact-source local release gate, redacted all-ref secret
  scan, real 15-minute live/replay evidence, actual cargo-dist artifact-byte
  smoke, and clean-checkout quickstart all pass; every external/manual blocker
  remains explicit.
- **Hypothesis 1:** The inherited candidate was technically strong but public
  claims and release evidence had drifted from actual callers and final runtime
  inputs.
- **Hypothesis 2:** Release automation and network defaults contained avoidable
  supply-chain and resource-boundary risk even though ordinary tests were green.
- **Hypothesis 3:** Local readiness could be proven independently from GitHub
  publication readiness, which would remain blocked by owner/settings/hosted
  evidence.
- **Rollback path:** Revert the task commits; the inherited candidate remains
  at `e287955`. No remote or published state was changed.

## Candidate Directions

| Candidate | Expected benefit | Main risk | Evidence before choice | Decision |
|---|---|---|---|---|
| A. Incremental, test-backed hardening on the inherited candidate | Preserves 18 audited commits, closes concrete blockers, and keeps rollback reviewable | Larger combined diff still needs full-gate and independent review | Baseline release gate was green, while three independent audits found bounded, reproducible gaps | Selected |
| B. Rebuild from `origin/main`, rewrite history, or transfer ownership during the task | Cleaner-looking public lineage or final owner in one step | Discards audited fixes or exceeds authority through destructive/external mutation | No active secret required a rewrite; owner and history exposure decisions were approval-gated | Rejected |

## Evidence

- **First meaningful failure signal:** The exact gitleaks gate lacked local
  8.30.1, and the first supervised soak failed closed with only 850 MiB free
  against its 2 GiB minimum.
- **Commands or runtime checks:** `scripts/check.sh release`; pinned gitleaks
  8.30.1 `scripts/check-history-secrets.sh`; schema-v2 902-second supervised
  soak and two replays; `dist plan`; native `dist build`; checksum/archive,
  unpacked binary, doctor, fixture-live, and `cargo audit bin`; clean local clone
  build/quickstart; live `check-public-surface.sh private-candidate`.
- **What the evidence ruled in or out:** Repository-local runtime, package, and
  source-archive readiness are proven on Apple Silicon. Hosted CI/release,
  non-native target artifacts, branch protections, code scanning, private
  vulnerability reporting, visibility, and canonical owner are not proven or
  configured at the candidate SHA.

## Decision

- **Root cause or remaining unknown:** Earlier gates emphasized static
  configuration and ancestor evidence; they did not bind runtime source to the
  binary, inspect cargo-dist bytes, or force public docs to distinguish tested
  library primitives from wired product behavior. The independent final review
  also found that hosted acceptance checked artifact names rather than bytes and
  allowed unbounded release jobs. Those local gaps are fixed; hosted launch
  state remains outside local authority.
- **Retained fix / direction:** Deterministic runtime-source and binary hashes,
  clean-tree soak enforcement, schema-v2 evidence, Cargo registry tool builds,
  bounded REST and WS validation, complete CLI help, truthful privacy/config/
  feature docs, source archive export policy, and canonical package checks.
- **Why alternatives were rejected:** A history rewrite was unnecessary for the
  accepted metadata categories; deleting ambiguous journals was avoided in
  favor of explicit historical banners plus `export-ignore`; external ownership
  and GitHub settings cannot be inferred or mutated safely.
- **Residual risk:** Windows, Linux, Intel macOS, hosted fork CI, artifact
  attestations, rulesets, public signed-out surfaces, and the first tagged
  release remain external/manual gates. Native runner validation is configured
  but cannot be claimed until exact-SHA hosted jobs execute. The configured
  canonical owner still needs approval.
- **Rollback trigger:** Revert if hosted candidate builds reveal a cross-target
  regression, if the owner rejects the recorded history/legal exposure, or if
  the final independent review finds a release-blocking local defect.

## Reusable Lesson

- **Pattern to retain:** Bind operational evidence to a content fingerprint and
  exact binary, then keep documentation-only closeout commits outside that
  fingerprint while revalidating the package gate.
- **Pattern to avoid:** Treating a green source build, a named artifact, or an
  ancestor soak report as proof of the actual release candidate.
- **Where it applies next:** Any RSI Tech CLI/TUI release with generated
  workflows, local-first persistence, live public data, or multi-platform
  packaging.
