use hls_core::market_state::*;
use hls_hyperliquid::shared::{SharedConsumer, SharedUpdate, parse_shared_ws_message};
use marketfeed_daemon::causal_bootstrap::projection::compare_with_source;
use marketfeed_engine::causal::projection::ProjectionReader;
#[path = "support/mfop.rs"]
mod mfop;

fn consume(consumer: &mut SharedConsumer, state: &mut LiveMarketState) -> Vec<MarketEvent> {
    let mut events = Vec::new();
    loop {
        for update in consumer.poll(4096).unwrap() {
            match update {
                SharedUpdate::Symbols(symbols) => state.include_symbols(symbols).unwrap(),
                SharedUpdate::Events(rows) => {
                    for row in rows {
                        state.apply(row.clone()).unwrap();
                        events.push(row);
                    }
                }
                _ => {}
            }
        }
        if consumer.caught_up() {
            break;
        }
    }
    events
}

fn qualify(event: &mut MarketEvent, key: &str) {
    match event {
        MarketEvent::Trade(v) => v.hl_coin = key.into(),
        MarketEvent::TopOfBook(v) => v.hl_coin = key.into(),
        MarketEvent::OrderBook(v) => v.hl_coin = key.into(),
        MarketEvent::AssetContext(v) => {
            v.hl_coin = key.into();
            v.native_context = Some(NativeContextFields {
                mid_price: Some(8.0),
                circulating_supply: Some(0.0),
                funding: Some(-0.000125),
                open_interest: Some(200.25),
                oracle_price: Some(10.2),
                day_base_volume: Some(0.0),
                premium: Some(-0.25),
                impact_prices: Some([10.0, 11.0]),
            });
        }
        MarketEvent::Candle(v) => v.hl_coin = key.into(),
        MarketEvent::AllMids(_) => panic!("all-mids not admitted by this producer"),
    }
}

#[test]
fn actual_native_spot_perps_projected_values_features_and_restart_match_reference() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    let target = temp.path().join("target");
    let wire = mfop::record(&source, false);
    mfop::project(&source, &target);
    assert!(
        compare_with_source(
            mfop::source(&source),
            ProjectionReader::open(&target, mfop::limits()).unwrap()
        )
        .unwrap()
        .sealed
    );
    let mut expected = Vec::new();
    for (key, raw, received) in wire {
        for mut event in parse_shared_ws_message(&raw).unwrap() {
            qualify(&mut event, &key);
            expected.push(event.with_recv_ts_ns(received.try_into().unwrap()));
        }
    }
    assert_eq!(expected.len(), 14);
    let mut consumer = SharedConsumer::open(&target).unwrap();
    let mut actual = Vec::new();
    let mut state = LiveMarketState::new([]);
    let mut pre_seal_clock = None;
    while !consumer.caught_up() {
        let before = consumer.as_of_ms();
        for update in consumer.poll(1).unwrap() {
            match update {
                SharedUpdate::Symbols(symbols) => state.include_symbols(symbols).unwrap(),
                SharedUpdate::Events(events) => {
                    for event in events {
                        state.apply(event.clone()).unwrap();
                        actual.push(event)
                    }
                }
                SharedUpdate::Sealed => {
                    pre_seal_clock = before;
                    assert_eq!(
                        consumer.as_of_ms(),
                        before,
                        "storage seal must not advance feature time"
                    );
                }
                _ => {}
            }
        }
    }
    assert!(consumer.sealed());
    assert!(pre_seal_clock.is_some());
    assert_eq!(actual, expected);
    assert_eq!(consumer.symbols(), ["perps:BTC", "spot:@12"]);
    assert_eq!(consumer.instruments().len(), 2);
    assert_eq!(consumer.metadata(31).unwrap().unwrap().capture.venue, 21);
    assert_eq!(consumer.metadata(32).unwrap().unwrap().capture.venue, 22);
    for identity in consumer.instruments() {
        assert_eq!(identity.instrument, identity.metadata.id);
        assert_eq!(identity.native_symbol, identity.metadata.key.native_symbol);
    }
    let mut reference = LiveMarketState::new(consumer.symbols());
    for event in expected {
        reference.apply(event).unwrap();
    }
    for symbol in consumer.symbols() {
        assert_eq!(state.symbol_state(&symbol), reference.symbol_state(&symbol));
        let s = state.symbol_state(&symbol).unwrap();
        assert!(s.bid_px.is_none());
        assert!(s.mid_px.is_none());
        assert_eq!(s.ask_px, Some(11.0));
        assert_eq!(
            s.native_context.as_ref().unwrap().day_base_volume,
            Some(0.0)
        );
    }
    let mut restarted = SharedConsumer::open(&target).unwrap();
    let mut replay = Vec::new();
    while !restarted.caught_up() {
        for u in restarted.poll(4096).unwrap() {
            if let SharedUpdate::Events(events) = u {
                replay.extend(events)
            }
        }
    }
    assert_eq!(replay, actual);
    assert_eq!(restarted.cursor(), consumer.cursor());
    assert_eq!(restarted.source_cursor(), consumer.source_cursor());
    assert_eq!(restarted.as_of_ms(), consumer.as_of_ms());
}

