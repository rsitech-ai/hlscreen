# TUI Runtime Hardening Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `hls tui` a durable run-until-quit workstation with coordinated shutdown, deterministic terminal restoration, real PTY regression coverage, and diagnostics that identify stale binaries or incompatible terminal environments.

**Architecture:** Keep public market ingestion, screening, scoring, recording formats, and Ratatui layouts unchanged. Add explicit session lifetime and stop-reason types around the existing WebSocket loop, make terminal and recorder ownership RAII-safe, and exercise the exact terminal path through a fixture-backed interactive session in a pseudo-terminal.

**Tech Stack:** Rust 2024, Tokio, Ratatui 0.29, Crossterm 0.29, Clap 4.5, `portable-pty` as a test-only dependency, existing fixture and workspace crates.

## Global Constraints

- Public market data only; no wallet, private stream, signing, order, execution, or advice surface.
- `hls tui` defaults to top 10, one-second refresh, ANSI color, and an unbounded session stopped by `q`, `Esc`, Ctrl-C, SIGINT, or SIGTERM.
- `hls live` remains bounded to 60 seconds by default; explicit `--duration-secs N` remains deterministic for both commands.
- Zero duration means unbounded only for TUI mode; a non-interactive unbounded session must fail with an actionable error.
- Rendering pause affects display only; ingestion and recording continue.
- No direct diagnostic writes may occur while Ratatui owns the alternate-screen stderr surface.
- Terminal restoration must disable raw mode and mouse capture, show the cursor, and leave the alternate screen on normal exit, runtime error, and panic unwind.
- Recorder workers must be joined on every Rust unwind/return path and mark non-clean closeout when explicit clean completion was not reached.
- Fixture snapshots remain deterministic and unchanged when `--once` is used.

---

### Task 1: Explicit Session Lifetime And Stop Reasons

**Files:**
- Modify: `Cargo.toml`
- Modify: `crates/hls-cli/src/main.rs`
- Modify: `crates/hls-cli/src/commands/live.rs`

**Interfaces:**
- Produces: `LiveRunLifetime::{Bounded, Unbounded}` and `LiveStopReason::{DurationElapsed, OperatorQuit, Signal}`.
- Preserves: `LiveArgs` 60-second default and every explicit duration override.

- [ ] **Step 1: Write failing argument and key tests**

Add assertions equivalent to:

```rust
assert_eq!(parse_tui([]).duration_secs, 0);
assert_eq!(parse_tui(["--duration-secs", "15"]).duration_secs, 15);
assert_eq!(
    key_to_workstation_action(
        KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
        &WorkstationUiState::default(),
    ),
    Some(WorkstationAction::Quit),
);
```

Add pure lifetime tests proving zero maps to `Unbounded`, positive values map to `Bounded`, and only bounded lifetimes report expiry.

Run:

```bash
cargo test -p hls-cli tui_command_defaults_to_unbounded_operator_session -- --exact
cargo test -p hls-cli live_run_lifetime -- --nocapture
cargo test -p hls-cli live_tui_control_keys_map_to_screen_actions -- --exact
```

Expected: fail because the TUI still defaults to 60 seconds, zero is rejected, and Ctrl-C maps to chart focus.

- [ ] **Step 2: Implement minimal lifetime and signal model**

Add:

```rust
const DEFAULT_TUI_DURATION_SECS: u64 = 0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LiveRunLifetime {
    Bounded(tokio::time::Instant),
    Unbounded,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LiveStopReason {
    DurationElapsed,
    OperatorQuit,
    Signal,
}
```

Enable Tokio's `signal` feature. Convert `duration_secs` once at the runtime boundary. Make deadline waits pending for `Unbounded`, select on one shutdown future, and check Ctrl-C modifiers before ordinary character bindings.

- [ ] **Step 3: Verify Task 1**

Run the three focused commands from Step 1 plus:

```bash
cargo test -p hls-cli --test live_mock
```

