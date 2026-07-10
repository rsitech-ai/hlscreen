# Tasks: Production Release And Packaging

**Input**: [spec.md](spec.md), [plan.md](plan.md), and [quickstart.md](quickstart.md)

## Local Proof

- [x] T001 Pin and validate the cargo-dist release configuration.
- [x] T002 Add a local archive/checksum/unpack smoke.
- [x] T003 Add a public-readiness documentation scan.
- [x] T004 Keep release validation independent of publication credentials.
- [x] T005 Document the read-only safety boundary and local-proof limitation.

## Public Release Proof

- [ ] T006 Review the complete release PR and generated artifact plan.
- [ ] T007 Create a reviewed `v*` tag only after main and CI are green.
- [ ] T008 Verify every published artifact and checksum on clean supported runners.
- [ ] T009 Verify the completed release page, installers, exact commit, and workflow run.
- [ ] T010 Record the public release evidence and only then publish binary install instructions.