#[test]
fn actual_negative_receipt_fails_whole_group_without_advancing_visible_cursor_and_poisons() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    let target = temp.path().join("target");
    mfop::record(&source, true);
    mfop::project(&source, &target);
    let mut consumer = SharedConsumer::open(&target).unwrap();
    loop {
        let before = consumer.cursor().clone();
        let source_before = consumer.source_cursor().cloned();
        match consumer.poll(1) {
            Ok(_) => assert!(!consumer.caught_up(), "negative receipt must be rejected"),
            Err(_) => {
                assert_eq!(consumer.cursor(), &before);
                assert_eq!(consumer.source_cursor(), source_before.as_ref());
                break;
            }
        }
    }
    assert!(consumer.poll(1).is_err());
    assert!(consumer.metadata(31).is_err());
    assert!(consumer.symbols().is_empty());
}

#[test]
fn failed_multi_group_batch_does_not_deliver_updates_or_acknowledge_prior_good_groups() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    let target = temp.path().join("target");
    mfop::record(&source, true);
    mfop::project(&source, &target);
    let mut consumer = SharedConsumer::open(&target).unwrap();
    let before = consumer.cursor().clone();
    let mut delivered = false;
    let result = consumer.consume(4096, |_| {
        delivered = true;
        Ok(())
    });
    assert!(result.is_err());
    assert!(!delivered, "decoding failure must precede application");
    assert_eq!(consumer.cursor(), &before);
    assert!(consumer.source_cursor().is_none());
    assert!(consumer.as_of_ms().is_none());
    assert!(!consumer.sealed());
    assert!(consumer.instruments().is_empty());
    assert!(consumer.metadata(31).is_err());
    assert!(consumer.poll(1).is_err());
}

#[test]
fn application_failure_preserves_acknowledged_pair_and_rebuild_applies_before_acknowledgment() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    let target = temp.path().join("target");
    mfop::record(&source, false);
    mfop::project(&source, &target);
    let mut consumer = SharedConsumer::open(&target).unwrap();
    let before = consumer.cursor().clone();
    let mut delivered = 0;
    let result: hls_core::error::HlsResult<()> = consumer.consume(4096, |updates| {
        delivered = updates
            .iter()
            .filter_map(|u| match u {
                SharedUpdate::Events(rows) => Some(rows.len()),
                _ => None,
            })
            .sum::<usize>();
        Err(hls_core::error::HlsError::External(
            "application rejected batch".into(),
        ))
    });
    assert!(result.is_err());
    assert_eq!(delivered, 14);
    assert_eq!(consumer.cursor(), &before);
    assert!(consumer.source_cursor().is_none());
    assert!(consumer.as_of_ms().is_none());
    assert!(!consumer.sealed());
    assert!(consumer.poll(1).is_err());

    // The caller discards failed application state and rebuilds from the header.
    let mut rebuilt = SharedConsumer::open(&target).unwrap();
    let mut state = LiveMarketState::new([]);
    let applied = rebuilt
        .consume(4096, |updates| {
            let mut applied = 0;
            for update in updates {
                match update {
                    SharedUpdate::Symbols(symbols) => state.include_symbols(symbols)?,
                    SharedUpdate::Events(rows) => {
                        for row in rows {
                            state.apply(row)?;
                            applied += 1;
                        }
                    }
                    _ => {}
                }
            }
            Ok(applied)
        })
        .unwrap();
    assert_eq!(applied, 14);
    assert!(
        state
            .symbol_state("spot:@12")
            .unwrap()
            .native_context
            .is_some()
    );
    assert_ne!(rebuilt.cursor(), &before);
    assert!(rebuilt.source_cursor().is_some());
    assert!(rebuilt.sealed());
}

#[test]
fn caught_application_panic_preserves_acknowledged_pair_and_refuses_reuse() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    let target = temp.path().join("target");
    mfop::record(&source, false);
    mfop::project(&source, &target);
    let mut consumer = SharedConsumer::open(&target).unwrap();
    let before = consumer.cursor().clone();
    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _: hls_core::error::HlsResult<()> = consumer.consume(4096, |updates| {
            assert!(updates.iter().any(|u| matches!(u, SharedUpdate::Events(_))));
            panic!("application panic before acknowledgment")
        });
    }));
    assert!(panic.is_err());
    assert_eq!(consumer.cursor(), &before);
    assert!(consumer.source_cursor().is_none());
    assert!(consumer.as_of_ms().is_none());
    assert!(!consumer.sealed());
    assert!(!consumer.caught_up());
    assert!(consumer.symbols().is_empty());
    assert!(consumer.instruments().is_empty());
    assert!(consumer.metadata(31).is_err());
    assert!(consumer.poll(1).is_err());
}

