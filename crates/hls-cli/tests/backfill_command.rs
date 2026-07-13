use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use assert_cmd::Command;
use hls_core::{confidence::ConfidenceReason, data_gap::DataGap};
use hls_store::{
    metadata::{BackfillStatus, MetadataRegistry},
    recorder::{RecordOptions, record_fixture_ndjson},
    replay::{ReplayOptions, replay_run},
};
use predicates::prelude::*;

const GAP_START_NS: u64 = 1_710_000_060_000_000_000;
const GAP_END_NS: u64 = 1_710_000_120_000_000_000;

fn fixture(path: &str) -> String {
    format!("{}/../../{path}", env!("CARGO_MANIFEST_DIR"))
}

fn seed_gap_run(data_dir: &std::path::Path, run_id: &str) {
    let fixture =
        std::fs::read_to_string(fixture("tests/fixtures/microstructure/gap_replay.ndjson"))
            .expect("read replay fixture");
    record_fixture_ndjson(
        &fixture,
        RecordOptions::new(data_dir, run_id, vec!["@107".to_owned()], false, true),
    )
    .expect("record fixture run");
    MetadataRegistry::open(data_dir.join("hls.sqlite"))
        .expect("open registry")
        .insert_gap(&DataGap::new(
            run_id,
            7,
            GAP_START_NS,
            GAP_END_NS,
            "fixture reconnect gap",
            vec!["@107".to_owned()],
            false,
        ))
        .expect("insert gap");
}

fn spawn_candle_server() -> (String, mpsc::Receiver<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind REST fixture");
    listener
        .set_nonblocking(true)
        .expect("nonblocking REST fixture");
    let address = listener.local_addr().expect("REST fixture address");
    let (request_tx, request_rx) = mpsc::channel();
    thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    stream
                        .set_read_timeout(Some(Duration::from_secs(1)))
                        .expect("REST fixture read timeout");
                    let mut request = Vec::new();
                    let mut buffer = [0_u8; 4096];
                    loop {
                        match stream.read(&mut buffer) {
                            Ok(0) => break,
                            Ok(count) => {
                                request.extend_from_slice(&buffer[..count]);
                                if request
                                    .windows(br#""type":"candleSnapshot""#.len())
                                    .any(|window| window == br#""type":"candleSnapshot""#)
                                {
                                    break;
                                }
                            }
                            Err(error)
                                if matches!(
                                    error.kind(),
                                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                                ) =>
                            {
                                assert!(
                                    Instant::now() < deadline,
                                    "REST fixture body was not received"
                                );
                            }
                            Err(error) => panic!("read REST request: {error}"),
                        }
                    }
                    let request = String::from_utf8(request).expect("UTF-8 REST request");
                    request_tx.send(request).expect("publish REST request");
                    let body = r#"[{"t":1710000060000,"T":1710000119999,"s":"@107","i":"1m","o":"35.0","c":"35.2","h":"35.4","l":"34.9","v":"25.0","n":12}]"#;
                    write!(
                        stream,
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    )
                    .expect("write REST response");
                    break;
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(Instant::now() < deadline, "REST fixture was not called");
                    thread::sleep(Duration::from_millis(10));
                }
                Err(error) => panic!("REST fixture accept failed: {error}"),
            }
        }
    });
    (format!("http://{address}"), request_rx)
}

#[test]
fn backfill_command_appends_coarse_candles_without_restoring_tick_confidence() {
    let temp = tempfile::tempdir().expect("tempdir");
    let data_dir = temp.path().join("data");
    let run_id = "cli-public-gap-repair";
    seed_gap_run(&data_dir, run_id);
    let (rest_url, request_rx) = spawn_candle_server();

    Command::cargo_bin("hls")
        .expect("hls binary")
        .args([
            "backfill",
            "--run-id",
            run_id,
            "--interval",
            "1m",
            "--rest-url",
            &rest_url,
            "--data-dir",
        ])
        .arg(&data_dir)
        .assert()
        .success()
        .stdout(predicate::str::contains("backfill_run=complete"))
        .stdout(predicate::str::contains("gaps_partially_repaired=1"))
        .stdout(predicate::str::contains("rows_written=1"))
        .stdout(predicate::str::contains("tick_gaps_recovered=0"));

    let request = request_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("candle request");
    assert!(request.contains(r#""type":"candleSnapshot""#));
    assert!(request.contains(r#""coin":"@107""#));

    let registry = MetadataRegistry::open(data_dir.join("hls.sqlite")).expect("registry");
    let attempts = registry
        .list_backfill_attempts(run_id)
        .expect("backfill attempts");
    assert_eq!(attempts.len(), 1);
    assert_eq!(attempts[0].status, BackfillStatus::PartiallyRepaired);
    assert!(!registry.list_gaps(run_id).expect("gaps")[0].recovered);

    let replay =
        replay_run(ReplayOptions::new(&data_dir, run_id, Vec::new())).expect("replay repaired run");
    assert!(
        replay
            .snapshots
            .iter()
            .find(|snapshot| snapshot.symbol == "@107")
            .expect("symbol snapshot")
            .confidence
            .has_reason(ConfidenceReason::ReconnectGap)
    );
}
