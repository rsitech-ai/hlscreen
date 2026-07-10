#![cfg(unix)]

use std::{
    io::{self, Read, Write},
    path::Path,
    process::Command,
    sync::mpsc::{self, Receiver, RecvTimeoutError},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use anyhow::{Context, Result, bail, ensure};
use portable_pty::{CommandBuilder, ExitStatus, MasterPty, PtySize, native_pty_system};

const INITIAL_TIMEOUT: Duration = Duration::from_secs(15);
const RESIZE_TIMEOUT: Duration = Duration::from_secs(10);
const EXIT_TIMEOUT: Duration = Duration::from_secs(10);
const CLEANUP_GRACE_TIMEOUT: Duration = Duration::from_secs(1);
const CLEANUP_FORCE_TIMEOUT: Duration = Duration::from_secs(3);
const CLEANUP_WATCHDOG_TIMEOUT: Duration = Duration::from_secs(8);
const HORIZONTAL_SHRINK_CLEAR_COUNT: usize = 2;

const ALT_SCREEN_ENTER: &[u8] = b"\x1b[?1049h";
const ALT_SCREEN_LEAVE: &[u8] = b"\x1b[?1049l";
const MOUSE_ENABLE: &[u8] = b"\x1b[?1000h\x1b[?1002h\x1b[?1003h\x1b[?1015h\x1b[?1006h";
const MOUSE_DISABLE: &[u8] = b"\x1b[?1006l\x1b[?1015l\x1b[?1003l\x1b[?1002l\x1b[?1000l";
const CURSOR_HIDE: &[u8] = b"\x1b[?25l";
const CURSOR_SHOW: &[u8] = b"\x1b[?25h";
const FULL_SCREEN_CLEAR: &[u8] = b"\x1b[2J";
const TRUECOLOR_FOREGROUND: &[u8] = b"\x1b[38;2;";
const TRUECOLOR_BACKGROUND: &[u8] = b";48;2;";
const EXIT_MARKER: &[u8] = b"__HLS_EXIT_STATUS__=";

const WRAPPER: &str = r#"
before=$(/bin/stty -g) || exit 97
printf '__HLS_STTY_BEFORE__=%s\n' "$before"
if [ -n "$4" ]; then
    "$1" tui --fixture-file "$2" --data-dir "$3" --color always --duration-secs "$4"
else
    "$1" tui --fixture-file "$2" --data-dir "$3" --color always
fi
status=$?
after_raw=$(/bin/stty -g) || exit 98
printf '__HLS_STTY_AFTER_RAW__=%s\n' "$after_raw"
printf '__HLS_STTY_AFTER__=%s\n' "$after_raw"
printf '__HLS_EXIT_STATUS__=%s\n' "$status"
exit "$status"
"#;

enum ReaderEvent {
    Bytes(Vec<u8>),
    Closed(Option<String>),
}

struct PtySession {
    master: Option<Box<dyn MasterPty + Send>>,
    writer: Option<Box<dyn Write + Send>>,
    child_process_id: u32,
    process_group_id: i32,
    reader_rx: Receiver<ReaderEvent>,
    status_rx: Receiver<io::Result<ExitStatus>>,
    reader_thread: Option<JoinHandle<()>>,
    waiter_thread: Option<JoinHandle<()>>,
    transcript: Vec<u8>,
    status: Option<ExitStatus>,
    reader_closed: bool,
    finished: bool,
}

impl PtySession {
    fn spawn(
        binary: &Path,
        fixture: &Path,
        data_dir: &Path,
        duration_secs: Option<u64>,
    ) -> Result<Self> {
        Self::spawn_with_size(
            binary,
            fixture,
            data_dir,
            duration_secs,
            PtySize {
                rows: 40,
                cols: 120,
                pixel_width: 0,
                pixel_height: 0,
            },
        )
    }

    fn spawn_with_size(
        binary: &Path,
        fixture: &Path,
        data_dir: &Path,
        duration_secs: Option<u64>,
        size: PtySize,
    ) -> Result<Self> {
        Self::spawn_with_size_and_no_color_env(
            binary,
            fixture,
            data_dir,
            duration_secs,
            size,
            false,
        )
    }

    fn spawn_with_no_color_env(
        binary: &Path,
        fixture: &Path,
        data_dir: &Path,
        duration_secs: Option<u64>,
    ) -> Result<Self> {
        Self::spawn_with_size_and_no_color_env(
            binary,
            fixture,
            data_dir,
            duration_secs,
            PtySize {
                rows: 40,
                cols: 120,
                pixel_width: 0,
                pixel_height: 0,
            },
            true,
        )
    }

    fn spawn_with_size_and_no_color_env(
        binary: &Path,
        fixture: &Path,
        data_dir: &Path,
        duration_secs: Option<u64>,
        size: PtySize,
        no_color_env: bool,
    ) -> Result<Self> {
        let mut command = CommandBuilder::new("/bin/sh");
        command.arg("-c");
        command.arg(WRAPPER);
        command.arg("hls-pty-wrapper");
        command.arg(binary);
        command.arg(fixture);
        command.arg(data_dir);
        command.arg(duration_secs.map_or_else(String::new, |value| value.to_string()));
        command.env("TERM", "xterm-256color");
        command.env("COLORTERM", "truecolor");
        command.env("PATH", "/usr/bin:/bin");
        if no_color_env {
            command.env("NO_COLOR", "1");
        } else {
            command.env_remove("NO_COLOR");
        }
        command.env_remove("FORCE_COLOR");
        command.env_remove("CLICOLOR_FORCE");
        command.env_remove("HLS_FORCE_COLOR");
        Self::spawn_command(command, size)
    }

    fn spawn_command(command: CommandBuilder, size: PtySize) -> Result<Self> {
        let pair = native_pty_system().openpty(size)?;
        let mut reader = pair.master.try_clone_reader()?;
        let writer = pair.master.take_writer()?;
        let mut child = pair.slave.spawn_command(command)?;
        let child_process_id = child
            .process_id()
            .context("PTY child process id unavailable")?;
        let process_group_id = pair
            .master
            .process_group_leader()
            .unwrap_or(i32::try_from(child_process_id).context("PTY child process id overflow")?);
        ensure!(
            process_group_id > 1,
            "refusing unsafe PTY process group id {process_group_id}"
        );
        drop(pair.slave);

        let (reader_tx, reader_rx) = mpsc::channel();
        let reader_thread = thread::spawn(move || {
            let mut buffer = [0_u8; 8192];
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => {
                        let _ = reader_tx.send(ReaderEvent::Closed(None));
                        break;
                    }
                    Ok(count) => {
                        if reader_tx
                            .send(ReaderEvent::Bytes(buffer[..count].to_vec()))
                            .is_err()
                        {
                            break;
                        }
                    }
                    Err(err) => {
                        let _ = reader_tx.send(ReaderEvent::Closed(Some(err.to_string())));
                        break;
                    }
                }
            }
        });

        let (status_tx, status_rx) = mpsc::channel();
        let waiter_thread = thread::spawn(move || {
            let _ = status_tx.send(child.wait());
        });

        Ok(Self {
            master: Some(pair.master),
            writer: Some(writer),
            child_process_id,
            process_group_id,
            reader_rx,
            status_rx,
            reader_thread: Some(reader_thread),
            waiter_thread: Some(waiter_thread),
            transcript: Vec::new(),
            status: None,
            reader_closed: false,
            finished: false,
        })
    }

    fn stubborn_child() -> Result<Self> {
        let mut command = CommandBuilder::new("/bin/sh");
        command.arg("-c");
        command.arg(
            "trap 'exit 0' HUP TERM; /bin/sh -c \"trap '' HUP TERM; while :; do /bin/sleep 60; done\" & printf '__HLS_STUBBORN_READY__\\n'; wait",
        );
        command.env("PATH", "/usr/bin:/bin");
        Self::spawn_command(
            command,
            PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            },
        )
    }

    fn signal_direct_child(&self, signal: &str) -> Result<()> {
        let child = direct_child_process(self.child_process_id)?;
        send_signal(signal, child.to_string())
            .with_context(|| format!("send {signal} to PTY command child {child}"))
    }

    fn process_group_id(&self) -> i32 {
        self.process_group_id
    }

    fn wait_for(
        &mut self,
        label: &str,
        timeout: Duration,
        predicate: impl Fn(&[u8]) -> bool,
    ) -> Result<usize> {
        let deadline = Instant::now() + timeout;
        loop {
            if predicate(&self.transcript) {
                return Ok(self.transcript.len());
            }
            self.receive_reader_event(deadline, label)?;
        }
    }

    fn receive_reader_event(&mut self, deadline: Instant, label: &str) -> Result<()> {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            bail!(
                "timed out waiting for {label}; transcript tail: {}",
                self.tail()
            );
        }
        match self.reader_rx.recv_timeout(remaining) {
            Ok(ReaderEvent::Bytes(bytes)) => self.transcript.extend(bytes),
            Ok(ReaderEvent::Closed(reason)) => {
                self.reader_closed = true;
                bail!(
                    "PTY closed before {label}{}; transcript tail: {}",
                    reason.map_or_else(String::new, |reason| format!(": {reason}")),
                    self.tail()
                );
            }
            Err(RecvTimeoutError::Timeout) => {
                bail!(
                    "timed out waiting for {label}; transcript tail: {}",
                    self.tail()
                );
            }
            Err(RecvTimeoutError::Disconnected) => {
                bail!(
                    "PTY reader disconnected before {label}; transcript tail: {}",
                    self.tail()
                );
            }
        }
        Ok(())
    }

    fn resize(&self, cols: u16, rows: u16) -> Result<()> {
        self.master
            .as_ref()
            .context("PTY master is unavailable")?
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
    }

    fn send(&mut self, input: &[u8]) -> Result<()> {
        let writer = self.writer.as_mut().context("PTY writer is unavailable")?;
        writer.write_all(input)?;
        writer.flush()?;
        Ok(())
    }

    fn finish(mut self, timeout: Duration) -> Result<(Vec<u8>, ExitStatus)> {
        let deadline = Instant::now() + timeout;
        let status = self.receive_status(deadline)?;
        while !self.reader_closed {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                bail!(
                    "PTY reader did not terminate; transcript tail: {}",
                    self.tail()
                );
            }
            match self.reader_rx.recv_timeout(remaining) {
                Ok(ReaderEvent::Bytes(bytes)) => self.transcript.extend(bytes),
                Ok(ReaderEvent::Closed(_)) => self.reader_closed = true,
                Err(RecvTimeoutError::Timeout) => {
                    bail!(
                        "PTY reader did not terminate; transcript tail: {}",
                        self.tail()
                    );
                }
                Err(RecvTimeoutError::Disconnected) => self.reader_closed = true,
            }
        }

        self.writer.take();
        self.master.take();
        if let Some(waiter) = self.waiter_thread.take() {
            waiter
                .join()
                .map_err(|_| anyhow::anyhow!("PTY waiter thread panicked"))?;
        }
        if let Some(reader) = self.reader_thread.take() {
            reader
                .join()
                .map_err(|_| anyhow::anyhow!("PTY reader thread panicked"))?;
        }
        self.finished = true;
        Ok((std::mem::take(&mut self.transcript), status))
    }

    fn receive_status(&mut self, deadline: Instant) -> Result<ExitStatus> {
        if let Some(status) = &self.status {
            return Ok(status.clone());
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            bail!("timed out waiting for PTY child exit");
        }
        let status = self
            .status_rx
            .recv_timeout(remaining)
            .context("wait for PTY child exit")??;
        self.status = Some(status.clone());
        Ok(status)
    }

    fn receive_cleanup_status(&mut self, timeout: Duration) -> bool {
        if self.status.is_some() {
            return true;
        }
        match self.status_rx.recv_timeout(timeout) {
            Ok(Ok(status)) => {
                self.status = Some(status);
                true
            }
            Ok(Err(_)) | Err(RecvTimeoutError::Disconnected | RecvTimeoutError::Timeout) => false,
        }
    }

    fn drain_reader_for_cleanup(&mut self, timeout: Duration) {
        if self.reader_closed {
            return;
        }
        let deadline = Instant::now() + timeout;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return;
            }
            match self.reader_rx.recv_timeout(remaining) {
                Ok(ReaderEvent::Bytes(bytes)) => self.transcript.extend(bytes),
                Ok(ReaderEvent::Closed(_)) | Err(RecvTimeoutError::Disconnected) => {
                    self.reader_closed = true;
                    return;
                }
                Err(RecvTimeoutError::Timeout) => return,
            }
        }
    }

    fn tail(&self) -> String {
        let start = self.transcript.len().saturating_sub(4_096);
        String::from_utf8_lossy(&self.transcript[start..])
            .escape_debug()
            .to_string()
    }
}

