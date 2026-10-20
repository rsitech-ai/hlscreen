//! Actual public bootstrap/owner/replay exercise of output amplification.
//! Synthetic long-symbol catalog; no exchange coverage or financial claim.
use hls_hyperliquid::shared::{SharedConsumer, SharedUpdate};
use marketfeed_adapter_api::*;
use marketfeed_daemon::causal_bootstrap::{
    PreparedBootstrap,
    owner::{InputSettlement, OwnerPhase},
    projection::{ProjectionPump, compare_with_source},
};
use marketfeed_engine::{CaptureInput, causal::projection::*, causal::*};
use marketfeed_model::*;
use marketfeed_recording::{shared_batch::*, shared_feed::*};
#[path = "support/mfop.rs"]
#[allow(dead_code)] // The same actual-factory fixture also serves CLI/value tests.
mod mfop;

fn bytes(updates: &[SharedUpdate]) -> usize {
    updates
        .iter()
        .map(|u| serde_json::to_vec(u).unwrap().len())
        .sum()
}

#[test]
fn actual_gap_amplification_defers_whole_bootstrap_without_future_metadata_or_duplicate_updates() {
    const CAP: usize = 8 << 20;
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    let target = temp.path().join("target");
    let names: Vec<_> = (0..24)
        .map(|i| format!("COIN{i}{}", "X".repeat(2000)))
        .collect();
    let meta = serde_json::to_vec(&serde_json::json!([{
        "collateralToken":0,
        "universe":names.iter().map(|name| serde_json::json!({"name":name,"szDecimals":5})).collect::<Vec<_>>()
    }])).unwrap();
    let symbols: Vec<_> = names.iter().map(|name| format!("perps:{name}")).collect();
    let symbol_bytes = bytes(&[SharedUpdate::Symbols(symbols.clone())]);
    let gap_bytes = bytes(&[SharedUpdate::Gap {
        symbols: symbols.clone(),
        reason: "committed source safety/continuity transition".into(),
    }]);
    let count = (CAP - symbol_bytes) / gap_bytes;
    // Choose a legitimate source workload whose next bootstrap cannot fit the
    // converted batch, while compact input groups remain well below its cap.
    assert!(CAP - symbol_bytes - count * gap_bytes < symbol_bytes);
    assert!(count > 100);
    let selected: Vec<_> = (1..=names.len() as u32).collect();
    let mut dto = mfop::fixture_with_meta(false, mfop::SPOT, &meta, &selected);
    dto.bind_observation_policy(NativeObservationPolicyDto::HyperliquidV1 {
        limits: ObservedBatchLimits {
            layout: NativeObservationLimits {
                rows: 64,
                associations: 128,
                book_levels: 40,
                mids: 0,
                users: 128,
                string_bytes: 4096,
            },
            encoded_bytes: 64 << 10,
        },
    })
    .unwrap();
    let actor = FeedBatchActor::spawn(
        FeedWriter::create_profile(&source, 64 << 20, FeedProfile::EngineCausalV4).unwrap(),
        mfop::batch(),
    )
    .unwrap();
    for session in [31, 32] {
        dto.capture.session = session;
        let owner = PreparedBootstrap::new_observed_history(dto.clone(), MAX_OPERATION_BYTES)
            .unwrap()
            .try_admit(&actor.handle())
            .unwrap()
            .wait()
            .unwrap()
            .instantiate()
            .unwrap()
            .spawn_history_owner()
            .unwrap();
        if session == 31 {
            for index in 0..count {
                assert!(matches!(
                    owner
                        .ingress()
                        .try_submit(
                            ConnectionId(0),
                            CaptureInput::Disconnected {
                                reason: DisconnectReason::RemoteClose,
                                now: TimestampNs(index as i64 + 1),
                            }
                        )
                        .unwrap()
                        .wait(),
                    InputSettlement::Committed { .. }
                ));
                mfop::wait(|| {
                    let release = owner.try_next();
                    let status = owner.status();
                    assert!(status.owner.failure.is_none(), "{status:?}");
                    release.is_some()
                });
            }
        }
        owner.close_ingress();
        mfop::wait(|| owner.status().owner.phase == OwnerPhase::Stopped);
        assert!(owner.shutdown().owner.failure.is_none());
    }
    actor
        .handle()
        .try_submit(
            PreparedFeedTransaction::new(vec![FeedRecord::Seal {
                schema_version: 1,
                now_ns: i64::MAX,
                reason: "actual output-budget fixture complete".into(),
            }])
            .unwrap(),
        )
        .unwrap()
        .wait()
        .unwrap();
    actor.shutdown().unwrap();
    eprintln!(
        "output-budget fixture: gaps={count} symbol_bytes={symbol_bytes} gap_bytes={gap_bytes} source_bytes={}",
        std::fs::metadata(source.join("events.mfsf")).unwrap().len()
    );
    // Long synthetic catalog needs larger source-record capacity than the
    // small value fixtures. HLS input/output caps remain the production values.
    let mut pump = ProjectionPump::create(
        mfop::source_with_record_limit(&source, 1 << 20),
        ProjectionPolicy {
            schema_version: 1,
            source_profile: 4,
            clock_policy: ProjectionClockPolicy::PerOwnerStartedPhysicalOrderV1,
            limits: mfop::limits(),
        },
        &target,
        64 << 20,
        mfop::batch(),
    )
    .unwrap();
    // This new fixture writes >8MiB of converted updates across hundreds of
    // actual durable operations. Its fixed test wall-clock budget is separate
    // from the unchanged small-fixture/producer 15s watchdog and owner policies.
    let until = std::time::Instant::now() + std::time::Duration::from_secs(60);
    loop {
        let poll = pump.poll().unwrap();
        if pump.last_acknowledged().sealed {
            break;
        }
        assert!(
            std::time::Instant::now() < until,
            "large output fixture projection watchdog: poll={poll:?} prepared={} ack={:?}",
            pump.prepared_groups(),
            pump.last_acknowledged()
        );
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    pump.shutdown().unwrap();
    assert!(
        compare_with_source(
            mfop::source_with_record_limit(&source, 1 << 20),
            ProjectionReader::open(&target, mfop::limits()).unwrap()
        )
        .unwrap()
        .sealed
    );

    let mut reference = SharedConsumer::open(&target).unwrap();
    let mut groups = ProjectionReader::open(&target, mfop::limits()).unwrap();
    let mut expected = Vec::new();
    let mut before_late_bootstrap = None;
    let mut source_before_late_bootstrap = None;
    let mut encoded_input = 0;
    while let Some(group) = groups.read_next().unwrap() {
        encoded_input += serde_json::to_vec(&group).unwrap().len();
        if matches!(&group.source, ProjectionSource::Bootstrap(dto) if dto.capture.session == 32) {
            before_late_bootstrap = Some(reference.cursor().clone());
            source_before_late_bootstrap = reference.source_cursor().cloned();
            assert!(reference.metadata(32).unwrap().is_none());
        }
        expected.extend(reference.poll(1).unwrap());
    }
    assert!(encoded_input < CAP, "input cap must not explain the split");
    assert!(bytes(&expected) > CAP);
    let mut consumer = SharedConsumer::open(&target).unwrap();
    let mut actual = consumer
        .consume(4096, |updates| {
            assert!(bytes(&updates) <= CAP);
            Ok(updates)
        })
        .unwrap();
    assert_eq!(consumer.cursor(), before_late_bootstrap.as_ref().unwrap());
    assert_eq!(
        consumer.source_cursor(),
        source_before_late_bootstrap.as_ref()
    );
    assert!(consumer.metadata(31).unwrap().is_some());
    assert!(
        consumer.metadata(32).unwrap().is_none(),
        "pending bootstrap leaked"
    );
    assert_eq!(consumer.instruments().len(), 24);
    assert!(!consumer.caught_up());
    while !consumer.caught_up() {
        actual.extend(
            consumer
                .consume(4096, |updates| {
                    assert!(bytes(&updates) <= CAP);
                    Ok(updates)
                })
                .unwrap(),
        );
    }
    assert!(consumer.metadata(32).unwrap().is_some());
    assert_eq!(consumer.instruments().len(), 48);
    assert!(consumer.sealed());
    assert_eq!(consumer.cursor(), reference.cursor());
    assert_eq!(consumer.source_cursor(), reference.source_cursor());
    assert_eq!(
        serde_json::to_vec(&actual).unwrap(),
        serde_json::to_vec(&expected).unwrap()
    );
    assert_eq!(
        actual
            .iter()
            .filter(|u| matches!(u, SharedUpdate::Symbols(_)))
            .count(),
        2
    );
}
