use hls_core::data_gap::DataGap;
use hls_store::metadata::{FileRegistryEntry, MetadataRegistry, RecordingRun, SymbolRegistryEntry};

#[test]
fn metadata_registry_tracks_runs_files_and_data_gaps() {
    let temp = tempfile::tempdir().expect("tempdir");
    let registry = MetadataRegistry::open(temp.path().join("hls.sqlite")).expect("open registry");

    registry
        .insert_run(&RecordingRun::new(
            "run-meta",
            1_710_000_000_000,
            true,
            true,
        ))
        .expect("insert run");
    registry
        .insert_symbol(&SymbolRegistryEntry::new(
            "@107",
            1_710_000_000_000,
            1_710_000_060_000,
        ))
        .expect("insert symbol");
    registry
        .insert_file(&FileRegistryEntry {
            path: "raw/ws/date=2026-07-07/hour=12/part-000000.ndjson.zst".to_owned(),
            event_type: "raw_ws".to_owned(),
            symbol: Some("@107".to_owned()),
            start_ts_ms: Some(1_710_000_000_000),
            end_ts_ms: Some(1_710_000_060_000),
            rows: 6,
            bytes: 512,
            created_at_ms: 1_710_000_060_000,
            run_id: "run-meta".to_owned(),
        })
        .expect("insert file");
    registry
        .insert_gap(&DataGap::new(
            "run-meta",
            7,
            1_710_000_030_000_000_000,
            1_710_000_031_000_000_000,
            "fixture gap",
            vec!["@107".to_owned()],
            true,
        ))
        .expect("insert gap");
    registry
        .finish_run("run-meta", 1_710_000_061_000, true)
        .expect("finish run");

    let run = registry
        .get_run("run-meta")
        .expect("get run")
        .expect("run exists");
    assert_eq!(run.run_id, "run-meta");
    assert_eq!(run.clean_shutdown, Some(true));
    assert_eq!(run.gap_count, 1);

    let symbols = registry.list_symbols().expect("symbols");
    assert_eq!(symbols.len(), 1);
    assert_eq!(symbols[0].hl_coin, "@107");

    let files = registry.list_files("run-meta").expect("files");
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].rows, 6);

    let gaps = registry.list_gaps("run-meta").expect("gaps");
    assert_eq!(gaps.len(), 1);
    assert_eq!(gaps[0].reason, "fixture gap");
}