impl Drop for PtySession {
    fn drop(&mut self) {
        if self.finished {
            return;
        }

        if let Some(writer) = self.writer.as_mut() {
            let _ = writer.write_all(b"q");
            let _ = writer.flush();
        }
        let mut child_exited = self.receive_cleanup_status(CLEANUP_GRACE_TIMEOUT);
        let mut group_exited = !process_group_exists(self.process_group_id).unwrap_or(true);
        if !child_exited || !group_exited {
            let _ = send_signal("-HUP", format!("-{}", self.process_group_id));
            if !child_exited {
                child_exited = self.receive_cleanup_status(CLEANUP_GRACE_TIMEOUT);
            }
            group_exited =
                wait_for_process_group_exit(self.process_group_id, CLEANUP_GRACE_TIMEOUT);
        }
        if !child_exited || !group_exited {
            let _ = send_signal("-KILL", format!("-{}", self.process_group_id));
            if !child_exited {
                let _ = self.receive_cleanup_status(CLEANUP_FORCE_TIMEOUT);
            }
            let _ = wait_for_process_group_exit(self.process_group_id, CLEANUP_FORCE_TIMEOUT);
        }
        self.writer.take();
        self.master.take();
        self.drain_reader_for_cleanup(CLEANUP_GRACE_TIMEOUT);

        // Rust has no timed JoinHandle::join. Detach after bounded process/PTY
        // evidence rather than letting a test failure hang the whole suite.
        self.waiter_thread.take();
        self.reader_thread.take();
    }
}

