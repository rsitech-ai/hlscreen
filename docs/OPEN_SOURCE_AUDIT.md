# Open-Source Publication Audit

This is the evidence log for the first public hlscreen release. A checked item
records repository-side proof only; GitHub account, visibility, protection, and
release gates remain separate until they are verified on the exact candidate
commit.

## Third-party attribution review

Review date: 2026-07-15

- The locked workspace graph contains crates.io packages only; no Git or
  unapproved registry dependency is present.
- `cargo-about 0.9.1` generated `THIRD_PARTY_LICENSES.txt` from the locked,
  all-feature, four-target workspace graph in offline fail-closed mode.
- The generator template emits unescaped full license text. The committed file
  is checked byte-for-byte by `scripts/check-third-party-licenses.sh`.
- Every locked dependency is scanned recursively for packaged `NOTICE*` files.
  Apache Parquet 59.1.0 packages `NOTICE.txt`; cfg_aliases 0.1.1 and 0.2.1
  package identical `NOTICES.md` files. All three detected files are tracked in
  `third_party/notices/manifest.json`, preserved byte-for-byte, and embedded in
  `THIRD_PARTY_NOTICES.md`.
- Packages declaring Apache-2.0 or `Apache-2.0 WITH LLVM-exception` were also
  inspected for packaged `COPYRIGHT` files. No additional Apache `NOTICE`
  obligation was found beyond Parquet's tracked notice.
- Packaged copyright files found in dual-licensed crates do not add a separate
  redistribution notice beyond their selected full license text. This check
  must be repeated whenever `Cargo.lock` changes.
- Vendored GitHub Spec Kit 0.11.1 material is covered separately by
  `THIRD_PARTY_NOTICES.md` and `third_party/spec-kit/LICENSE`; the same complete
  MIT notice is embedded in `THIRD_PARTY_LICENSES.txt` for binary archives.

This is an engineering attribution review, not legal advice. Offline generation
avoids mutable enrichment but cannot recover extra copyright information from a
poorly packaged dependency. Dependency upgrades therefore require both the
deterministic generator check and a fresh packaged-notice review.

## Publication gates

- [ ] Canonical local release gate passes on the candidate commit.
- [ ] Redacted full-history secret scan covers remote branches and pull-request
  head refs.
- [ ] Private hosted CI and release-plan artifacts pass on the exact candidate
  commit.
- [ ] GitHub billing/spending state permits jobs to execute.
- [ ] Repository visibility, ruleset/protection, and security features are
  verified after publication.
- [ ] `v0.1.0` artifacts, checksums, SBOMs, and attestations are verified.

## Hosted surface snapshot

Snapshot date: 2026-07-15

