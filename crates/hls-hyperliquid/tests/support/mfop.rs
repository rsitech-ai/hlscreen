// Bootstrap fixture adapted from the immutable producer e812e44 public API.
// Real native factory, durable owner, verified replay and MFOP1 pump; no network.
use marketfeed_adapter_api::*;
use marketfeed_adapter_hyperliquid::HyperliquidFactory;
use marketfeed_daemon::causal_bootstrap::{
    PreparedBootstrap, current_build,
    owner::{InputSettlement, OwnerPhase},
    projection::*,
    replay::*,
};
use marketfeed_engine::{CaptureConfig, CaptureInput, causal::projection::*, causal::*};
use marketfeed_model::*;
use marketfeed_recording::{shared_batch::*, shared_feed::*};
use std::{
    path::Path,
    sync::OnceLock,
    thread,
    time::{Duration, Instant},
};
pub(super) const SPOT: &[u8] = br#"{"tokens":[{"name":"USDC","szDecimals":8,"weiDecimals":8,"index":0,"tokenId":"0x0"},{"name":"PURR","szDecimals":0,"weiDecimals":5,"index":7,"tokenId":"0x7"}],"universe":[{"name":"@12","tokens":[7,0],"index":12}]}"#;
const PERPS: &[u8] = br#"[{"collateralToken":0,"universe":[{"name":"BTC","szDecimals":5}]}]"#;
pub(super) fn fixture(spot: bool) -> BootstrapDto {
    fixture_with_meta(spot, SPOT, PERPS, &[1])
}
pub(super) fn fixture_with_meta(
    spot: bool,
    spot_meta: &[u8],
    perp_meta: &[u8],
    selected: &[u32],
) -> BootstrapDto {
    static BUILD: OnceLock<BootstrapBuildDto> = OnceLock::new();
    let factory = HyperliquidFactory { spot };
    let mut definitions = Vec::new();
    factory
        .parse_instruments(
            &[spot_meta, perp_meta].map(|bytes| HttpResponse {
                status: 200,
                headers: vec![],
                body: bytes.to_vec().into(),
            }),
            &mut definitions,
        )
        .unwrap();
    let catalog = CatalogView::with_instruments(
        factory.specification().id,
        CatalogVersion(7),
        definitions
            .into_iter()
            .enumerate()
            .map(|(i, d)| d.into_instrument(InstrumentId(i as u32 + 1), CatalogVersion(7)))
            .collect::<Vec<_>>(),
    );
    let mut channels = vec![
        Channel::Trades,
        Channel::Quote,
        Channel::L2Book {
            depth: Some(20),
            cadence: None,
        },
        Channel::Candles {
            interval: CandleInterval::M1,
        },
        Channel::MarkPrice,
        Channel::Statistics24h,
    ];
    if !spot {
        channels.extend([Channel::IndexPrice, Channel::Funding, Channel::OpenInterest]);
    }
    let request = ConcreteSubscriptionSet {
        items: channels
            .iter()
            .flat_map(|channel| {
                selected.iter().map(move |instrument| ConcreteSubscription {
                    instrument: InstrumentId(*instrument),
                    channel: channel.clone(),
                    delivery: DeliveryOptions {
                        emit_book_snapshots: true,
                        emit_book_deltas: false,
                        emit_bbo: true,
                    },
                })
            })
            .collect(),
    };
    let mut plans = factory.plan(&request, &catalog).unwrap();
    assert_eq!(plans.len(), 1);
    let spec = plans.remove(0);
    let capture = CaptureConfig {
        venue: catalog.venue,
        session: SessionId(31),
        dispatch_capacity: 64,
        overflow: OverflowPolicy::FailEngine,
        max_bytes: 1 << 20,
        max_entries: 2048,
    };
    let mut dto = BootstrapDto {
        schema_version: 2,
        build: BUILD.get_or_init(|| current_build().unwrap()).clone(),
        factory: FactoryDto::HyperliquidPublic {
            market: if spot {
                HyperliquidMarketDto::Spot
            } else {
                HyperliquidMarketDto::Perps
            },
        },
        environment: BootstrapEnvironmentDto::Production,
        output_profile: OutputProfileDto::DiskNullDispatchV1,
        adapter_policy: AdapterEmissionPolicyDto::HyperliquidSelectedChannelsV1,
        protocol_topics: HyperliquidFactory::public_plan_topics(spot, &spec, &catalog).unwrap(),
        session: (&spec).into(),
        catalog: (&catalog).into(),
        capture: capture.into(),
        runner_policy: capture.runtime_policy(),
        owner_policy: Some(JournalOwnerPolicyDto {
            max_pending_operations: 16,
            max_pending_bytes: 2 << 20,
            max_input_bytes: 8192,
            max_operation_bytes: 256 << 10,
            max_ingress_age: Duration::from_secs(10).into(),
            receipt_poll_interval: Duration::from_millis(1).into(),
        }),
        public_inputs: vec![
            PublicReproductionInputDto::new(HYPERLIQUID_SPOT_META_ROLE, spot_meta),
            PublicReproductionInputDto::new(HYPERLIQUID_PERP_METAS_ROLE, perp_meta),
        ],
    };
    dto.bind_history_policy(HistoryRuntimePolicyDto {
        schema_version: 1,
        max_pending_observations: 32,
        max_pending_observation_bytes: 4 << 20,
        max_record_bytes: 256 << 10,
        max_finalization_age: Duration::from_secs(2).into(),
    })
    .unwrap();
    dto
}

