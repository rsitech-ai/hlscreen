//! Actual MFCJ5 owner + opaque wire calls + verified MFOP1 + HLS conversion.
//! In-memory transport only; no venue authentication or availability claim.
use hls_core::{HlsError, market_state::MarketEvent};
use hls_hyperliquid::shared::{SharedConsumer, SharedUpdate, parse_shared_ws_message};
use marketfeed_adapter_api::{NativeObservationLimits, ObservedBatchLimits};
use marketfeed_daemon::causal_bootstrap::{
    PreparedBootstrap,
    owner::{InputSettlement, OwnerFailure, OwnerPhase, history::HistoryJournalOwner},
    projection::{ProjectionPoll, ProjectionPump, compare_with_source},
};
use marketfeed_engine::{CaptureInput, causal::projection::*, causal::*};
use marketfeed_model::{ConnectionId, FrameStamp, TimestampNs};
use marketfeed_recording::{shared_batch::*, shared_feed::*};
use marketfeed_transport::*;
use std::{
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
#[path = "support/mfop.rs"]
#[allow(dead_code)]
mod mfop;

const CLOCK: i64 = 1_710_000_000_000_000_000;
fn hls_limits() -> ProjectionLimits {
    ProjectionLimits {
        max_group_bytes: 8 << 20,
        max_items: 4096,
        max_owners: 10,
        max_bootstrap_bytes: 16 << 20,
        max_catalog_entries: 10_000,
        max_pending_inputs: 4096,
    }
}
#[test]
fn public_capture_profile_fits_unchanged_hls_reader_limits() {
    let capture = marketfeed_daemon::causal_bootstrap::capture::projection_limits();
    let hls = hls_limits();
    assert!(capture.max_group_bytes <= hls.max_group_bytes);
    assert!(capture.max_items <= hls.max_items);
    assert!(capture.max_owners <= hls.max_owners);
    assert!(capture.max_bootstrap_bytes <= hls.max_bootstrap_bytes);
    assert!(capture.max_catalog_entries <= hls.max_catalog_entries);
    assert!(capture.max_pending_inputs <= hls.max_pending_inputs);
}

struct Transport {
    inner: MemoryWebSocket,
    mode: usize,
    calls: Arc<AtomicUsize>,
}
impl WebSocketTransport for Transport {
    async fn connect(&mut self, spec: &WebSocketSpec) -> Result<(), TransportError> {
        self.inner.connect(spec).await
    }
    async fn read_frame(
        &mut self,
        buffer: &mut FrameBuffer,
    ) -> Result<InboundFrame, TransportError> {
        self.inner.read_frame(buffer).await
    }
    async fn write_frame(&mut self, frame: OutboundFrame) -> Result<(), TransportError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        match self.mode {
            0 => self.inner.write_frame(frame).await,
            1 => Err(TransportError::Stub(
                "declared offline application-write failure".into(),
            )),
            2 => std::future::pending().await,
            _ => unreachable!(),
        }
    }
    async fn close(&mut self, reason: CloseReason) -> Result<(), TransportError> {
        self.inner.close(reason).await
    }
}
fn dto(spot: bool) -> BootstrapDto {
    let mut dto = mfop::fixture(spot);
    dto.session
        .subscriptions
        .retain(|s| s.channel == ChannelDto::Trades);
    dto.protocol_topics = marketfeed_adapter_hyperliquid::HyperliquidFactory::public_plan_topics(
        spot,
        &dto.session.session_spec().unwrap(),
        &dto.catalog.catalog().unwrap(),
    )
    .unwrap();
    assert_eq!(dto.protocol_topics.len(), 1);
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
    dto.bind_wire_policy(WirePolicyDto {
        schema_version: 1,
        max_pending_writes: 1,
        max_pending_bytes: 64 << 10,
        transport_controls: TransportControlsDto::ManagedConnectReadAutomaticPongCloseUnjournaledV1,
    })
    .unwrap();
    dto
}
fn actor(path: &Path) -> FeedBatchActor {
    FeedBatchActor::spawn(
        FeedWriter::create_profile(path, 64 << 20, FeedProfile::EngineCausalV5).unwrap(),
        mfop::batch(),
    )
    .unwrap()
}
fn owner(dto: BootstrapDto, actor: &FeedBatchActor) -> HistoryJournalOwner {
    PreparedBootstrap::new_wire_history(dto, MAX_OPERATION_BYTES)
        .unwrap()
        .try_admit(&actor.handle())
        .unwrap()
        .wait()
        .unwrap()
        .instantiate()
        .unwrap()
        .spawn_history_owner()
        .unwrap()
}
fn handoff(owner: &HistoryJournalOwner, input: CaptureInput) {
    assert!(matches!(
        owner
            .ingress()
            .try_submit(ConnectionId(1), input)
            .unwrap()
            .wait(),
        InputSettlement::Committed { .. }
    ));
    mfop::wait(|| owner.try_next().is_some());
}
async fn flush(owner: &HistoryJournalOwner, transport: &mut Transport) {
    tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            while let Some(token) = owner.try_take_wire() {
                assert_eq!(
                    token.invoke(transport).await.unwrap().outcome,
                    WireOutcomeDto::Succeeded
                );
            }
            let status = owner.status();
            if status.owner.phase == OwnerPhase::Idle
                && status.owner.pending_operations == 0
                && status.ingress_observations == 0
                && owner.wire_progress() == Some((0, 0, true))
            {
                break;
            }
            assert!(status.owner.failure.is_none(), "{status:?}");
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .expect("actual owner wire-flush watchdog");
}
fn text(payload: String, now: i64) -> CaptureInput {
    CaptureInput::Text {
        payload: payload.into_bytes(),
        received: FrameStamp {
            receive_ts: TimestampNs(now),
            mono_ns: u64::try_from(now - CLOCK).unwrap(),
        },
    }
}
async fn bootstrap_market(
    owner: &HistoryJournalOwner,
    dto: &BootstrapDto,
    transport: &mut Transport,
) -> (String, String, i64) {
    handoff(
        owner,
        CaptureInput::Connected {
            connection: ConnectionId(1),
            now: TimestampNs(CLOCK),
        },
    );
    flush(owner, transport).await;
    for topic in &dto.protocol_topics {
        handoff(
            owner,
            text(
                format!(
                    r#"{{"channel":"subscriptionResponse","data":{{"method":"subscribe","subscription":{topic}}}}}"#
                ),
                CLOCK + 1,
            ),
        );
        flush(owner, transport).await;
    }
    let coin = &dto.catalog.instruments[0].key.native_symbol;
    let raw = format!(
        r#"{{"channel":"trades","data":[{{"coin":"{coin}","side":"B","px":"10.25","sz":"2","hash":"h1","time":1710000000001,"tid":1}}]}}"#
    );
    handoff(owner, text(raw.clone(), CLOCK + 2));
    flush(owner, transport).await;
    let product = if matches!(
        dto.factory,
        FactoryDto::HyperliquidPublic {
            market: HyperliquidMarketDto::Spot
        }
    ) {
        "spot"
    } else {
        "perps"
    };
    (format!("{product}:{coin}"), raw, CLOCK + 2)
}
fn project(source: &Path, target: &Path, sealed: bool) {
    let mut pump = ProjectionPump::create(
        mfop::source_profile(source, FeedProfile::EngineCausalV5),
        ProjectionPolicy {
            schema_version: 1,
            source_profile: 5,
            clock_policy: ProjectionClockPolicy::PerOwnerStartedPhysicalOrderV1,
            limits: marketfeed_daemon::causal_bootstrap::capture::projection_limits(),
        },
        target,
        64 << 20,
        // Match the capture target actor: its bound includes the maximum 8MiB
        // projection group plus journal framing. Consumer limits stay unchanged.
        BatchConfig {
            max_pending_operations: 16,
            max_pending_bytes: 32 << 20,
            max_group_operations: 8,
            max_group_bytes: 16 << 20,
            max_age: Duration::from_millis(1),
        },
    )
    .unwrap();
    mfop::wait(|| {
        matches!(
            pump.poll().unwrap(),
            ProjectionPoll::Sealed | ProjectionPoll::CaughtUp
        )
    });
    assert_eq!(pump.last_acknowledged().sealed, sealed);
    pump.shutdown().unwrap();
}
async fn successful_source(path: &Path) -> Vec<(String, String, i64)> {
    let actor = actor(path);
    let mut expected = Vec::new();
    for spot in [true, false] {
        let mut dto = dto(spot);
        dto.capture.session = if spot { 31 } else { 32 };
        let owner = owner(dto.clone(), &actor);
        let mut transport = Transport {
            inner: MemoryWebSocket::new(),
            mode: 0,
            calls: Arc::new(AtomicUsize::new(0)),
        };
        transport.connect(&WebSocketSpec::default()).await.unwrap();
        expected.push(bootstrap_market(&owner, &dto, &mut transport).await);
        handoff(
            &owner,
            CaptureInput::TimerPoll {
                now: TimestampNs(CLOCK + 21_000_000_000),
            },
        );
        flush(&owner, &mut transport).await;
        assert_eq!(transport.inner.outbound.len(), 2);
        assert_eq!(transport.inner.outbound[1].payload, br#"{"method":"ping"}"#);
        let stop = owner
            .ingress()
            .try_submit(ConnectionId(1), CaptureInput::Stop)
            .unwrap()
            .wait();
        assert!(matches!(stop, InputSettlement::Committed { .. }));
        owner.close_ingress();
        mfop::wait(|| owner.status().owner.phase == OwnerPhase::Stopped);
        assert_eq!(owner.wire_progress(), Some((0, 0, true)));
        assert_eq!(
            owner.shutdown().owner.failure,
            Some(OwnerFailure::NativeStopped)
        );
    }
    actor
        .handle()
        .try_submit(
            PreparedFeedTransaction::new(vec![FeedRecord::Seal {
                schema_version: 1,
                now_ns: i64::MAX,
                reason: "closed offline MFCJ5 source; no network".into(),
            }])
            .unwrap(),
        )
        .unwrap()
        .wait()
        .unwrap();
    actor.shutdown().unwrap();
    expected
}
#[tokio::test]
async fn mfcj5_spot_perps_native_values_and_control_only_groups_are_consumed_atomically() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    let target = temp.path().join("target");
    let native = successful_source(&source).await;
    project(&source, &target, true);
    assert!(
        compare_with_source(
            mfop::source_profile(&source, FeedProfile::EngineCausalV5),
            ProjectionReader::open(&target, hls_limits()).unwrap()
        )
        .unwrap()
        .sealed
    );
    let mut expected = Vec::new();
    for (key, raw, received) in native {
        for mut event in parse_shared_ws_message(&raw).unwrap() {
            let MarketEvent::Trade(trade) = &mut event else {
                panic!("trade-only fixture")
            };
            trade.hl_coin = key.clone();
            expected.push(event.with_recv_ts_ns(received as u64));
        }
    }
    let mut reader = ProjectionReader::open(&target, hls_limits()).unwrap();
    let mut consumer = SharedConsumer::open(&target).unwrap();
    let mut actual = Vec::new();
    let mut wire_groups = 0;
    let mut group_classes = std::collections::BTreeMap::<&str, u64>::new();
    let mut items = 0;
    let mut gaps = 0;
    while let Some(group) = reader.read_next().unwrap() {
        items += group.items.len();
        let class = match &group.source {
            ProjectionSource::Bootstrap(_) => "bootstrap",
            ProjectionSource::Owner {
                event: ProjectionOwnerEvent::Wire(_),
                ..
            } => "wire",
            ProjectionSource::Owner { .. } => "owner_control",
            ProjectionSource::Seal { .. } => "seal",
        };
        *group_classes.entry(class).or_default() += 1;
        let updates = consumer.poll(1).unwrap();
        if matches!(
            group.source,
            ProjectionSource::Owner {
                event: ProjectionOwnerEvent::Wire(_),
                ..
            }
        ) {
            wire_groups += 1;
            assert!(
                updates.is_empty(),
                "successful wire control is no feature: {updates:?}"
            );
        }
        for update in updates {
            match update {
                SharedUpdate::Events(events) => actual.extend(events),
                SharedUpdate::Gap { .. } => gaps += 1,
                _ => {}
            }
        }
    }
    assert_eq!(wire_groups, 12);
    assert_eq!(actual, expected);
    assert_eq!(actual.len(), 2);
    assert!(consumer.sealed());
    assert_eq!(consumer.symbols(), ["perps:BTC", "spot:@12"]);
    assert_eq!(
        consumer.source_cursor(),
        reader.progress().unwrap().source_cursor.as_ref()
    );
    let mut reopened = SharedConsumer::open(&target).unwrap();
    let mut replay = Vec::new();
    while !reopened.caught_up() {
        for update in reopened.poll(1).unwrap() {
            if let SharedUpdate::Events(events) = update {
                replay.extend(events);
            }
        }
    }
    assert_eq!(replay, actual);
    assert_eq!(reopened.cursor(), consumer.cursor());
    if let Some(destination) = std::env::var_os("HLS_MFOP5_FIXTURE_EXPORT") {
        export_synthetic_fixture(
            Path::new(&destination),
            &source,
            &target,
            &consumer,
            serde_json::json!({"group_classes":group_classes,"items":items,"wire_groups":wire_groups}),
            gaps,
        );
    }
}
// Explicit test-only export after every genuine owner/wire/native/replay check.
// No product/debug CLI. Partial failed exports remain unqualified and are never
// silently overwritten; callers must choose a fresh root for another attempt.
fn export_synthetic_fixture(
    destination: &Path,
    source: &Path,
    target: &Path,
    consumer: &SharedConsumer,
    counts: serde_json::Value,
    gaps: u64,
) {
    use std::{fs, io::Write};
    fs::create_dir(destination).expect("fixture export destination must be new");
    for (name, from) in [("source", source), ("feed", target)] {
        let to = destination.join(name);
        fs::create_dir(&to).unwrap();
        let mut paths = fs::read_dir(from)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect::<Vec<_>>();
        paths.sort();
        for path in paths {
            let metadata = path.symlink_metadata().unwrap();
            assert!(metadata.is_file(), "regular fixture files only");
            assert!(metadata.len() <= 2 << 20, "finite synthetic fixture bound");
            let bytes = fs::read(&path).unwrap();
            assert_eq!(bytes.len() as u64, metadata.len());
            fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(to.join(path.file_name().unwrap()))
                .unwrap()
                .write_all(&bytes)
                .unwrap();
        }
    }
    let metadata = serde_json::json!({
        "schema_version":1,"synthetic":true,"source_profile":5,"owner_history_schema":4,
        "generator":"mfcj5_spot_perps_native_values_and_control_only_groups_are_consumed_atomically",
        "generator_source":"crates/hls-hyperliquid/tests/projected_wire.rs",
        "generator_executable":std::env::current_exe().unwrap(),
        "generator_build":marketfeed_daemon::causal_bootstrap::current_build().unwrap(),
        "source_native_verified_at_generation":true,
        "counts":counts,"symbols":consumer.symbols(),
        "expected_summary":{"cursor":consumer.cursor(),"source_cursor":consumer.source_cursor(),
            "events":{"trade":2},"data_gaps":gaps,"sealed":true,"as_of_ms":consumer.as_of_ms(),
            "clock_policy":consumer.clock_policy(),"instruments":consumer.instruments(),
            "availability_time_ns":null,"float_projection":"features_and_display_only"},
        "expected_market_events":2,
        "limits":"Synthetic consumer fixture. Portable consumption is structural only, not same-platform producer authentication, live or availability qualification."
    });
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination.join("export.json"))
        .unwrap()
        .write_all(&serde_json::to_vec_pretty(&metadata).unwrap())
        .unwrap();
}
async fn failed_source(path: &Path, unknown: bool) {
    let actor = actor(path);
    let dto = dto(true);
    let owner = owner(dto.clone(), &actor);
    let calls = Arc::new(AtomicUsize::new(0));
    let mut transport = Transport {
        inner: MemoryWebSocket::new(),
        mode: 0,
        calls: calls.clone(),
    };
    transport.connect(&WebSocketSpec::default()).await.unwrap();
    bootstrap_market(&owner, &dto, &mut transport).await;
    handoff(
        &owner,
        CaptureInput::TimerPoll {
            now: TimestampNs(CLOCK + 21_000_000_000),
        },
    );
    let mut token = None;
    mfop::wait(|| {
        token = owner.try_take_wire();
        token.is_some()
    });
    transport.mode = if unknown { 2 } else { 1 };
    if unknown {
        let before = calls.load(Ordering::SeqCst);
        let invocation = token.unwrap().invoke(&mut transport);
        tokio::pin!(invocation);
        tokio::time::timeout(Duration::from_secs(15), async {
            loop {
                tokio::select! {
                    result = &mut invocation => panic!("pending fixture returned: {result:?}"),
                    _ = tokio::time::sleep(Duration::from_millis(1)) => {
                        if calls.load(Ordering::SeqCst) > before {
                            break;
                        }
                    }
                }
            }
        })
        .await
        .unwrap();
        // Scope exit drops the actual in-flight invocation, recording Unknown.
    } else {
        assert_eq!(
            token.unwrap().invoke(&mut transport).await.unwrap().outcome,
            WireOutcomeDto::Failed
        );
    }
    owner.close_ingress();
    mfop::wait(|| owner.status().owner.phase == OwnerPhase::Stopped);
    assert!(!owner.wire_progress().unwrap().2);
    owner.shutdown();
    actor.shutdown().unwrap();
}
async fn assert_failed_wire_gap(unknown: bool) {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    let target = temp.path().join("target");
    failed_source(&source, unknown).await;
    project(&source, &target, false);
    let mut reader = ProjectionReader::open(&target, hls_limits()).unwrap();
    let mut consumer = SharedConsumer::open(&target).unwrap();
    let mut saw = false;
    let mut trades = 0;
    while let Some(group) = reader.read_next().unwrap() {
        let unsafe_wire = matches!(
            group.source,
            ProjectionSource::Owner {
                event: ProjectionOwnerEvent::Wire(WireEventDto::Settled {
                    outcome: WireOutcomeDto::Failed | WireOutcomeDto::Unknown,
                    ..
                }),
                ..
            }
        );
        let updates = consumer.poll(1).unwrap();
        if unsafe_wire {
            assert!(
                updates
                    .iter()
                    .any(|u| matches!(u,SharedUpdate::Gap {symbols,..} if symbols==&["spot:@12"]))
            );
            assert!(!updates.iter().any(|u| matches!(u, SharedUpdate::Events(_))));
            saw = true;
        }
        for update in updates {
            if let SharedUpdate::Events(events) = update {
                trades += events.len();
            }
        }
    }
    assert!(saw);
    assert_eq!(trades, 1);
    assert!(!consumer.sealed());
    assert!(consumer.poll(1).unwrap().is_empty());
    assert!(consumer.caught_up());
}
#[tokio::test]
async fn failed_application_write_invalidates_at_its_committed_wire_group() {
    assert_failed_wire_gap(false).await;
}
#[tokio::test]
async fn cancelled_application_write_invalidates_at_its_committed_unknown_group() {
    assert_failed_wire_gap(true).await;
}
#[tokio::test]
async fn callback_failure_on_empty_wire_group_preserves_ack_pair_and_poisons_reader() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    let target = temp.path().join("target");
    successful_source(&source).await;
    project(&source, &target, true);
    let mut reader = ProjectionReader::open(&target, hls_limits()).unwrap();
    let mut consumer = SharedConsumer::open(&target).unwrap();
    let mut tested = false;
    while let Some(group) = reader.read_next().unwrap() {
        if matches!(
            group.source,
            ProjectionSource::Owner {
                event: ProjectionOwnerEvent::Wire(_),
                ..
            }
        ) {
            let before = (
                consumer.cursor().clone(),
                consumer.source_cursor().cloned(),
                consumer.as_of_ms(),
            );
            assert!(
                consumer
                    .consume(1, |updates| {
                        assert!(updates.is_empty());
                        Err::<(), _>(HlsError::Parse("declared callback refusal".into()))
                    })
                    .is_err()
            );
            assert_eq!(
                (
                    consumer.cursor().clone(),
                    consumer.source_cursor().cloned(),
                    consumer.as_of_ms()
                ),
                before
            );
            assert!(consumer.poll(1).is_err());
            tested = true;
            break;
        }
        consumer.poll(1).unwrap();
    }
    assert!(tested);
}
