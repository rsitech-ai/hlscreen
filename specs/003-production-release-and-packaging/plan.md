# Implementation Plan: Production Release And Packaging

**Branch**: `003-production-release-and-packaging` | **Date**: 2026-07-10 | **Spec**: [spec.md](spec.md)

## Summary

Keep source-build and local artifact checks reproducible, maintain truthful
open-source documentation, and prove the first public binary release through a
reviewed `v*` workflow before advertising binary installation.

## Technical Context

**Language/Version**: Rust stable, edition 2024, pinned by `rust-toolchain.toml`.

**Primary Dependencies**: Existing cargo-dist draft, GitHub Actions, shell
validation scripts, and public documentation.

**Testing**: Release configuration contract tests, local archive/checksum smoke,
public-readiness scan, source build, and final published-artifact verification.

**Target Platform**: Supported macOS, Linux, and Windows release targets already
declared by the release configuration.

## Safety And Truth Boundaries

- No secrets in Git or local smoke commands.
- No publication claim before the reviewed release page and checksums exist.
- No wallet, private-data, order, signing, or exchange-action behavior.
- Local archive/checksum smoke is evidence for packaging mechanics only.

## Phases

1. Keep source builds, local archive/checksum smoke, and public-readiness checks green.
2. Review release configuration and generated plan on a release PR.
3. Create a `v*` tag only after main and release checks are green.
4. Verify every published artifact, checksum, installer, and release-page statement.
5. Record the exact release URL, commit, workflow run, and clean-machine smoke.
