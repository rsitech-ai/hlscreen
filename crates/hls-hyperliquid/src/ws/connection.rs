use hls_core::health::{ConnectionHealth, ConnectionState};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReconnectPolicy {
    pub initial_backoff_ms: u64,
    pub max_backoff_ms: u64,
    pub multiplier: u64,
}

impl ReconnectPolicy {
    pub fn backoff_ms(&self, attempt: u64) -> u64 {
        let multiplier = self.multiplier.max(1);
        let mut backoff = self.initial_backoff_ms;

        for _ in 0..attempt {
            backoff = backoff.saturating_mul(multiplier);
            if backoff >= self.max_backoff_ms {
                return self.max_backoff_ms;
            }
        }

        backoff.min(self.max_backoff_ms)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HeartbeatAction {
    None,
    SendPing,
    Reconnect {
        backoff_ms: u64,
        gap_started_at_ms: i64,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MockWsServer {
    subscriptions: Vec<String>,
}

impl MockWsServer {
    pub fn new<I, S>(subscriptions: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            subscriptions: subscriptions.into_iter().map(Into::into).collect(),
        }
    }

    pub fn subscriptions(&self) -> &[String] {
        &self.subscriptions
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConnectionHealthMachine {
    policy: ReconnectPolicy,
    heartbeat_after_ms: u64,
    reconnect_after_ms: u64,
    reconnect_attempt: u64,
    subscriptions: Vec<String>,
    health: ConnectionHealth,
}

impl ConnectionHealthMachine {
    pub fn new(policy: ReconnectPolicy) -> Self {
        Self {
            policy,
            heartbeat_after_ms: 30_000,
            reconnect_after_ms: 60_000,
            reconnect_attempt: 0,
            subscriptions: Vec::new(),
            health: ConnectionHealth {
                state: ConnectionState::Disconnected,
                connected_at_ms: None,
                last_message_at_ms: None,
                reconnect_count: 0,
                last_reconnect_backoff_ms: None,
                gap_count: 0,
            },
        }
    }

    pub fn connect(&mut self, now_ms: i64, subscriptions: Vec<String>) {
        self.subscriptions = subscriptions;
        self.health.state = ConnectionState::Connected;
        self.health.connected_at_ms = Some(now_ms);
        self.health.last_message_at_ms = Some(now_ms);
    }

    pub fn tick(&mut self, now_ms: i64) -> HeartbeatAction {
        let Some(last_message_at_ms) = self.health.last_message_at_ms else {
            return HeartbeatAction::None;
        };
        let age_ms = now_ms.saturating_sub(last_message_at_ms).max(0) as u64;

        if age_ms >= self.reconnect_after_ms {
            if self.health.state == ConnectionState::Reconnecting {
                return HeartbeatAction::None;
            }

            let backoff_ms = self.policy.backoff_ms(self.reconnect_attempt);
            self.reconnect_attempt = self.reconnect_attempt.saturating_add(1);
            self.health.state = ConnectionState::Reconnecting;
            self.health.reconnect_count = self.health.reconnect_count.saturating_add(1);
            self.health.last_reconnect_backoff_ms = Some(backoff_ms);
            self.health.gap_count = self.health.gap_count.saturating_add(1);

            return HeartbeatAction::Reconnect {
                backoff_ms,
                gap_started_at_ms: last_message_at_ms,
            };
        }

        if age_ms >= self.heartbeat_after_ms && self.health.state == ConnectionState::Connected {
            self.health.state = ConnectionState::PingSent;
            return HeartbeatAction::SendPing;
        }

        HeartbeatAction::None
    }

    pub fn mark_reconnected(&mut self, now_ms: i64) -> Vec<String> {
        self.health.state = ConnectionState::Connected;
        self.health.connected_at_ms = Some(now_ms);
        self.health.last_message_at_ms = Some(now_ms);
        self.reconnect_attempt = 0;
        self.subscriptions.clone()
    }

    pub fn health(&self) -> &ConnectionHealth {
        &self.health
    }
}
