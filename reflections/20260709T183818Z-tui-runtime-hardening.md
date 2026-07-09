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
| A | Represent session bounds explicitly, add a shutdown outcome, and keep all active-session diagnostics inside the TUI model. | Accepted | Unbounded operator sessions, bounded automation, cancellation, and deferred diagnostics all pass process-level tests. | Preferred because duration and terminal ownership remain explicit and testable. |
| B | Use a very large duration as a proxy for infinity and rely only on `q`. | Rejected | Risks deadline overflow, makes Ctrl-C restoration unproven, and obscures intent. | Rejected as mystery behavior. |
| C | Replace the runtime wholesale with Ratatui's global `run/init/restore` helpers. | Rejected for this slice | The current app deliberately renders to stderr while stdout remains machine-readable and owns recording closeout ordering. | Too invasive; retain the existing RAII guard and harden it directly. |

## Reflection
- **Failure modes observed:** The canonical command was time-bounded; shutdown futures installed handlers too late; recorder drop could detach its worker; terminal writes and panic restoration could interleave; diagnostics bypassed the alternate-screen frame; automatic color ignored the destination TTY state; a hosted macOS PTY could miss the resize event during first-frame handoff; the first PTY cleanup used a direct-child `SIGHUP` followed by unbounded joins; and the globally resolved binary was stale.
- **Root cause:** Session duration, cancellation, recorder ownership, terminal ownership, and operator binary identity were implicit across separate code paths. Unit snapshots could not prove process or terminal state.
- **Fix that resolved it:** Added explicit bounded/unbounded lifetimes and stop reasons, eager signal listeners, cancellable network writes, joined recorder teardown, serialized terminal operations, deferred diagnostics, persistent Ratatui rendering, event-independent viewport polling, process-group PTY cleanup, startup-signal/resize/color/quit PTY coverage, destination-aware stdout/stderr auto-color policy, renderer-tagged version output, and read-only terminal diagnostics. Upgraded Ratatui to 0.30.2 to remove the RustSec `lru` unsoundness warning and the unmaintained `paste` dependency.
- **What improved score/quality:** Regular frames now rely on Ratatui buffer diffs with no full clears; only startup and documented horizontal-shrink resize paths clear. Both macOS and Linux PTYs prove balanced alternate screen, mouse capture, cursor restoration, stable termios, early `SIGINT`/`SIGTERM`, fixture identity, and bounded failure cleanup.
- **Useful command-level evidence:** Strict workspace Clippy and tests pass, including 119 cockpit and 11 interaction cases; debug/release builds, packaging checks, screenshot regeneration, and `cargo audit --deny warnings` pass. A three-second truecolor public top-10 run received 122 WebSocket messages and 372 market events with zero reconnects or gaps; a deliberate unreachable endpoint failed closed after bounded retries.
- **Branch comparison insight (if multiple attempts):** Work is isolated on `feat/andrzej_tui_runtime_hardening`; the user's dirty primary worktree remains untouched.

## Reusable Lesson
- **Pattern that worked:** Model terminal ownership and session bounds as explicit lifecycle state, then test the actual process through a PTY.
- **Pattern to avoid:** Treating snapshot output as proof that real terminal setup, diffing, and restoration work.
- **Where to apply next:** Other long-running Rust CLIs using alternate screens, raw mode, or async signal handling.

## Decision
- **Final chosen approach:** Candidate A with one persistent Ratatui terminal, serialized lifecycle operations, and process-level PTY evidence.
- **Commit/rollback decision:** Keep the branch after independent review and CI; rollback remains a normal revert because no data format or capital-touching behavior changed.
- **Next step / follow-up:** Merge only after GitHub CI passes, then install the merged CLI and verify `hls --version`, `hls doctor --terminal`, and a bounded public live run from the resolved binary.
