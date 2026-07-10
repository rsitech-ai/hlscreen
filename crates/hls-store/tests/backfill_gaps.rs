use std::collections::BTreeMap;

use hls_core::{confidence::ConfidenceReason, data_gap::DataGap, market_state::CandleEvent};
use hls_store::{
    backfill::{
        BackfillGapsOptions, CandleBackfillRequest, CandleBackfillSource, backfill_public_gaps,
    },
    metadata::{BackfillConfidenceImpact, BackfillStatus, MetadataRegistry},
    normalized::read_normalized_events,
    recorder::{RecordOptions, record_fixture_ndjson},
    replay::{ReplayOptions, replay_run},
};

#[test]
fn backfill_rejects_path_shaping_run_ids_and_intervals_before_writing() {
    let temp = tempfile::tempdir().expect("tempdir");
    let data_dir = temp.path().join("data");
    let source = FixtureCandleSource::default();

    let run_error = backfill_public_gaps(BackfillGapsOptions::new(&data_dir, "../escape"), &source)
        .expect_err("run ID path components must be rejected");
    assert!(run_error.to_string().contains("run ID"));
    assert!(!data_dir.exists());

    let interval_error = backfill_public_gaps(
        BackfillGapsOptions::new(&data_dir, "valid-run").with_interval("../1m"),
        &source,
    )
    .expect_err("unsupported interval must be rejected before opening metadata");
    assert!(
        interval_error
            .to_string()
            .contains("supported public candle interval")
    );
    assert!(!data_dir.exists());
}

#[test]
fn public_candle_backfill_writes_coarse_rows_but_keeps_tick_gap_degraded() {
    let temp = tempfile::tempdir().expect("tempdir");
    let data_dir = temp.path().join("data");
    record_fixture_ndjson(
        include_str!("../../../tests/fixtures/microstructure/gap_replay.ndjson"),
        RecordOptions::new(
            &data_dir,
            "backfill-partial",
            vec!["@107".to_owned()],
            false,
            true,
        ),
    )
    .expect("record fixture");
    let registry = MetadataRegistry::open(data_dir.join("hls.sqlite")).expect("registry");
    let gap = DataGap::new(
        "backfill-partial",
        7,
        1_710_000_060_000_000_000,
        1_710_000_120_000_000_000,
        "fixture reconnect gap",
        vec!["@107".to_owned()],
        false,
    );
    registry.insert_gap(&gap).expect("insert gap");

    let mut source = FixtureCandleSource::default();
    source.insert(
        "@107",
        vec![CandleEvent {
            recv_ts_ns: gap.ended_at_ns,
            open_ts_ms: 1_710_000_060_000,
            close_ts_ms: 1_710_000_119_999,
            hl_coin: "@107".to_owned(),
            interval: "1m".to_owned(),
            open: 35.0,
            high: 35.4,
            low: 34.9,
            close: 35.2,
            volume_base: 25.0,
            trade_count: 12,
            provenance: Default::default(),
            completion: Default::default(),
        }],
    );

    let summary = backfill_public_gaps(
        BackfillGapsOptions::new(&data_dir, "backfill-partial").with_interval("1m"),
        &source,
    )
    .expect("backfill succeeds");

    assert_eq!(summary.gaps_examined, 1);
    assert_eq!(summary.gaps_repaired, 0);
    assert_eq!(summary.gaps_partially_repaired, 1);
    assert_eq!(summary.rows_written, 1);

    let attempts = registry
        .list_backfill_attempts("backfill-partial")
        .expect("attempts");
    assert_eq!(attempts.len(), 1);
    assert_eq!(attempts[0].status, BackfillStatus::PartiallyRepaired);
    assert_eq!(
        attempts[0].confidence_impact,
        BackfillConfidenceImpact::Partial
    );

    let gaps = registry.list_gaps("backfill-partial").expect("gaps");
    assert!(!gaps[0].recovered);
    let files = registry.list_files("backfill-partial").expect("files");
    let backfill_file = files
        .iter()
        .find(|file| file.path.contains("backfill-"))
        .expect("backfill file is registered");
    let recovered_events =
        read_normalized_events(data_dir.join(&backfill_file.path)).expect("read backfill file");
    assert_eq!(recovered_events.len(), 1);

    let replay = replay_run(ReplayOptions::new(
        &data_dir,
        "backfill-partial",
        Vec::new(),
    ))
    .expect("replay");
    let snapshot = replay
        .snapshots
        .iter()
        .find(|snapshot| snapshot.symbol == "@107")
        .expect("snapshot");
    assert!(
        snapshot
            .confidence
            .has_reason(ConfidenceReason::ReconnectGap),
        "coarse candle rows must not hide a missing trade/BBO interval"
    );
}

