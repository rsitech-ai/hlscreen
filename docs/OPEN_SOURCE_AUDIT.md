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

- [x] Canonical local release gate passes on the candidate commit.
- [x] Redacted full-history secret scan covers remote branches and pull-request
  head refs.
- [ ] Private hosted CI and release-plan artifacts pass on the exact candidate
  commit.
- [ ] GitHub billing/spending state permits jobs to execute. The owner approved
  a quota-only exception for private `main` integration on 2026-07-16; this
  does not satisfy the public-visibility or release gate.
- [ ] Repository visibility, ruleset/protection, and security features are
  verified after publication.
- [ ] `v0.1.0` artifacts, checksums, SBOMs, and attestations are verified.

## Hosted surface snapshot

Snapshot date: 2026-07-16

- Repository: `s1korrrr/hlscreen`, private, default branch `main`.
- Audited private base: `1d5e9cd53c6982316b9d4698b4f605ddc753980c`.
- Candidate identity contract: the exact 40-character SHA is supplied outside
  this tracked file to the private PR, hosted workflows, and
  `scripts/check-public-surface.sh`. Embedding the file's own candidate SHA here
  would invalidate it whenever this audit changed.
- Exact-candidate CI run `29481365077` and Release run `29481365057` were
  refused before executing job steps because of GitHub billing/spending state.
  They were evidence of an account-level refusal, not valid hosted proof.
- Before cleanup, the hosted inventory contained nine branches, one open human
  PR (`#47`), no open Dependabot PRs, and no issues, tags, releases, Pages site,
  deployments, environments, deploy keys, webhooks, or configured
  repository/Dependabot/Codespaces secret or variable names.
- Actions cleanup completed on 2026-07-16: all 90 artifacts and all 410
  historical workflow runs were deleted through the authenticated GitHub API;
  every deletion succeeded. The current hosted inventory has zero workflow
  runs and zero artifacts. Any run created by a later private push must be
  removed if it is refused before steps, or retained only after successful
  execution and an in-memory privacy scan.
- Actions-history decision: DELETE_NON_CANDIDATE_RUNS_BEFORE_PUBLIC — completed
  for the pre-merge inventory.
- The candidate, audit, and `main` pushes created additional zero-step workflow
  runs with the same billing annotation; all were deleted after merge.
- Actions is restricted to the six SHA-pinned actions referenced by the tracked
  workflows: `actions/checkout`, `actions/cache`, `actions/upload-artifact`,
  `actions/download-artifact`, `actions/attest`, and `astral-sh/setup-uv`.
  GitHub-owned and verified actions are not allowed as broad categories, and
  required SHA pinning is enabled. Default `GITHUB_TOKEN` permissions are
  read-only and cannot approve pull-request reviews.
- Hosted branch cleanup completed on 2026-07-16: the seven reviewed stale
  feature branches and the merged closeout candidate branch were deleted
  remotely. The reviewed post-merge audit branch was also deleted after PR
  `#48` merged. The hosted inventory returned to `main` only; all local branches
  and worktrees were preserved.
- Repository metadata is set for publication discovery: a read-only product
  description and the `cli`, `hyperliquid`, `market-data`, `ratatui`, `rust`,
  `screener`, `terminal`, and `tui` topics. Discussions is enabled and its Q&A
  category is answerable. Automatic head-branch deletion after merge is
  enabled.
- Security inventory: vulnerability alerts and Dependabot automated security
  updates are enabled, with zero current Dependabot alerts. The authenticated
  repository security overview confirms Dependabot alerts enabled and code
  scanning disabled because Advanced Security is available only to
  organizations on this private-plan surface. Secret scanning and push
  protection return GitHub's `422` unavailable response. Private vulnerability
  reporting has no repository API or UI surface while the repository remains
  private on the current plan. `main` is not protected; the ruleset and branch
  protection APIs require a paid private plan or public visibility.
- The authenticated owner Packages view filtered to `hlscreen` reports zero
  packages. The private-advisory URL returns GitHub's authenticated 404, and
  the repository security navigation exposes only the security policy under
  Reporting; there is no private-advisory draft surface to inspect until
  private vulnerability reporting becomes available.

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
- Branch decision: `feat/andrzej_oss_postmerge_audit` — MERGE_BEFORE_PUBLIC.
- Branch decision: `feat/andrzej_oss_private_settings` — MERGE_BEFORE_PUBLIC.
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

