# TODO

- [x] Confirm owning repo(s) and affected files
  DoD: File/module scope is listed in `PLAN.md`.

- [x] Initialize local Spec Kit scaffold
  DoD: `hlscreen/.specify/` exists and scripts/templates are available.

- [x] Generate feature specification
  DoD: `spec.md` and its quality checklist exist with no unresolved clarification markers.

- [x] Generate implementation plan and design artifacts
  DoD: `plan.md`, `research.md`, `data-model.md`, contracts, and `quickstart.md` exist.

- [x] Generate dependency-ordered tasks
  DoD: `tasks.md` uses the required task checklist format and maps work by user story.

- [x] Run relevant validation commands
  DoD: Markdown and diff checks pass; task format and Spec Kit active feature pointer are verified.

- [x] Update `MEMORY.md`
  DoD: Durable local project convention is recorded without secrets.

- [x] Final close-out
  DoD: `PLAN.md` final notes complete, `TODO.md` fully checked or explicitly deferred, reflection is closed.

## 2026-07-07 Implementation Slice

- [x] Create Rust workspace and crate skeletons
  DoD: `cargo metadata` can load all planned crates.

- [x] Add config/docs/fixture skeleton
  DoD: default config, docs stubs, and Hyperliquid REST fixtures exist.

- [x] Implement and test `hls-core`
  DoD: config parsing, symbol mapping, and time helpers have passing tests.

- [x] Implement and test fixture-backed REST metadata parsing
  DoD: `hls-hyperliquid` parses spot metadata plus asset contexts from fixtures.

- [x] Implement and test CLI `init`, `doctor`, and `symbols`
  DoD: CLI tests pass and `symbols` can run against fixtures without live credentials.

- [x] Run validation and audit
  DoD: fmt, clippy, tests, diff check, and read-only boundary audit are recorded.

- [x] Push coherent validated slice if checks pass
  DoD: standalone `hlscreen` Git repo is committed and pushed to `s1korrrr/hlscreen.git` without unrelated parent changes.

- [x] Close implementation notes
  DoD: `PLAN.md`, `TODO.md`, `MEMORY.md`, and Spec Kit `tasks.md` reflect actual state.
