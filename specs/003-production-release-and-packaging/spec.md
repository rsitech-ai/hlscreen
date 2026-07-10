# Feature Specification: Production Release And Packaging

**Feature Branch**: `003-production-release-and-packaging`

**Created**: 2026-07-08

**Status**: Partial; public release publication not proven

**Input**: Turn the current read-only local preview into professionally packaged
open-source software with reviewed binaries, checksums, source-build
instructions, and truthful public-repository readiness.

## User Story 1 - Install From A Reviewed Release (Priority: P1)

A new user can verify and run a reviewed `hlscreen` release artifact without
cloning the repository.

**Independent Test**: On a clean supported runner, download the artifact and
checksum from a completed `v*` release, verify them, run `hls --help`, run
`hls doctor`, and complete a bounded fixture smoke.

This story is not complete. Local archive/checksum smoke exists, but no reviewed
public `v*` release artifact has been published and verified.

## User Story 2 - Public Repository Readiness (Priority: P2)

An open-source contributor can evaluate license, support, security,
contribution, roadmap, release status, screenshots, and limitations without
private context.

**Independent Test**: Run the public-readiness scan and confirm the README,
roadmap, release docs, and checklist distinguish local proof from publication.

## Requirements

- **FR-001**: Docs MUST NOT claim published binary installation until a reviewed
  `v*` release page contains the expected artifacts and checksums.
- **FR-002**: Release artifacts MUST include checksums and an unpacked-binary
  verification workflow.
- **FR-003**: Release docs MUST separate source builds, local dry runs, and
  published artifacts.
- **FR-004**: CI MUST validate release configuration and local artifact smoke
  without publishing credentials.
- **FR-005**: Public-readiness docs MUST include license, contributing,
  security, support, code of conduct, roadmap, screenshots, and issue/PR
  templates.
- **FR-006**: No release workflow may require wallet credentials, exchange
  credentials, private endpoints, or order-capable functionality.

## Success Criteria

- **SC-001**: Every published artifact checksum verifies on a clean supported
  runner.
- **SC-002**: The unpacked binary passes `hls --help`, `hls doctor`, and a
  bounded fixture smoke.
- **SC-003**: The release CI gate passes without publication credentials.
- **SC-004**: Active docs contain no installability claim that lacks reviewed
  artifact evidence.

## Assumptions

- Source builds remain the only proven user installation route until the first
  reviewed public release succeeds.
- Tag creation and release publication are external actions that require an
  explicit ship decision after green CI and review.
