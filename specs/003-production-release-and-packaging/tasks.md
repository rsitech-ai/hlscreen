# Tasks: Production Release And Packaging

**Input**: [spec.md](spec.md), [plan.md](plan.md), and [quickstart.md](quickstart.md)

## Local Proof

- [x] T001 Pin and validate the cargo-dist release configuration.
- [x] T002 Add a local archive/checksum/unpack smoke.
- [x] T003 Add a public-readiness documentation scan.
- [x] T004 Keep release validation independent of publication credentials.
- [x] T005 Document the read-only safety boundary and local-proof limitation.
- [x] T011 Pin release actions and runners, disable credential persistence, and
  scope write permissions to tag publication.
- [x] T012 Build PR candidate artifacts with source, checksums, CycloneDX SBOM,
  and auditable binaries.
- [x] T013 Add a deterministic hardening/check step for cargo-dist 0.32.0
  generated workflow defects.
- [x] T014 Reject tracked developer paths and common credential formats in the
  public-readiness gate.
- [x] T017 Enforce concurrency, Dependabot cooldowns, release cache/container
  safety, shell-expression isolation, and a zero-finding zizmor gate.
- [x] T018 Enforce all-target dependency license and source-registry policy.

## Public Release Proof

- [ ] T006 Review the complete release PR and generated artifact plan.
- [ ] T007 Create a reviewed `v*` tag only after main and CI are green.
- [ ] T008 Verify every published artifact and checksum on clean supported runners.
- [ ] T009 Verify the completed release page, installers, exact commit, and workflow run.
- [ ] T010 Record the public release evidence and only then publish binary install instructions.
- [ ] T015 Prove the pull-request candidate artifacts on every supported clean runner.
- [ ] T016 Verify repository eligibility for GitHub artifact attestations before tagging.
