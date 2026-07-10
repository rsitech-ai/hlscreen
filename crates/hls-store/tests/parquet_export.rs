use std::{
    fs::{self, File},
    io::Read,
};

use hls_core::market_state::MarketEvent;
use hls_store::{
    metadata::MetadataRegistry,
    parquet::{export_feature_snapshots_to_parquet, export_normalized_events_to_parquet},
    recorder::{RecordOptions, record_fixture_ndjson},
    schema::{
        CURRENT_NORMALIZED_EVENT_SCHEMA_VERSION, CURRENT_PARQUET_EVENT_SCHEMA_VERSION,
        CURRENT_PARQUET_FEATURE_SCHEMA_VERSION, CURRENT_SQLITE_SCHEMA_VERSION,
        StorageSchemaManifest,
    },
};
use parquet::{
    file::reader::{FileReader, SerializedFileReader},
    record::RowAccessor,
};

#[test]
fn normalized_events_export_to_readable_parquet() {
    let temp = tempfile::tempdir().expect("temp dir");
    let summary = record_fixture_ndjson(
        include_str!("../../../tests/fixtures/hyperliquid/ws_mock_live.ndjson"),
        RecordOptions::new(
            temp.path(),
            "parquet-fixture",
            vec!["@107".to_owned()],
            false,
            true,
        ),
    )
    .expect("record fixture");

    let exported = export_normalized_events_to_parquet(temp.path(), "parquet-fixture")
        .expect("export normalized events to parquet");

    assert_eq!(exported.event_type, "normalized_parquet");
    assert_eq!(exported.run_id, "parquet-fixture");
    assert_eq!(exported.rows, summary.normalized_events);
    assert!(exported.bytes > 0);
    assert_eq!(
        exported.path,
        "parquet/events/run=parquet-fixture/part-000000.parquet"
    );

    let parquet_path = temp.path().join(&exported.path);
    let mut magic = [0_u8; 4];
    File::open(&parquet_path)
        .expect("open parquet")
        .read_exact(&mut magic)
        .expect("read parquet magic");
    assert_eq!(&magic, b"PAR1");

    let reader = SerializedFileReader::new(File::open(&parquet_path).expect("open reader"))
        .expect("read parquet file");
    assert_eq!(
        reader.metadata().file_metadata().num_rows(),
        i64::try_from(summary.normalized_events).expect("row count fits i64")
    );

    let mut rows = reader.get_row_iter(None).expect("row iterator");
    let first = rows.next().expect("first row exists").expect("first row");
    assert_eq!(first.get_long(0).expect("row_index"), 0);
    assert_eq!(first.get_string(1).expect("event_type"), "trade");
    assert_eq!(first.get_string(3).expect("hl_coin"), "@107");
    assert!(
        first
            .get_string(4)
            .expect("event_json")
            .contains("\"Trade\"")
    );
    assert_eq!(rows.count() + 1, summary.normalized_events as usize);

    let schema_path = temp
        .path()
        .join("parquet/events/run=parquet-fixture/schema.json");
    let manifest =
        StorageSchemaManifest::read_from_path(&schema_path).expect("read parquet schema manifest");
    manifest
        .validate_supported()
        .expect("parquet schema manifest validates");
    assert_eq!(
        manifest.normalized_event_schema_version,
        CURRENT_NORMALIZED_EVENT_SCHEMA_VERSION
    );
    assert_eq!(
        manifest.sqlite_schema_version,
        CURRENT_SQLITE_SCHEMA_VERSION
    );
    assert_eq!(
        manifest.parquet_event_schema_version,
        Some(CURRENT_PARQUET_EVENT_SCHEMA_VERSION)
    );

    let registry = MetadataRegistry::open(temp.path().join("hls.sqlite")).expect("registry");
    let files = registry
        .list_files("parquet-fixture")
        .expect("registered files");
    assert!(files.iter().any(|file| file == &exported));
}

