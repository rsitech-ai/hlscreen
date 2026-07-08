use hls_hyperliquid::ws::connection::{
    ConnectionHealthMachine, HeartbeatAction, MockWsServer, ReconnectPolicy,
};

#[test]
fn heartbeat_ping_reconnect_and_resubscribe_flow_is_deterministic() {
    let mock_server = MockWsServer::new(vec!["trades:@107", "bbo:@107"]);
    let mut machine = ConnectionHealthMachine::new(ReconnectPolicy {
        initial_backoff_ms: 1_000,
        max_backoff_ms: 8_000,
        multiplier: 2,
    });

    machine.connect(0, mock_server.subscriptions().to_vec());
    assert_eq!(machine.tick(29_999), HeartbeatAction::None);
    assert_eq!(machine.tick(30_000), HeartbeatAction::SendPing);
    assert_eq!(
        machine.tick(60_000),
        HeartbeatAction::Reconnect {
            backoff_ms: 1_000,
            gap_started_at_ms: 0,
        }
    );

    let resubscribe = machine.mark_reconnected(61_000);
    assert_eq!(resubscribe, vec!["trades:@107", "bbo:@107"]);
    assert_eq!(machine.health().reconnect_count, 1);
    assert_eq!(machine.health().last_reconnect_backoff_ms, Some(1_000));
    assert_eq!(machine.health().gap_count, 1);
}

#[test]
fn reconnect_backoff_is_bounded() {
    let policy = ReconnectPolicy {
        initial_backoff_ms: 1_000,
        max_backoff_ms: 5_000,
        multiplier: 3,
    };

    assert_eq!(policy.backoff_ms(0), 1_000);
    assert_eq!(policy.backoff_ms(1), 3_000);
    assert_eq!(policy.backoff_ms(2), 5_000);
    assert_eq!(policy.backoff_ms(10), 5_000);
}