#[test]
fn explicit_color_overrides_inherited_no_color_in_a_real_pty() -> Result<()> {
    let binary = assert_cmd::cargo::cargo_bin("hls");
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/hyperliquid/ws_mock_live.ndjson")
        .canonicalize()
        .context("canonical fixture path")?;
    let data_dir = tempfile::tempdir()?;
    let mut session =
        PtySession::spawn_with_no_color_env(&binary, &fixture, data_dir.path(), Some(1))?;

    session.wait_for("forced truecolor output", INITIAL_TIMEOUT, |bytes| {
        contains(bytes, ALT_SCREEN_ENTER)
            && contains(bytes, b"WATCHLIST")
            && contains(bytes, TRUECOLOR_FOREGROUND)
            && contains(bytes, TRUECOLOR_BACKGROUND)
    })?;
    session.wait_for("forced-color wrapper exit marker", EXIT_TIMEOUT, |bytes| {
        contains(bytes, EXIT_MARKER)
    })?;
    let (transcript, status) = session.finish(EXIT_TIMEOUT)?;

    ensure!(
        status.success(),
        "forced-color PTY child failed: {status:?}"
    );
    ensure!(marker_value(&transcript, "__HLS_EXIT_STATUS__=") == Some("0"));
    assert_balanced_terminal_sequences(&transcript)?;
    assert_stty_restored(&transcript)?;
    ensure!(contains(&transcript, TRUECOLOR_FOREGROUND));
    ensure!(contains(&transcript, TRUECOLOR_BACKGROUND));

    Ok(())
}