#[test]
fn every_committed_live_prefix_matches_rebuilt_feature_state_without_future_metadata() {
    use marketfeed_recording::shared_feed::{FeedProfile, FeedRecord, FeedWriter};
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    let target = temp.path().join("target");
    mfop::record(&source, false);
    mfop::project(&source, &target);
    let live = temp.path().join("live");
    let mut writer =
        FeedWriter::create_profile(&live, 64 << 20, FeedProfile::ObservationProjectionV1).unwrap();
    let mut reader = ProjectionReader::open(&target, mfop::limits()).unwrap();
    let mut consumer = SharedConsumer::open(&live).unwrap();
    let mut state = LiveMarketState::new([]);
    let mut emitted = 0;
    consume(&mut consumer, &mut state);
    assert!(consumer.symbols().is_empty());
    while let Some(group) = reader.read_next().unwrap() {
        let committed = writer
            .append(FeedRecord::Projection(group.encode().unwrap()))
            .unwrap();
        emitted += consume(&mut consumer, &mut state).len();
        assert_eq!(consumer.cursor(), &committed);
        assert_eq!(consumer.source_cursor(), Some(&group.source_after));
        let mut rebuilt = SharedConsumer::open(&live).unwrap();
        let mut rebuilt_state = LiveMarketState::new([]);
        consume(&mut rebuilt, &mut rebuilt_state);
        assert_eq!(rebuilt.symbols(), consumer.symbols());
        assert_eq!(rebuilt.instruments(), consumer.instruments());
        assert_eq!(rebuilt.as_of_ms(), consumer.as_of_ms());
        for key in consumer.symbols() {
            assert_eq!(state.symbol_state(&key), rebuilt_state.symbol_state(&key));
        }
    }
    assert_eq!(emitted, 14);
    assert!(consumer.sealed());
}

#[test]
fn accepted_trade_without_committed_observed_handoff_is_never_resurrected() {
    use marketfeed_adapter_api::NativeObservation;
    use marketfeed_engine::causal::*;
    use marketfeed_recording::shared_feed::{
        CausalPhase, FeedProfile, FeedReader, FeedRecord, FeedWriter,
    };
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    mfop::record(&source, false);
    let mut reader = FeedReader::open_profile(&source, None, FeedProfile::EngineCausalV4).unwrap();
    let mut records = Vec::new();
    while let Some(r) = reader.read_next().unwrap() {
        records.push(r)
    }
    let FeedRecord::Causal(first) = &records[0] else {
        panic!()
    };
    let identity = OperationIdentity {
        venue: first.route.venue,
        session: first.route.session,
        bootstrap_id: first.route.bootstrap_id.clone(),
    };
    let trade_id=records.iter().find_map(|r| {
        let FeedRecord::Causal(e)=r else{return None};if e.phase==CausalPhase::Bootstrap||e.route.session!=identity.session{return None}
        match decode_owner_history(e,&identity).unwrap().event {
            OwnerEventDto::ExecutionCompleted{input_id,operation,..} if operation.entries.iter().any(|e|matches!(&e.effect,EffectDto::QueueDrain{items,..} if items.iter().any(|i|matches!(i,ItemDto::ObservedMarket(leaf) if leaf.batch().unwrap().native().rows.iter().any(|r|matches!(r.observation,NativeObservation::HyperliquidTrade(_)))))))=>Some(input_id),_=>None
        }
    }).unwrap();
    for intent in [false, true] {
        let end = records
            .iter()
            .position(|r| {
                let FeedRecord::Causal(e) = r else {
                    return false;
                };
                if e.phase == CausalPhase::Bootstrap || e.route.session != identity.session {
                    return false;
                }
                match decode_owner_history(e, &identity).unwrap().event {
                    OwnerEventDto::ExecutionPermitted { input_id, .. } => {
                        !intent && input_id == trade_id
                    }
                    OwnerEventDto::ReleaseIntent { input_id, .. } => intent && input_id == trade_id,
                    _ => false,
                }
            })
            .unwrap();
        let prefix = temp.path().join(format!("prefix-{intent}"));
        let mut cut = records[..=end].to_vec();
        cut.push(FeedRecord::Seal {
            schema_version: 1,
            now_ns: i64::MAX,
            reason: "unknown trade execution/handoff".into(),
        });
        FeedWriter::create_profile(&prefix, 64 << 20, FeedProfile::EngineCausalV4)
            .unwrap()
            .append_many(cut)
            .unwrap();
        let target = temp.path().join(format!("target-{intent}"));
        mfop::project(&prefix, &target);
        let mut consumer = SharedConsumer::open(&target).unwrap();
        let mut gaps = 0;
        while !consumer.caught_up() {
            for u in consumer.poll(1).unwrap() {
                match u {
                    SharedUpdate::Events(rows) => {
                        assert!(rows.is_empty(), "unobserved trade resurrected")
                    }
                    SharedUpdate::Gap { .. } => gaps += 1,
                    _ => {}
                }
            }
        }
        assert!(consumer.sealed());
        assert!(gaps > 0);
    }
}