- Repository: `s1korrrr/hlscreen`, private, default branch `main`.
- Audited private base: `9cdc32822636bd7159fbc87517e6ea05b38cfdf9`.
- Local closeout candidate at snapshot: `ab2f093` (not pushed; this value must
  be replaced by the final candidate's full SHA before hosted proof).
- Latest `main` CI run: `29411491370`, failed before executing job steps because
  of GitHub billing/spending state. It is not valid hosted proof.
- Hosted inventory: 14 branches, six open Dependabot PRs, no issues, tags,
  releases, Pages site, deployments, environments, deploy keys, webhooks, or
  configured repository/Dependabot/Codespaces secret or variable names.
- Actions inventory: 396 historical workflow runs across 317 head SHAs and 90
  unexpired artifacts; Actions currently allows all actions. Default
  `GITHUB_TOKEN` permissions are read-only and cannot approve pull-request
  reviews.
- Actions-history decision: DELETE_NON_CANDIDATE_RUNS_BEFORE_PUBLIC. Retain only
  final-candidate runs after their logs pass the in-memory privacy scan; remove
  all older runs and artifacts before visibility changes so their logs do not
  become public.
- Security inventory: dependency alerts, code scanning, secret scanning, and
  push protection are not enabled while the repository is private on the
  current plan. `main` is not protected; GitHub requires a paid private plan or
  public visibility before the prepared protection can be applied.

### Hosted branch decisions

Every non-main branch has a final public-surface disposition. Before deletion,
compare its unique commits against the closeout candidate and preserve any
wanted change on `main`; branch deletion is hygiene and is not a substitute for
purging sensitive history.

- Branch decision: `feat/andrzej_agent_sota_lab` — RETIRE_BEFORE_PUBLIC.
- Branch decision: `feat/andrzej_hlscreen_closeout` — RETIRE_BEFORE_PUBLIC.
- Branch decision: `feat/andrzej_hlscreen_foundation` — RETIRE_BEFORE_PUBLIC.
- Branch decision: `feat/andrzej_hlscreen_open_source_readiness` — RETIRE_BEFORE_PUBLIC.
- Branch decision: `feat/andrzej_hlscreen_oss_closeout` — RETIRE_BEFORE_PUBLIC.
- Branch decision: `feat/andrzej_oss_full_closeout` — MERGE_BEFORE_PUBLIC.
- Branch decision: `feat/andrzej_ratatui_live_tui` — RETIRE_BEFORE_PUBLIC.
- Branch decision: `feat/andrzej_tui_runtime_hardening` — RETIRE_BEFORE_PUBLIC.
- Branch decision: `dependabot/cargo/sha2-0.11.0` — RETIRE_BEFORE_PUBLIC.
- Branch decision: `dependabot/cargo/tokio-tungstenite-0.30.0` — RETIRE_BEFORE_PUBLIC.
- Branch decision: `dependabot/cargo/wasmtime-46.0.1` — RETIRE_BEFORE_PUBLIC.
- Branch decision: `dependabot/cargo/toml-1.1.3spec-1.1.0` — RETIRE_BEFORE_PUBLIC.
- Branch decision: `dependabot/github_actions/astral-sh/setup-uv-8.3.2` — RETIRE_BEFORE_PUBLIC.
- Branch decision: `dependabot/github_actions/actions/cache-6.1.0` — RETIRE_BEFORE_PUBLIC.

Private comparison evidence on 2026-07-15: six feature branches have zero
commits outside `origin/main`. `feat/andrzej_ratatui_live_tui` reports three
historical commits; its design commit is patch-equivalent to `61862a7` on
`main`, while its superseded plan/checkpoint predate the current
`specs/004-advanced-tui-workstation` plan and the July 13-15 merged workstation
hardening. No unique branch commit is selected for preservation.

### Open dependency PR decisions

Each dependency update still receives its own compatibility review and green
checks. The four runtime major-version proposals are deferred to separate
post-0.1 work because they expand release scope; the two action updates stay in
the closeout lane and must be rebased, reviewed, and pass hosted checks.

- PR decision: `#28` — CLOSE_BEFORE_PUBLIC.
- PR decision: `#42` — CLOSE_BEFORE_PUBLIC.
- PR decision: `#43` — CLOSE_BEFORE_PUBLIC.
- PR decision: `#44` — CLOSE_BEFORE_PUBLIC.
- PR decision: `#45` — UPDATE_AND_MERGE_BEFORE_PUBLIC.
- PR decision: `#46` — UPDATE_AND_MERGE_BEFORE_PUBLIC.

### Secret-history scan

- Tool contract: gitleaks 8.30.1, `--redact=100`, remote heads and all
  `refs/pull/*/head` fetched into temporary scan refs, `--all` history.
- Preliminary private scan: passed on 2026-07-15 with gitleaks 8.30.1 across 60
  temporary refs (14 remote heads and 46 pull-request heads); the script
  reported 348 reachable commits and no leaks. This predates the final
  candidate and must be rerun at that exact SHA.
- Final-candidate status: not yet run. Record only pass/fail, exact tool
  version, ref counts, and commit count. Never commit or paste a finding's
  matched content.

### Commit-author metadata

The preliminary 60-ref hosted inventory covers the same 348 commits as the
gitleaks scan, including 46 pull-request heads. It found three unique mailboxes:
one non-noreply mailbox appears in 337 author fields and 301 committer fields.
Addresses are intentionally omitted. This inventory must be rerun on the final
candidate. `.mailmap` cannot hide raw commit objects; publication therefore
requires the owner either to accept that exposure or explicitly authorize a
separately reviewed history rewrite. No rewrite is authorized by this audit.

### Owner confirmations

- [ ] Owner confirmation: GitHub billing/spending permits job execution.
- [ ] Owner confirmation: Packages inventory checked in GitHub UI.
- [ ] Owner confirmation: Private advisory drafts checked in GitHub UI.
- [ ] Owner confirmation: info@rsitech.ai monitoring checked for the documented
  security and conduct subjects.
- [ ] Owner confirmation: Git commit-author metadata exposure accepted, or a
  separately reviewed history rewrite authorized.
- [ ] Owner confirmation: Discussions Q&A and private vulnerability reporting
  enabled before public launch.