#[test]
fn fixture_tui_uses_production_terminal_session_and_restores_the_pty() -> Result<()> {
    let binary = assert_cmd::cargo::cargo_bin("hls");
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/hyperliquid/ws_mock_live.ndjson")
        .canonicalize()
        .context("canonical fixture path")?;
    let data_dir = tempfile::tempdir()?;
    let mut session = PtySession::spawn(&binary, &fixture, data_dir.path(), None)?;

    let initial_offset =
        session.wait_for("WATCHLIST and truecolor output", INITIAL_TIMEOUT, |bytes| {
            contains(bytes, ALT_SCREEN_ENTER)
                && contains(bytes, b"WATCHLIST")
                && contains(bytes, TRUECOLOR_FOREGROUND)
                && contains(bytes, TRUECOLOR_BACKGROUND)
                && count_occurrences(bytes, CURSOR_HIDE) >= 2
        })?;
    let initial_clear_count = count_occurrences(&session.transcript, FULL_SCREEN_CLEAR);
    ensure!(
        initial_clear_count == 1,
        "constructor must be the only pre-resize full-screen clear, got {initial_clear_count}; transcript tail: {}",
        session.tail()
    );
    ensure!(
        !contains(&session.transcript[..initial_offset], b"MICRO LAYOUT"),
        "120x40 should render the full workstation before the resize"
    );

    session.resize(72, 18)?;
    session.wait_for("MICRO LAYOUT resize redraw", RESIZE_TIMEOUT, |bytes| {
        contains(&bytes[initial_offset..], b"MICRO LAYOUT")
            && count_occurrences(bytes, FULL_SCREEN_CLEAR)
                == initial_clear_count + HORIZONTAL_SHRINK_CLEAR_COUNT
    })?;
    let resized_clear_count = count_occurrences(&session.transcript, FULL_SCREEN_CLEAR);
    ensure!(
        resized_clear_count == initial_clear_count + HORIZONTAL_SHRINK_CLEAR_COUNT,
        "horizontal shrink must add exactly {HORIZONTAL_SHRINK_CLEAR_COUNT} full-screen clears"
    );

    session.send(b"q")?;
    session.wait_for("wrapper exit marker", EXIT_TIMEOUT, |bytes| {
        contains(bytes, EXIT_MARKER)
    })?;
    let (transcript, status) = session.finish(EXIT_TIMEOUT)?;

    ensure!(status.success(), "PTY child failed: {status:?}");
    ensure!(marker_value(&transcript, "__HLS_EXIT_STATUS__=") == Some("0"));
    assert_balanced_terminal_sequences(&transcript)?;
    ensure!(contains(&transcript, TRUECOLOR_FOREGROUND));
    ensure!(contains(&transcript, TRUECOLOR_BACKGROUND));
    ensure!(contains(&transcript, b"MICRO LAYOUT"));
    ensure!(
        count_occurrences(&transcript, FULL_SCREEN_CLEAR) == resized_clear_count,
        "only the constructor and documented Ratatui horizontal-shrink resize may clear the full screen"
    );

    let stty_before =
        marker_value(&transcript, "__HLS_STTY_BEFORE__=").context("missing stty-before marker")?;
    let stty_after =
        marker_value(&transcript, "__HLS_STTY_AFTER__=").context("missing stty-after marker")?;
    ensure!(!stty_before.is_empty());
    ensure!(
        stty_before == stty_after,
        "terminal mode changed across TUI session: before={stty_before:?} after={stty_after:?}"
    );

    let active = between(&transcript, ALT_SCREEN_ENTER, ALT_SCREEN_LEAVE)
        .context("alternate-screen session was not balanced")?;
    let active_plain = strip_csi_sequences(active);
    let active_upper = String::from_utf8_lossy(&active_plain).to_ascii_uppercase();
    ensure!(
        active_upper.contains("STATUS FIXTURE"),
        "fixture replay must render fixture status"
    );
    ensure!(
        !active_upper.contains("STATUS LIVE"),
        "fixture replay must not identify itself as a live network session"
    );
    let active_lower = String::from_utf8_lossy(active).to_ascii_lowercase();
    for forbidden in [
        "live reconnect:",
        "tui preferences",
        "panicked at",
        "error:",
    ] {
        ensure!(
            !active_lower.contains(forbidden),
            "plain diagnostic {forbidden:?} leaked into alternate-screen output"
        );
    }

    Ok(())
}