Expected: all pass with no changes to fixture snapshot behavior.

### Task 2: Terminal And Recorder Lifecycle Ownership

**Files:**
- Modify: `crates/hls-cli/src/main.rs`
- Modify: `crates/hls-cli/src/commands/live.rs`

**Interfaces:**
- Produces: idempotent active-TUI restoration hook and `LiveRecorder::shutdown(clean_shutdown)` used by explicit finish and `Drop`.
- Preserves: existing raw/normalized data contracts and clean-shutdown metadata.

- [ ] **Step 1: Write failing lifecycle tests**

Add tests that:

```rust
let recorder = fixture_recorder(&temp_dir);
drop(recorder);
assert_eq!(load_run(&temp_dir).clean_shutdown, false);
```

Also assert that Ctrl-C/SIGTERM and operator quit have distinct stop reasons, reconnect diagnostics are retained in state instead of written directly in TUI mode, malformed preferences are loaded before terminal activation, and renderer initialization performs one full clear only.

Run:

```bash
cargo test -p hls-cli live_recorder_drop_joins_worker_as_unclean -- --exact
cargo test -p hls-cli live_tui_diagnostics_do_not_bypass_frame_sink -- --exact
```

Expected: fail because `LiveRecorder` has no `Drop` closeout and reconnects write directly to stderr.

- [ ] **Step 2: Implement lifecycle hardening**

Change recorder ownership to optional sender/handle fields so both paths share one closeout:

```rust
impl LiveRecorder {
    fn shutdown(&mut self, clean_shutdown: bool) -> HlsResult<RecordSummary>;
    fn finish(mut self, clean_shutdown: bool) -> HlsResult<RecordSummary> {
        self.shutdown(clean_shutdown)
    }
}

impl Drop for LiveRecorder {
    fn drop(&mut self) {
        let _ = self.shutdown(false);
    }
}
```

Load preferences and complete fallible preflight before entering alternate-screen mode. Install a panic hook guarded by an atomic active-session flag so restoration happens before panic output. Buffer recorder/reconnect warnings until after renderer and terminal guard drop. Remove the redundant backend flush after `Terminal::draw`, which already flushes and swaps buffers.

- [ ] **Step 3: Verify Task 2**

Run:

```bash
cargo test -p hls-cli live_recorder -- --nocapture
cargo test -p hls-cli live_progress_reuses_supplied_tui_frame_sink -- --exact
cargo test -p hls-cli --test live_mock
```

Expected: all pass; recorder threads are joined and no TUI-active code writes plain diagnostics.

### Task 3: Deterministic Interactive Fixture And PTY Proof

**Files:**
- Modify: `Cargo.toml`
- Modify: `crates/hls-cli/Cargo.toml`
- Modify: `crates/hls-cli/src/commands/live.rs`
- Create: `crates/hls-cli/tests/pty_tui.rs`

**Interfaces:**
- Produces: fixture-backed interactive `hls tui --fixture-file ...` when `--once` is absent.
- Uses: the production `LiveTuiGuard`, `LiveTuiRenderer`, keyboard mapping, resize handling, and preference closeout.

- [ ] **Step 1: Write the failing PTY test**

Add `portable-pty = "0.9"` as a workspace dev dependency and create a Unix PTY test that:

```text
1. Starts the exact hls binary at 120x40 with TERM=xterm-256color and COLORTERM=truecolor.
2. Runs fixture-backed `hls tui` without `--once`.
3. Reads until WATCHLIST and a truecolor SGR sequence are present.
4. Resizes the PTY and waits for a redraw marker.
5. Sends q and waits for successful process exit.
6. Asserts alternate-screen enter/leave, mouse enable/disable, cursor restore, and exact stty state restoration.
7. Asserts no plain reconnect/preference diagnostic appears between alternate-screen entry and exit.
```

Run:

```bash
cargo test -p hls-cli --test pty_tui -- --nocapture
```

