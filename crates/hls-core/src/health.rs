use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Interrupted,
}

impl HealthStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Healthy => "healthy",
            Self::Degraded => "degraded",
            Self::Interrupted => "interrupted",
        }
    }

    fn severity(self) -> u8 {
        match self {
            Self::Healthy => 0,
            Self::Degraded => 1,
            Self::Interrupted => 2,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Stale,
    PingSent,
    Reconnecting,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReadOnlySafety {
    pub read_only: bool,
    pub wallet_enabled: bool,
    pub trading_enabled: bool,
}

impl ReadOnlySafety {
    pub fn read_only() -> Self {
        Self {
            read_only: true,
            wallet_enabled: false,
            trading_enabled: false,
        }
    }

    pub fn is_ok(&self) -> bool {
        self.read_only && !self.wallet_enabled && !self.trading_enabled
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ConnectionHealth {
    pub state: ConnectionState,
    pub connected_at_ms: Option<i64>,
    pub last_message_at_ms: Option<i64>,
    pub reconnect_count: u64,
    pub last_reconnect_backoff_ms: Option<u64>,
    pub gap_count: u64,
}

impl ConnectionHealth {
    pub fn connected(now_ms: i64, last_message_age_ms: u64) -> Self {
        Self {
            state: ConnectionState::Connected,
            connected_at_ms: Some(now_ms),
            last_message_at_ms: Some(
                now_ms
                    .checked_sub(last_message_age_ms as i64)
                    .unwrap_or(0)
                    .max(0),
            ),
            reconnect_count: 0,
            last_reconnect_backoff_ms: None,
            gap_count: 0,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WriterHealth {
    pub backlog: u64,
    pub warn_at: u64,
    pub rows_written: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RecordingHealth {
    pub enabled: bool,
    pub clean_shutdown: Option<bool>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HealthInputs {
    pub safety: ReadOnlySafety,
    pub connection: ConnectionHealth,
    pub subscription_count: u64,
    pub last_message_age_ms: Option<u64>,
    pub lag_ms: Option<u64>,
    pub writer: WriterHealth,
    pub recording: RecordingHealth,
    pub gap_count: u64,
}

impl HealthInputs {
    pub fn healthy_fixture() -> Self {
        Self {
            safety: ReadOnlySafety::read_only(),
            connection: ConnectionHealth::connected(1_000, 500),
            subscription_count: 2,
            last_message_age_ms: Some(500),
            lag_ms: Some(20),
            writer: WriterHealth {
                backlog: 0,
                warn_at: 100,
                rows_written: 0,
            },
            recording: RecordingHealth {
                enabled: false,
                clean_shutdown: None,
            },
            gap_count: 0,
        }
    }

    pub fn writer_lag_fixture() -> Self {
        Self {
            writer: WriterHealth {
                backlog: 250,
                warn_at: 100,
                rows_written: 900,
            },
            ..Self::healthy_fixture()
        }
    }

    pub fn interrupted_fixture() -> Self {
        Self {
            connection: ConnectionHealth {
                state: ConnectionState::Disconnected,
                reconnect_count: 1,
                gap_count: 1,
                ..ConnectionHealth::connected(1_000, 75_000)
            },
            last_message_age_ms: Some(75_000),
            gap_count: 1,
            ..Self::healthy_fixture()
        }
    }

    pub fn snapshot(self) -> HealthSnapshot {
        let mut status = HealthStatus::Healthy;
        let mut degraded_reasons = Vec::new();
        let read_only = self.safety.is_ok();

        if !read_only {
            raise_status(&mut status, HealthStatus::Interrupted);
            degraded_reasons.push("read-only safety violation".to_owned());
        }

        match self.connection.state {
            ConnectionState::Disconnected => {
                raise_status(&mut status, HealthStatus::Interrupted);
                degraded_reasons.push("connection disconnected".to_owned());
            }
            ConnectionState::Reconnecting => {
                raise_status(&mut status, HealthStatus::Degraded);
                degraded_reasons.push("connection reconnecting".to_owned());
            }
            ConnectionState::Stale => {
                raise_status(&mut status, HealthStatus::Degraded);
                degraded_reasons.push("connection stale".to_owned());
            }
            ConnectionState::Connecting
            | ConnectionState::PingSent
            | ConnectionState::Connected => {}
        }

        if self.writer.warn_at > 0 && self.writer.backlog > self.writer.warn_at {
            raise_status(&mut status, HealthStatus::Degraded);
            degraded_reasons.push("writer backlog high".to_owned());
        }

        if self
            .last_message_age_ms
            .is_some_and(|age_ms| age_ms >= 60_000)
        {
            raise_status(&mut status, HealthStatus::Degraded);
            degraded_reasons.push("stale inbound data".to_owned());
        }

        if self.lag_ms.is_some_and(|lag_ms| lag_ms >= 10_000) {
            raise_status(&mut status, HealthStatus::Degraded);
            degraded_reasons.push("data lag high".to_owned());
        }

        let gap_count = self.gap_count.max(self.connection.gap_count);
        if gap_count > 0 {
            raise_status(&mut status, HealthStatus::Degraded);
            degraded_reasons.push("data gaps detected".to_owned());
        }

        HealthSnapshot {
            status,
            read_only,
            connections: vec![self.connection.clone()],
            subscription_count: self.subscription_count,
            last_message_age_ms: self.last_message_age_ms,
            lag_ms: self.lag_ms,
            writer_backlog: self.writer.backlog,
            writer_warn_at: self.writer.warn_at,
            rows_written: self.writer.rows_written,
            recording: self.recording,
            gap_count,
            reconnect_count: self.connection.reconnect_count,
            degraded_reasons,
        }
    }
}

fn raise_status(status: &mut HealthStatus, candidate: HealthStatus) {
    if candidate.severity() > status.severity() {
        *status = candidate;
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct HealthSnapshot {
    pub status: HealthStatus,
    pub read_only: bool,
    pub connections: Vec<ConnectionHealth>,
    pub subscription_count: u64,
    pub last_message_age_ms: Option<u64>,
    pub lag_ms: Option<u64>,
    pub writer_backlog: u64,
    pub writer_warn_at: u64,
    pub rows_written: u64,
    pub recording: RecordingHealth,
    pub gap_count: u64,
    pub reconnect_count: u64,
    pub degraded_reasons: Vec<String>,
}