#[test]
fn fixture_tui_honors_a_bounded_duration_without_operator_input() -> Result<()> {
    let binary = assert_cmd::cargo::cargo_bin("hls");
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/hyperliquid/ws_mock_live.ndjson")
        .canonicalize()
        .context("canonical fixture path")?;
    let data_dir = tempfile::tempdir()?;
    let mut session = PtySession::spawn(&binary, &fixture, data_dir.path(), Some(1))?;

    session.wait_for("bounded fixture initial frame", INITIAL_TIMEOUT, |bytes| {
        contains(bytes, b"WATCHLIST") && count_occurrences(bytes, CURSOR_HIDE) >= 2
    })?;
    session.wait_for("bounded fixture exit marker", EXIT_TIMEOUT, |bytes| {
        contains(bytes, EXIT_MARKER)
    })?;
    let (transcript, status) = session.finish(EXIT_TIMEOUT)?;

    ensure!(status.success(), "bounded PTY child failed: {status:?}");
    ensure!(marker_value(&transcript, "__HLS_EXIT_STATUS__=") == Some("0"));
    assert_balanced_terminal_sequences(&transcript)?;
    let stty_before = marker_value(&transcript, "__HLS_STTY_BEFORE__=")
        .context("missing bounded stty-before marker")?;
    let stty_after = marker_value(&transcript, "__HLS_STTY_AFTER__=")
        .context("missing bounded stty-after marker")?;
    ensure!(!stty_before.is_empty());
    ensure!(
        stty_before == stty_after,
        "bounded terminal mode changed: before={stty_before:?} after={stty_after:?}"
    );

    Ok(())
}