#[test]
fn feature_snapshots_export_to_readable_parquet() {
    let temp = tempfile::tempdir().expect("temp dir");
    record_fixture_ndjson(
        include_str!("../../../tests/fixtures/hyperliquid/ws_mock_live.ndjson"),
        RecordOptions::new(
            temp.path(),
            "feature-parquet",
            vec!["@107".to_owned()],
            false,
            true,
        ),
    )
    .expect("record fixture");

    let exported = export_feature_snapshots_to_parquet(temp.path(), "feature-parquet")
        .expect("export feature snapshots to parquet");

    assert_eq!(exported.event_type, "feature_snapshot_parquet");
    assert_eq!(exported.run_id, "feature-parquet");
    assert!(exported.rows > 0);
    assert!(exported.bytes > 0);
    assert_eq!(
        exported.path,
        "parquet/features/run=feature-parquet/part-000000.parquet"
    );

    let parquet_path = temp.path().join(&exported.path);
    let reader = SerializedFileReader::new(File::open(&parquet_path).expect("open reader"))
        .expect("read parquet file");
    assert_eq!(
        reader.metadata().file_metadata().num_rows(),
        i64::try_from(exported.rows).expect("row count fits i64")
    );

    let mut rows = reader.get_row_iter(None).expect("row iterator");
    let first = rows.next().expect("first row exists").expect("first row");
    assert_eq!(first.get_long(0).expect("row_index"), 0);
    assert!(first.get_long(1).expect("snapshot_ts_ms") > 0);
    assert_eq!(first.get_string(2).expect("symbol"), "@107");
    assert_eq!(first.get_long(3).expect("confidence_score"), 100);
    assert_eq!(first.get_string(4).expect("confidence_level"), "high");
    assert_eq!(first.get_string(5).expect("confidence_reasons_json"), "[]");
    assert!(first.get_double(6).expect("price").is_finite());
    assert!(first.get_double(8).expect("spread_bps").is_finite());
    assert_eq!(first.get_string(14).expect("tradeability_state"), "unknown");
    assert!(
        first
            .get_string(16)
            .expect("snapshot_json")
            .contains("\"confidence\"")
    );

    let schema_path = temp
        .path()
        .join("parquet/features/run=feature-parquet/schema.json");
    let manifest =
        StorageSchemaManifest::read_from_path(&schema_path).expect("read feature schema manifest");
    manifest
        .validate_supported()
        .expect("feature parquet schema manifest validates");
    assert_eq!(
        manifest.parquet_feature_schema_version,
        Some(CURRENT_PARQUET_FEATURE_SCHEMA_VERSION)
    );

    let registry = MetadataRegistry::open(temp.path().join("hls.sqlite")).expect("registry");
    let files = registry
        .list_files("feature-parquet")
        .expect("registered files");
    assert!(files.iter().any(|file| file == &exported));
}

#[test]
fn parquet_export_preserves_committed_jsonl_fixture_rows() {
    let temp = tempfile::tempdir().expect("temp dir");
    let run_id = "parquet-parity";
    let normalized_dir = temp
        .path()
        .join("normalized/events")
        .join(format!("run={run_id}"));
    fs::create_dir_all(&normalized_dir).expect("create normalized dir");
    fs::write(
        normalized_dir.join("part-000000.ndjson"),
        include_str!("../../../tests/fixtures/microstructure/parquet_parity_events.ndjson"),
    )
    .expect("write fixture normalized events");

    let fixture_lines =
        include_str!("../../../tests/fixtures/microstructure/parquet_parity_events.ndjson")
            .lines()
            .filter(|line| !line.trim().is_empty())
            .collect::<Vec<_>>();
    let source_events = fixture_lines
        .iter()
        .map(|line| serde_json::from_str::<MarketEvent>(line).expect("fixture event parses"))
        .collect::<Vec<_>>();

    let exported = export_normalized_events_to_parquet(temp.path(), run_id)
        .expect("export fixture normalized events to parquet");
    assert_eq!(exported.rows, source_events.len() as u64);

    let parquet_path = temp.path().join(&exported.path);
    let reader = SerializedFileReader::new(File::open(&parquet_path).expect("open reader"))
        .expect("read parquet file");
    let rows = reader
        .get_row_iter(None)
        .expect("row iterator")
        .map(|row| row.expect("parquet row"))
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), source_events.len());

    for (index, (row, event)) in rows.iter().zip(source_events.iter()).enumerate() {
        assert_eq!(
            row.get_long(0).expect("row_index"),
            i64::try_from(index).expect("index fits i64")
        );
        assert_eq!(
            row.get_string(1).expect("event_type"),
            expected_event_type(event)
        );
        assert_eq!(
            row.get_long(2).expect("recv_ts_ns"),
            i64::try_from(event.recv_ts_ns()).expect("recv_ts fits i64")
        );
        match event.hl_coin() {
            Some(hl_coin) => assert_eq!(row.get_string(3).expect("hl_coin"), hl_coin),
            None => assert!(row.get_string(3).is_err()),
        }
        let parquet_event =
            serde_json::from_str::<MarketEvent>(row.get_string(4).expect("event_json"))
                .expect("parquet event json parses");
        assert_eq!(&parquet_event, event);
    }
}

fn expected_event_type(event: &MarketEvent) -> &'static str {
    match event {
        MarketEvent::Trade(_) => "trade",
        MarketEvent::TopOfBook(_) => "top_of_book",
        MarketEvent::OrderBook(_) => "order_book",
        MarketEvent::AssetContext(_) => "asset_context",
        MarketEvent::AllMids(_) => "all_mids",
        MarketEvent::Candle(_) => "candle",
    }
}
