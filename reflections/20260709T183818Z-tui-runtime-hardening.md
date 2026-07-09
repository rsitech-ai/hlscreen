# TUI Runtime Hardening Reflection

## Task
- **ID/Title:** Canonical TUI runtime hardening
- **Date:** 2026-07-09
- **Scope:** multi-file

## Plan and Risks
- **Planned approach:** Preserve the existing read-only market-data and Ratatui presentation layers, make `hls tui` a real run-until-quit session, route shutdown through one explicit runtime outcome, and add process-level pseudo-terminal verification for color, resize, quit, and terminal restoration.
- **Top failure hypotheses:** (1) the 60-second `TuiArgs` default and fixed `Instant` deadline make the canonical command a bounded smoke rather than an operator session; (2) direct stderr diagnostics during alternate-screen ownership corrupt Ratatui's diff model; (3) unit/TestBackend coverage cannot detect incorrect terminal escape ordering, stale binaries, or failed restoration.
- **Success criteria:** `hls tui` runs until `q` or Ctrl-C by default; explicit `--duration-secs N` remains bounded; raw mode, mouse capture, cursor visibility, and alternate screen are restored on success and errors; fixture, PTY, resize, color, workspace, packaging, and live smoke checks pass; read-only safety remains unchanged.

## Candidate Attempts
| Candidate | Summary | Outcome | Signals | Why selected / rejected |
|---|---|---|---|---|
| A | Represent session bounds explicitly (`Option<Instant>`), add a shutdown outcome, and keep all active-session diagnostics inside the TUI model. | Pending | Matches the existing event loop and supports bounded tests plus unbounded operator sessions. | Preferred because duration semantics remain explicit and testable. |
| B | Use a very large duration as a proxy for infinity and rely only on `q`. | Rejected | Risks deadline overflow, makes Ctrl-C restoration unproven, and obscures intent. | Rejected as mystery behavior. |
| C | Replace the runtime wholesale with Ratatui's global `run/init/restore` helpers. | Rejected for this slice | The current app deliberately renders to stderr while stdout remains machine-readable and owns recording closeout ordering. | Too invasive; retain the existing RAII guard and harden it directly. |

## Reflection
- **Failure modes observed:** Canonical `hls tui` is time-bounded; `--duration-secs 0` is rejected; no process-level PTY test proves terminal restoration; reconnect diagnostics bypass the Ratatui frame while the alternate screen is active.
- **Root cause:** Pending implementation and verification.
- **Fix that resolved it:** Pending.
- **What improved score/quality:** Pending.
- **Useful command-level evidence:** Baseline `cargo build --workspace --all-features` and `cargo test --workspace --all-features` pass on `main` commit `29596a4`.
- **Branch comparison insight (if multiple attempts):** Work is isolated on `feat/andrzej_tui_runtime_hardening`; the user's dirty primary worktree remains untouched.

## Reusable Lesson
- **Pattern that worked:** Model terminal ownership and session bounds as explicit lifecycle state, then test the actual process through a PTY.
- **Pattern to avoid:** Treating snapshot output as proof that real terminal setup, diffing, and restoration work.
- **Where to apply next:** Other long-running Rust CLIs using alternate screens, raw mode, or async signal handling.

## Decision
- **Final chosen approach:** Candidate A, subject to red-green regression evidence.
- **Commit/rollback decision:** Pending.
- **Next step / follow-up:** Add the implementation plan, then begin with failing CLI/runtime tests.