#[test]
fn fixture_tui_restores_terminal_when_signaled_during_initial_frame() -> Result<()> {
    let binary = assert_cmd::cargo::cargo_bin("hls");
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/hyperliquid/ws_mock_live.ndjson")
        .canonicalize()
        .context("canonical fixture path")?;

    for signal in ["-INT", "-TERM"] {
        let data_dir = tempfile::tempdir()?;
        let mut session = PtySession::spawn_with_size(
            &binary,
            &fixture,
            data_dir.path(),
            None,
            PtySize {
                rows: 40,
                cols: 120,
                pixel_width: 0,
                pixel_height: 0,
            },
        )?;

        session.wait_for("raw-mode startup", INITIAL_TIMEOUT, |bytes| {
            contains(bytes, ALT_SCREEN_ENTER) && contains(bytes, MOUSE_ENABLE)
        })?;
        session.signal_direct_child(signal)?;
        session.wait_for("signal closeout marker", EXIT_TIMEOUT, |bytes| {
            contains(bytes, EXIT_MARKER)
        })?;
        let (transcript, status) = session.finish(EXIT_TIMEOUT)?;

        ensure!(
            status.success(),
            "fixture TUI failed after startup {signal}: {status:?}; transcript tail: {}",
            transcript_tail(&transcript)
        );
        ensure!(marker_value(&transcript, "__HLS_EXIT_STATUS__=") == Some("0"));
        assert_balanced_terminal_sequences(&transcript)
            .with_context(|| format!("terminal escape restoration after {signal}"))?;
        assert_stty_restored(&transcript)
            .with_context(|| format!("termios restoration after {signal}"))?;
    }

    Ok(())
}

