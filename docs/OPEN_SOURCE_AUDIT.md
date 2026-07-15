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
