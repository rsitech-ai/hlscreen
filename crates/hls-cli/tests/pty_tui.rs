#![cfg(unix)]

use std::{
    io::{self, Read, Write},
    path::Path,
    sync::mpsc::{self, Receiver, RecvTimeoutError},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use anyhow::{Context, Result, bail, ensure};
use portable_pty::{
    ChildKiller, CommandBuilder, ExitStatus, MasterPty, PtySize, native_pty_system,
};

const INITIAL_TIMEOUT: Duration = Duration::from_secs(15);
const RESIZE_TIMEOUT: Duration = Duration::from_secs(10);
const EXIT_TIMEOUT: Duration = Duration::from_secs(10);
const CLEANUP_TIMEOUT: Duration = Duration::from_secs(3);

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
after=$(/bin/stty -g) || exit 98
printf '__HLS_STTY_AFTER__=%s\n' "$after"
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
    killer: Box<dyn ChildKiller + Send + Sync>,
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
        let pair = native_pty_system().openpty(PtySize {
            rows: 40,
            cols: 120,
            pixel_width: 0,
            pixel_height: 0,
        })?;
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
        command.env_remove("NO_COLOR");
        command.env_remove("FORCE_COLOR");
        command.env_remove("CLICOLOR_FORCE");
        command.env_remove("HLS_FORCE_COLOR");

        let mut reader = pair.master.try_clone_reader()?;
        let writer = pair.master.take_writer()?;
        let mut child = pair.slave.spawn_command(command)?;
        let killer = child.clone_killer();
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
            killer,
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
        let exited = if self.status.is_some() {
            true
        } else {
            match self.status_rx.recv_timeout(CLEANUP_TIMEOUT) {
                Ok(Ok(status)) => {
                    self.status = Some(status);
                    true
                }
                Ok(Err(_)) | Err(_) => false,
            }
        };
        if !exited {
            let _ = self.killer.kill();
            let _ = self.status_rx.recv_timeout(CLEANUP_TIMEOUT);
        }
        self.writer.take();
        self.master.take();
        if let Some(waiter) = self.waiter_thread.take() {
            let _ = waiter.join();
        }
        if let Some(reader) = self.reader_thread.take() {
            let _ = reader.join();
        }
    }
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
            && count_occurrences(bytes, FULL_SCREEN_CLEAR) == initial_clear_count + 1
    })?;
    let resized_clear_count = count_occurrences(&session.transcript, FULL_SCREEN_CLEAR);
    ensure!(
        resized_clear_count == initial_clear_count + 1,
        "resize must add exactly one full-screen clear"
    );

    session.send(b"q")?;
    session.wait_for("wrapper exit marker", EXIT_TIMEOUT, |bytes| {
        contains(bytes, EXIT_MARKER)
    })?;
    let (transcript, status) = session.finish(EXIT_TIMEOUT)?;

    ensure!(status.success(), "PTY child failed: {status:?}");
    ensure!(marker_value(&transcript, "__HLS_EXIT_STATUS__=") == Some("0"));
    ensure!(contains(&transcript, ALT_SCREEN_ENTER));
    ensure!(contains(&transcript, ALT_SCREEN_LEAVE));
    ensure!(contains(&transcript, MOUSE_ENABLE));
    ensure!(contains(&transcript, MOUSE_DISABLE));
    ensure!(contains(&transcript, CURSOR_HIDE));
    ensure!(contains(&transcript, CURSOR_SHOW));
    ensure!(contains(&transcript, TRUECOLOR_FOREGROUND));
    ensure!(contains(&transcript, TRUECOLOR_BACKGROUND));
    ensure!(contains(&transcript, b"MICRO LAYOUT"));
    ensure!(
        count_occurrences(&transcript, FULL_SCREEN_CLEAR) == resized_clear_count,
        "only the constructor and documented Ratatui resize may clear the full screen"
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
    ensure!(contains(&transcript, ALT_SCREEN_ENTER));
    ensure!(contains(&transcript, ALT_SCREEN_LEAVE));
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