#[test]
fn pty_cleanup_escalates_past_ignored_hup_without_hanging() -> Result<()> {
    let mut session = PtySession::stubborn_child()?;
    session.wait_for("stubborn child readiness", INITIAL_TIMEOUT, |bytes| {
        contains(bytes, b"__HLS_STUBBORN_READY__")
    })?;
    let process_group_id = session.process_group_id();
    let (completed_tx, completed_rx) = mpsc::channel();
    let watchdog = thread::spawn(move || -> Result<bool> {
        match completed_rx.recv_timeout(CLEANUP_WATCHDOG_TIMEOUT) {
            Ok(()) => Ok(true),
            Err(RecvTimeoutError::Timeout) => {
                send_signal("-KILL", format!("-{process_group_id}"))?;
                Ok(false)
            }
            Err(RecvTimeoutError::Disconnected) => Ok(true),
        }
    });

    let started = Instant::now();
    drop(session);
    let elapsed = started.elapsed();
    let _ = completed_tx.send(());
    let completed_without_watchdog = watchdog
        .join()
        .map_err(|_| anyhow::anyhow!("cleanup watchdog panicked"))??;

    ensure!(
        completed_without_watchdog,
        "PTY cleanup required the external {}s watchdog",
        CLEANUP_WATCHDOG_TIMEOUT.as_secs()
    );
    ensure!(
        elapsed < CLEANUP_WATCHDOG_TIMEOUT,
        "PTY cleanup exceeded its bounded deadline: {elapsed:?}"
    );
    ensure!(
        !process_group_exists(process_group_id)?,
        "PTY cleanup left live members in process group {process_group_id}"
    );
    Ok(())
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

fn count_occurrences(haystack: &[u8], needle: &[u8]) -> usize {
    haystack
        .windows(needle.len())
        .filter(|window| *window == needle)
        .count()
}

fn strip_csi_sequences(input: &[u8]) -> Vec<u8> {
    let mut plain = Vec::with_capacity(input.len());
    let mut index = 0;
    while index < input.len() {
        if input[index] == b'\x1b' && input.get(index + 1) == Some(&b'[') {
            index += 2;
            while index < input.len() {
                let byte = input[index];
                index += 1;
                if (0x40..=0x7e).contains(&byte) {
                    break;
                }
            }
        } else {
            plain.push(input[index]);
            index += 1;
        }
    }
    plain
}

fn assert_balanced_terminal_sequences(transcript: &[u8]) -> Result<()> {
    assert_exact_pair(
        transcript,
        ALT_SCREEN_ENTER,
        ALT_SCREEN_LEAVE,
        1,
        "alternate screen",
    )?;
    assert_exact_pair(transcript, MOUSE_ENABLE, MOUSE_DISABLE, 1, "mouse capture")?;

    let hidden = count_occurrences(transcript, CURSOR_HIDE);
    let shown = count_occurrences(transcript, CURSOR_SHOW);
    ensure!(hidden > 0, "cursor was never hidden");
    ensure!(shown > 0, "cursor was never restored");
    ensure!(
        last_occurrence(transcript, CURSOR_SHOW) > last_occurrence(transcript, CURSOR_HIDE),
        "final cursor-show must follow the final cursor-hide"
    );
    Ok(())
}

fn assert_exact_pair(
    transcript: &[u8],
    enter: &[u8],
    leave: &[u8],
    expected: usize,
    label: &str,
) -> Result<()> {
    let enters = count_occurrences(transcript, enter);
    let leaves = count_occurrences(transcript, leave);
    ensure!(
        enters == expected && leaves == expected,
        "{label} escapes are unbalanced: enter={enters} leave={leaves} expected={expected}"
    );
    ensure!(
        last_occurrence(transcript, leave) > last_occurrence(transcript, enter),
        "final {label} restore must follow its final activation"
    );
    Ok(())
}

fn assert_stty_restored(transcript: &[u8]) -> Result<()> {
    let before =
        marker_value(transcript, "__HLS_STTY_BEFORE__=").context("missing stty-before marker")?;
    let raw_after = marker_value(transcript, "__HLS_STTY_AFTER_RAW__=")
        .context("missing raw stty-after marker")?;
    let after =
        marker_value(transcript, "__HLS_STTY_AFTER__=").context("missing stty-after marker")?;
    ensure!(!before.is_empty() && !raw_after.is_empty());
    ensure!(
        stable_stty_mode(before) == stable_stty_mode(after),
        "terminal mode changed across TUI session: before={before:?} after={after:?}"
    );
    Ok(())
}

fn stable_stty_mode(mode: &str) -> String {
    #[cfg(target_os = "macos")]
    {
        const PENDIN: u64 = 0x2000_0000;
        mode.split(':')
            .map(|field| {
                let Some(value) = field.strip_prefix("lflag=") else {
                    return field.to_owned();
                };
                u64::from_str_radix(value, 16).map_or_else(
                    |_| field.to_owned(),
                    |flags| format!("lflag={:x}", flags & !PENDIN),
                )
            })
            .collect::<Vec<_>>()
            .join(":")
    }

    #[cfg(not(target_os = "macos"))]
    mode.to_owned()
}

fn last_occurrence(haystack: &[u8], needle: &[u8]) -> usize {
    haystack
        .windows(needle.len())
        .rposition(|window| window == needle)
        .unwrap_or(0)
}

fn direct_child_process(parent: u32) -> Result<u32> {
    let output = Command::new("/bin/ps")
        .args(["-eo", "pid=,ppid="])
        .output()
        .context("list PTY child processes")?;
    ensure!(output.status.success(), "/bin/ps failed: {}", output.status);
    let processes = String::from_utf8(output.stdout).context("parse /bin/ps output")?;
    processes
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let pid = fields.next()?.parse::<u32>().ok()?;
            let ppid = fields.next()?.parse::<u32>().ok()?;
            (ppid == parent).then_some(pid)
        })
        .next()
        .with_context(|| format!("no direct child found for PTY wrapper {parent}"))
}