Expected: fail because fixture mode currently requires `--once` and never enters the production terminal session.

- [ ] **Step 2: Add the fixture-backed interactive session**

Keep `--once` unchanged. When a fixture is supplied without `--once`, require TUI mode and either a TTY or explicit positive duration, apply fixture events once, then run the normal render/input/lifetime loop. Use event-driven PTY reads with deadlines; do not synchronize with arbitrary sleeps.

- [ ] **Step 3: Add renderer byte regression**

Generalize `LiveTuiRenderer<W: Write>` with a production `io::Stderr` constructor and an injected writer constructor for tests. Render repeated frames and assert initialization is the only non-resize full-screen clear.

- [ ] **Step 4: Verify Task 3**

Run:

```bash
cargo test -p hls-cli --test pty_tui -- --nocapture
cargo test -p hls-cli --test live_mock
cargo test -p hls-tui --test ratatui_cockpit --test workstation_interaction
```

Expected: all pass on macOS and Linux.

### Task 4: Terminal Provenance And Operator Diagnostics

**Files:**
- Modify: `crates/hls-cli/src/main.rs`
- Modify: `crates/hls-cli/src/commands/doctor.rs`
- Modify: `crates/hls-cli/tests/basic_commands.rs`
- Modify: `README.md`
- Modify: `.github/workflows/ci.yml`

**Interfaces:**
- Produces: `hls --version`, `hls doctor --terminal`, canonical `cargo run -p hls-cli -- tui` guidance, and macOS/Linux PTY CI.

- [ ] **Step 1: Write failing diagnostics tests**

Assert that `hls --version` contains `ratatui-workstation`, and `hls doctor --terminal` reports:

```text
binary=
version=
renderer=ratatui-workstation
cwd=
stdin_tty=
stderr_tty=
TERM=
COLORTERM=
TMUX=
NO_COLOR=
force_color=
auto_color=
```

Add equivalent JSON-field assertions.

- [ ] **Step 2: Implement diagnostics and docs**

Add Clap version identity and a terminal-only doctor path that performs no data-directory writes. Update README to make `cargo run -p hls-cli -- tui` the development source of truth, explain stderr ownership and stale binaries/worktrees, document `0 = until quit`, and provide exact `doctor` commands.

Add a focused CI matrix for `cargo test -p hls-cli --test pty_tui` on `ubuntu-latest` and `macos-latest`; keep the existing workspace job unchanged.

- [ ] **Step 3: Verify Task 4**

Run:

```bash
cargo test -p hls-cli --test basic_commands
cargo run -p hls-cli -- --version
cargo run -p hls-cli -- doctor --terminal
cargo run -p hls-cli -- tui --help
```

Expected: provenance and terminal decisions are explicit, and help documents the unbounded default.

### Task 5: Full Verification, Live Smoke, And Ship Gate

**Files:**
- Modify: `reflections/20260709T183818Z-tui-runtime-hardening.md`

- [ ] **Step 1: Run static and workspace verification**

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --release --workspace --all-features
scripts/check-release-packaging.sh
python3 scripts/generate-screenshots.py --check
git diff --check
```

- [ ] **Step 2: Run realistic smokes**

```bash
cargo run -p hls-cli -- tui --duration-secs 5
cargo run -p hls-cli -- tui --symbols HYPE/USDC --duration-secs 5 --refresh-secs 1
cargo run -p hls-cli -- doctor --terminal
```

Require top-10 live selection, public subscriptions, zero render-induced gaps in a healthy run, clean terminal restoration, and no panic/error/warning text inside the alternate-screen transcript.

- [ ] **Step 3: Review and publish**

Inspect the complete diff against `origin/main`, perform an independent correctness/security/performance review, commit, push, open a PR, resolve review and CI findings, merge only when all required checks pass, then verify post-merge `main` CI and install the merged CLI from the clean worktree so `hls tui` invokes the validated binary.