pub(super) fn wait(mut predicate: impl FnMut() -> bool) {
    let until = Instant::now() + Duration::from_secs(15);
    while !predicate() {
        assert!(Instant::now() < until, "actual owner/pump watchdog");
        thread::sleep(Duration::from_millis(1));
    }
}
pub(super) fn batch() -> BatchConfig {
    BatchConfig {
        max_pending_operations: 32,
        max_pending_bytes: 8 << 20,
        max_group_operations: 32,
        max_group_bytes: 8 << 20,
        max_age: Duration::from_millis(1),
    }
}
pub fn limits() -> ProjectionLimits {
    ProjectionLimits {
        max_group_bytes: 1 << 20,
        max_items: 1024,
        max_owners: 8,
        max_bootstrap_bytes: 8 << 20,
        max_catalog_entries: 128,
        max_pending_inputs: 128,
    }
}
pub fn source(path: &Path) -> MultiHistoryReplay {
    source_with_record_limit(path, 256 << 10)
}
pub(super) fn source_with_record_limit(path: &Path, max_record_bytes: usize) -> MultiHistoryReplay {
    source_profile_with_record_limit(path, max_record_bytes, FeedProfile::EngineCausalV4)
}
// Some targets include this helper module only for its MFCJ4 fixtures.
#[allow(dead_code)]
pub(super) fn source_profile(path: &Path, profile: FeedProfile) -> MultiHistoryReplay {
    source_profile_with_record_limit(path, 256 << 10, profile)
}
fn source_profile_with_record_limit(
    path: &Path,
    max_record_bytes: usize,
    profile: FeedProfile,
) -> MultiHistoryReplay {
    MultiHistoryReplay::open_profile(
        path,
        MultiReplayLimits {
            per_owner: OwnerHistoryLimits {
                max_retained_input_bytes: 4 << 20,
                max_input_bytes: 256 << 10,
                max_record_bytes,
            },
            maximum: ReplayResources {
                owners: 8,
                bootstrap_payload_bytes: 8 << 20,
                pending_inputs: 128,
                pending_input_bytes: 8 << 20,
                active_operation_and_state_bytes: 8 << 20,
                source_owned_pending_inputs: 128,
                source_owned_pending_bytes: 8 << 20,
                capture_trace_reservation_bytes: 16 << 20,
                catalog_entries: 128,
                selected_instruments: 128,
                maintained_book_levels: 4096,
                cached_book_levels: 32768,
            },
        },
        profile,
    )
    .unwrap()
}
pub fn project(source_path: &Path, target: &Path) {
    project_source(source(source_path), target)
}
pub(super) fn project_source(source: MultiHistoryReplay, target: &Path) {
    let mut pump = ProjectionPump::create(
        source,
        ProjectionPolicy {
            schema_version: 1,
            source_profile: 4,
            clock_policy: ProjectionClockPolicy::PerOwnerStartedPhysicalOrderV1,
            limits: limits(),
        },
        target,
        64 << 20,
        batch(),
    )
    .unwrap();
    wait(|| {
        pump.poll().unwrap();
        pump.last_acknowledged().sealed
    });
    pump.shutdown().unwrap();
}
/// Native wire references retained only in this test helper for independent
/// old-conversion parity. The consumer never reads them.
pub fn record(path: &Path, negative: bool) -> Vec<(String, String, i64)> {
    let actor = FeedBatchActor::spawn(
        FeedWriter::create_profile(path, 64 << 20, FeedProfile::EngineCausalV4).unwrap(),
        batch(),
    )
    .unwrap();
    let mut expected = Vec::new();
    for spot in [true, false] {
        let mut dto = fixture(spot);
        dto.capture.session = if spot { 31 } else { 32 };
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
        let coin = dto.catalog.instruments[0].key.native_symbol.clone();
        let key = format!("{}:{coin}", if spot { "spot" } else { "perps" });
        let topics = dto.protocol_topics.clone();
        let owner = PreparedBootstrap::new_observed_history(dto, MAX_OPERATION_BYTES)
            .unwrap()
            .try_admit(&actor.handle())
            .unwrap()
            .wait()
            .unwrap()
            .instantiate()
            .unwrap()
            .spawn_history_owner()
            .unwrap();
        let submit = |input| {
            assert!(matches!(
                owner
                    .ingress()
                    .try_submit(ConnectionId(1), input)
                    .unwrap()
                    .wait(),
                InputSettlement::Committed { .. }
            ));
            let mut release = None;
            wait(|| {
                release = owner.try_next();
                release.is_some()
            });
        };
        submit(CaptureInput::Connected {
            connection: ConnectionId(1),
            now: TimestampNs(1_710_000_000_000_000_000),
        });
        for topic in topics {
            submit(CaptureInput::Text{payload:format!(r#"{{"channel":"subscriptionResponse","data":{{"method":"subscribe","subscription":{topic}}}}}"#).into_bytes(),received:FrameStamp{receive_ts:TimestampNs(1_710_000_000_000_000_001),mono_ns:1}});
        }
        let rows = [
            format!(
                r#"{{"channel":"trades","data":[{{"coin":"{coin}","side":"B","px":"10.25","sz":"2","hash":"h1","time":1710000000001,"tid":1}},{{"coin":"{coin}","side":"A","px":"10.5","sz":"3","hash":"h2","time":1710000000002,"tid":2}}]}}"#
            ),
            format!(
                r#"{{"channel":"bbo","data":{{"coin":"{coin}","time":1710000000003,"bbo":[{{"px":"10","sz":"2","n":18446744073709551615}},{{"px":"11","sz":"3","n":0}}]}}}}"#
            ),
            format!(
                r#"{{"channel":"l2Book","data":{{"coin":"{coin}","time":1710000000004,"levels":[[{{"px":"10","sz":"2","n":3}}],[{{"px":"11","sz":"3","n":4}}]]}}}}"#
            ),
            format!(
                r#"{{"channel":"candle","data":{{"t":1710000000000,"T":1710000000017,"s":"{coin}","i":"1m","o":"10","h":"11","l":"9","c":"10.5","v":"23.5","n":18446744073709551615}}}}"#
            ),
            format!(
                r#"{{"channel":"activeAssetCtx","data":{{"coin":"{coin}","ctx":{{"dayNtlVlm":"100.25","prevDayPx":"9","markPx":"10.1","midPx":"8","circulatingSupply":"0","funding":"-0.000125","openInterest":"200.25","oraclePx":"10.2","dayBaseVlm":"0","premium":"-0.25","impactPxs":["10","11"]}}}}}}"#
            ),
            format!(
                r#"{{"channel":"bbo","data":{{"coin":"{coin}","time":1710000000005,"bbo":[null,{{"px":"11","sz":"3","n":18446744073709551615}}]}}}}"#
            ),
        ];
        for (index, raw) in rows.into_iter().enumerate() {
            let received = if negative && index == 5 {
                -1
            } else {
                1_710_000_000_001_000_000 + index as i64
            };
            submit(CaptureInput::Text {
                payload: raw.as_bytes().to_vec(),
                received: FrameStamp {
                    receive_ts: TimestampNs(received),
                    mono_ns: index as u64 + 2,
                },
            });
            expected.push((key.clone(), raw, received));
        }
        owner.close_ingress();
        wait(|| owner.status().owner.phase == OwnerPhase::Stopped);
        assert!(owner.shutdown().owner.failure.is_none());
    }
    actor
        .handle()
        .try_submit(
            PreparedFeedTransaction::new(vec![FeedRecord::Seal {
                schema_version: 1,
                now_ns: i64::MAX,
                reason: "closed offline actual fixture".into(),
            }])
            .unwrap(),
        )
        .unwrap()
        .wait()
        .unwrap();
    actor.shutdown().unwrap();
    expected
}