### Dependency PR decisions

The four runtime major-version proposals are deferred to separate post-0.1
work because they expand release scope. The two action updates were reviewed
and integrated into the closeout candidate with exact SHA allowlist parity.
All six bot PRs are closed and their remote branches are deleted.

- PR decision: `#28` — CLOSE_BEFORE_PUBLIC.
- PR decision: `#42` — CLOSE_BEFORE_PUBLIC.
- PR decision: `#43` — CLOSE_BEFORE_PUBLIC.
- PR decision: `#44` — CLOSE_BEFORE_PUBLIC.
- PR decision: `#45` — INTEGRATED_IN_CLOSEOUT_CLOSE_BEFORE_PUBLIC.
- PR decision: `#46` — INTEGRATED_IN_CLOSEOUT_CLOSE_BEFORE_PUBLIC.

### Secret-history scan

- Tool contract: gitleaks 8.30.1, `--redact=100`, remote heads and all
  `refs/pull/*/head` fetched into temporary scan refs, `--all` history.
- Pre-integration candidate scan passed on 2026-07-16 with gitleaks 8.30.1
  across 62 temporary refs (15 remote heads and 47 pull-request heads). The script
  reported 353 reachable commits and no leaks. Record only pass/fail, exact
  tool version, ref counts, and commit count; never commit or paste a finding's
  matched content.
- Post-merge verification at `bbaca40` passed with gitleaks 8.30.1 across 48
  hosted refs (one remote head and 47 pull-request heads), 354 reachable
  commits, and no leaks.
- Final private-main verification at `1d5e9cd` passed with gitleaks 8.30.1
  across 49 hosted refs (one remote head and 48 pull-request heads), 358
  reachable commits, and no leaks.

### Commit-author metadata

The post-merge `bbaca40` inventory covers 354 reachable commits, including 47
pull-request heads. It found three unique mailboxes: one non-noreply mailbox
appears in 343 author fields and 307 committer fields. Addresses are
intentionally omitted. `.mailmap` cannot hide raw commit objects. The owner
accepted this raw metadata exposure on 2026-07-16; no history rewrite is
authorized by this audit.

### Historical content privacy metadata

The post-merge `bbaca40` metadata-only history pass streamed patch and commit
message content without writing or printing matched values. Across 354 commits,
it counted 672 developer-home path occurrences in 22 commits, five private
temporary-worktree occurrences in five commits, and 15 non-public email
occurrences in 13 commits. The summarizer counts matched commit-message text and
added or removed patch lines while excluding diff headers and unchanged
context. These are occurrence counts, not unique values or confirmed secrets;
gitleaks separately reported no leaks. The owner accepted these raw historical
content categories on 2026-07-16. No rewrite is authorized by this audit.

### Owner confirmations

- [x] Owner decision: proceed with reviewed private `main` integration despite
  the GitHub billing refusal; keep hosted proof blocking for public visibility
  and release publication.
- [x] Owner confirmation: Packages inventory checked in GitHub UI.
  The repository-filtered owner view reports zero packages.
- [x] Owner confirmation: Private advisory drafts checked through the
  authenticated private-advisory endpoint and repository Security navigation.
  GitHub exposes no advisory draft surface while private vulnerability
  reporting is unavailable.
- [x] Owner confirmation: info@rsitech.ai monitoring checked for the documented
  security and conduct subjects and confirmed active; it is a monitored company
  address.
- [x] Owner confirmation: Git commit-author metadata exposure accepted.
- [x] Owner confirmation: Historical developer-path and non-public email
  content exposure accepted.
- [x] Owner confirmation: Discussions and its answerable Q&A category are enabled.
- [ ] Owner confirmation: private vulnerability reporting enabled before public launch.
  GitHub does not expose this feature for the repository while it remains
  private on the current plan.

The owner also directed that the repository remain private until the complete
production and publication gates are satisfied. The private merge exception is
not permission to change visibility, tag, publish a release, or claim hosted
production proof.
