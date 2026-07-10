use std::fs;

use hls_store::{
    metadata::{FileRegistryEntry, MetadataRegistry, RecordingRun},
    recorder::{RecordOptions, record_fixture_ndjson},
    replay::{ReplayOptions, replay_run},
};
use rusqlite::{Connection, params};

const FIXTURE: &str = include_str!("../../../tests/fixtures/hyperliquid/ws_mock_live.ndjson");

#[test]
fn recording_rejects_run_ids_with_path_components_before_writing() {
    let temp = tempfile::tempdir().expect("tempdir");
    let data_dir = temp.path().join("data");
    let options = RecordOptions::new(&data_dir, "../escape", vec!["@107".to_owned()], true, true);

    let error = record_fixture_ndjson(FIXTURE, options)
        .expect_err("run IDs must not influence recorder path structure");

    assert!(error.to_string().contains("run ID"));
    assert!(
        !data_dir.exists(),
        "invalid input must fail before SQLite or recorder directories are created"
    );
}

#[test]
fn recording_rejects_duplicate_run_ids_without_replacing_evidence() {
    let temp = tempfile::tempdir().expect("tempdir");
    let first = RecordOptions::new(
        temp.path(),
        "duplicate",
        vec!["@107".to_owned()],
        true,
        true,
    );
    record_fixture_ndjson(FIXTURE, first).expect("first run records");

    let second = RecordOptions::new(
        temp.path(),
        "duplicate",
        vec!["@107".to_owned()],
        true,
        false,
    );
    let error = record_fixture_ndjson(FIXTURE, second)
        .expect_err("a completed run must not be silently replaced");

    assert!(error.to_string().contains("already exists"));
    let replay = replay_run(ReplayOptions::new(temp.path(), "duplicate", Vec::new()))
        .expect("the original normalized evidence remains replayable");
    assert_eq!(replay.events_read, 6);
}

#[test]
fn file_registry_does_not_replace_existing_evidence_rows() {
    let temp = tempfile::tempdir().expect("tempdir");
    let registry = MetadataRegistry::open(temp.path().join("hls.sqlite")).expect("registry");
    registry
        .insert_run(&RecordingRun::new("append-only", 1, true, false))
        .expect("run insert");
    let first = FileRegistryEntry {
        path: "raw/ws/run=append-only/part-000000.ndjson.zst".to_owned(),
        event_type: "raw_ws".to_owned(),
        symbol: None,
        start_ts_ms: None,
        end_ts_ms: None,
        rows: 1,
        bytes: 10,
        created_at_ms: 1,
        run_id: "append-only".to_owned(),
    };
    registry.insert_file(&first).expect("first file insert");
    let mut replacement = first;
    replacement.rows = 99;

    registry
        .insert_file(&replacement)
        .expect_err("existing file evidence must remain append-only");
    let files = registry.list_files("append-only").expect("file rows");
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].rows, 1);
}

#[test]
fn replay_rejects_registry_paths_that_escape_the_data_directory() {
    let temp = tempfile::tempdir().expect("tempdir");
    let data_dir = temp.path().join("data");
    fs::create_dir_all(&data_dir).expect("data directory");
    let outside = temp.path().join("outside.ndjson");
    fs::write(&outside, FIXTURE.lines().next().expect("fixture line")).expect("outside fixture");

    let registry = MetadataRegistry::open(data_dir.join("hls.sqlite")).expect("registry");
    let mut run = RecordingRun::new("escaped", 1, false, true);
    run.ended_at_ms = Some(2);
    run.clean_shutdown = Some(true);
    registry.insert_run(&run).expect("run metadata");
    drop(registry);
    let conn = Connection::open(data_dir.join("hls.sqlite")).expect("raw registry connection");
    conn.execute(
        "INSERT INTO files (
            path, event_type, symbol, start_ts_ms, end_ts_ms, rows, bytes, created_at_ms, run_id
        ) VALUES (?1, ?2, NULL, NULL, NULL, ?3, ?4, ?5, ?6)",
        params![
            "../outside.ndjson",
            "normalized_jsonl",
            1_u64,
            fs::metadata(&outside).expect("outside metadata").len(),
            1_i64,
            "escaped"
        ],
    )
    .expect("malformed legacy registry entry");

    let error = replay_run(ReplayOptions::new(&data_dir, "escaped", Vec::new()))
        .expect_err("replay must not trust an operator-controlled registry path");
    assert!(error.to_string().contains("registered data path"));
}

#[cfg(unix)]
#[test]
fn replay_rejects_symlinks_that_escape_the_data_directory() {
    use std::os::unix::fs::symlink;

    let temp = tempfile::tempdir().expect("tempdir");
    let data_dir = temp.path().join("data");
    let relative = "normalized/events/run=symlink/part-000000.ndjson";
    let link = data_dir.join(relative);
    fs::create_dir_all(link.parent().expect("link parent")).expect("link parent");
    let outside = temp.path().join("outside.ndjson");
    fs::write(&outside, FIXTURE.lines().next().expect("fixture line")).expect("outside fixture");
    symlink(&outside, &link).expect("escaping symlink");

    let registry = MetadataRegistry::open(data_dir.join("hls.sqlite")).expect("registry");
    let mut run = RecordingRun::new("symlink", 1, false, true);
    run.ended_at_ms = Some(2);
    run.clean_shutdown = Some(true);
    registry.insert_run(&run).expect("run insert");
    registry
        .insert_file(&FileRegistryEntry {
            path: relative.to_owned(),
            event_type: "normalized_jsonl".to_owned(),
            symbol: None,
            start_ts_ms: None,
            end_ts_ms: None,
            rows: 1,
            bytes: 1,
            created_at_ms: 1,
            run_id: "symlink".to_owned(),
        })
        .expect("lexically valid path insert");

    let error = replay_run(ReplayOptions::new(&data_dir, "symlink", Vec::new()))
        .expect_err("replay must remain under the canonical data directory");
    assert!(error.to_string().contains("outside"), "{error}");
}

#[cfg(unix)]
#[test]
fn recording_rejects_symlinked_run_directories() {
    use std::os::unix::fs::symlink;

    let temp = tempfile::tempdir().expect("tempdir");
    let data_dir = temp.path().join("data");
    let outside = temp.path().join("outside");
    fs::create_dir_all(data_dir.join("raw/ws")).expect("raw parent");
    fs::create_dir_all(&outside).expect("outside directory");
    symlink(&outside, data_dir.join("raw/ws/run=symlinked")).expect("run symlink");

    let options = RecordOptions::new(&data_dir, "symlinked", vec!["@107".to_owned()], true, false);
    let error =
        record_fixture_ndjson(FIXTURE, options).expect_err("writer must not follow run symlinks");

    assert!(error.to_string().contains("symbolic link"), "{error}");
    assert_eq!(
        fs::read_dir(&outside)
            .expect("outside remains readable")
            .count(),
        0
    );
}