fn send_signal(signal: &str, target: String) -> Result<()> {
    let output = Command::new("/bin/kill")
        .arg(signal)
        .arg("--")
        .arg(&target)
        .output()
        .with_context(|| format!("invoke /bin/kill {signal} {target}"))?;
    ensure!(
        output.status.success(),
        "/bin/kill {signal} {target} failed: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    );
    Ok(())
}

fn process_group_exists(process_group_id: i32) -> Result<bool> {
    let output = Command::new("/bin/ps")
        .args(["-eo", "pgid=,stat="])
        .output()
        .context("probe PTY process group")?;
    ensure!(output.status.success(), "/bin/ps failed: {}", output.status);
    let processes = String::from_utf8(output.stdout).context("parse PTY process-group state")?;
    Ok(processes.lines().any(|line| {
        let mut fields = line.split_whitespace();
        let Some(group) = fields.next().and_then(|value| value.parse::<i32>().ok()) else {
            return false;
        };
        let Some(state) = fields.next() else {
            return false;
        };
        group == process_group_id && !state.starts_with('Z')
    }))
}

fn wait_for_process_group_exit(process_group_id: i32, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        if !process_group_exists(process_group_id).unwrap_or(true) {
            return true;
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return false;
        }
        thread::sleep(remaining.min(Duration::from_millis(10)));
    }
}

fn transcript_tail(transcript: &[u8]) -> String {
    let start = transcript.len().saturating_sub(4_096);
    String::from_utf8_lossy(&transcript[start..])
        .escape_debug()
        .to_string()
}

fn marker_value<'a>(transcript: &'a [u8], marker: &str) -> Option<&'a str> {
    let marker = marker.as_bytes();
    let marker_start = transcript
        .windows(marker.len())
        .position(|window| window == marker)?;
    let value = &transcript[marker_start + marker.len()..];
    let value_end = value
        .iter()
        .position(|byte| matches!(byte, b'\r' | b'\n'))
        .unwrap_or(value.len());
    std::str::from_utf8(&value[..value_end]).ok()
}

fn between<'a>(haystack: &'a [u8], start: &[u8], end: &[u8]) -> Option<&'a [u8]> {
    let start_index = haystack
        .windows(start.len())
        .position(|window| window == start)?;
    let content_start = start_index + start.len();
    let end_offset = haystack[content_start..]
        .windows(end.len())
        .position(|window| window == end)?;
    Some(&haystack[content_start..content_start + end_offset])
}