#[test]
fn unrepaired_public_backfill_attempt_keeps_gap_confidence_degraded() {
    let temp = tempfile::tempdir().expect("tempdir");
    let data_dir = temp.path().join("data");
    record_fixture_ndjson(
        include_str!("../../../tests/fixtures/microstructure/gap_replay.ndjson"),
        RecordOptions::new(
            &data_dir,
            "backfill-unrepaired",
            vec!["@107".to_owned()],
            false,
            true,
        ),
    )
    .expect("record fixture");
    let registry = MetadataRegistry::open(data_dir.join("hls.sqlite")).expect("registry");
    let gap = DataGap::new(
        "backfill-unrepaired",
        8,
        1_710_000_060_000_000_000,
        1_710_000_120_000_000_000,
        "fixture reconnect gap",
        vec!["@107".to_owned()],
        false,
    );
    registry.insert_gap(&gap).expect("insert gap");

    let summary = backfill_public_gaps(
        BackfillGapsOptions::new(&data_dir, "backfill-unrepaired").with_interval("1m"),
        &FixtureCandleSource::default(),
    )
    .expect("backfill succeeds");

    assert_eq!(summary.gaps_examined, 1);
    assert_eq!(summary.gaps_unrepaired, 1);
    assert_eq!(summary.rows_written, 0);
    let attempts = registry
        .list_backfill_attempts("backfill-unrepaired")
        .expect("attempts");
    assert_eq!(attempts[0].status, BackfillStatus::Unrepaired);
    assert_eq!(
        attempts[0].confidence_impact,
        BackfillConfidenceImpact::Degraded
    );

    let replay = replay_run(ReplayOptions::new(
        &data_dir,
        "backfill-unrepaired",
        Vec::new(),
    ))
    .expect("replay");
    let snapshot = replay
        .snapshots
        .iter()
        .find(|snapshot| snapshot.symbol == "@107")
        .expect("snapshot");
    assert!(
        snapshot
            .confidence
            .has_reason(ConfidenceReason::ReconnectGap),
        "unrepaired backfill should keep reconnect-gap confidence penalty"
    );
}

#[test]
fn failed_backfill_registration_rolls_back_registry_and_file() {
    let temp = tempfile::tempdir().expect("tempdir");
    let data_dir = temp.path().join("data");
    let run_id = "backfill-atomic-failure";
    record_fixture_ndjson(
        include_str!("../../../tests/fixtures/microstructure/gap_replay.ndjson"),
        RecordOptions::new(&data_dir, run_id, vec!["@107".to_owned()], false, true),
    )
    .expect("record fixture");
    let registry = MetadataRegistry::open(data_dir.join("hls.sqlite")).expect("registry");
    let gap = DataGap::new(
        run_id,
        9,
        1_710_000_060_000_000_000,
        1_710_000_120_000_000_000,
        "fixture reconnect gap",
        vec!["@107".to_owned()],
        false,
    );
    registry.insert_gap(&gap).expect("insert gap");
    let database = rusqlite::Connection::open(data_dir.join("hls.sqlite")).expect("open sqlite");
    database
        .execute_batch(
            "CREATE TRIGGER reject_backfill_attempt
             BEFORE INSERT ON backfill_attempts
             BEGIN
                 SELECT RAISE(ABORT, 'injected backfill registration failure');
             END;",
        )
        .expect("install failure trigger");
    drop(database);

    let mut source = FixtureCandleSource::default();
    source.insert(
        "@107",
        vec![CandleEvent {
            recv_ts_ns: gap.ended_at_ns,
            open_ts_ms: 1_710_000_060_000,
            close_ts_ms: 1_710_000_119_999,
            hl_coin: "@107".to_owned(),
            interval: "1m".to_owned(),
            open: 35.0,
            high: 35.4,
            low: 34.9,
            close: 35.2,
            volume_base: 25.0,
            trade_count: 12,
            provenance: Default::default(),
            completion: Default::default(),
        }],
    );

    let error = backfill_public_gaps(
        BackfillGapsOptions::new(&data_dir, run_id).with_interval("1m"),
        &source,
    )
    .expect_err("injected registry failure must fail the backfill");
    assert!(
        error
            .to_string()
            .contains("injected backfill registration failure")
    );
    assert!(
        registry
            .list_files(run_id)
            .expect("files")
            .iter()
            .all(|file| {
                !file
                    .path
                    .rsplit('/')
                    .next()
                    .is_some_and(|name| name.starts_with("backfill-"))
            }),
        "file registry insert must roll back with the attempt"
    );
    assert!(
        registry
            .list_backfill_attempts(run_id)
            .expect("attempts")
            .is_empty()
    );
    let backfill_dir = data_dir.join(format!("normalized/events/run={run_id}"));
    assert!(
        std::fs::read_dir(backfill_dir)
            .expect("normalized run directory")
            .all(|entry| !entry
                .expect("directory entry")
                .file_name()
                .to_string_lossy()
                .contains("backfill-")),
        "failed registration must remove the unregistered backfill artifact"
    );
}

#[derive(Default)]
struct FixtureCandleSource {
    candles_by_symbol: BTreeMap<String, Vec<CandleEvent>>,
}

impl FixtureCandleSource {
    fn insert(&mut self, symbol: &str, candles: Vec<CandleEvent>) {
        self.candles_by_symbol.insert(symbol.to_owned(), candles);
    }
}

impl CandleBackfillSource for FixtureCandleSource {
    fn candle_snapshot(
        &self,
        request: &CandleBackfillRequest<'_>,
    ) -> hls_core::HlsResult<Vec<CandleEvent>> {
        Ok(self
            .candles_by_symbol
            .get(request.symbol)
            .cloned()
            .unwrap_or_default())
    }
}
