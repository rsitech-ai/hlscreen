use anyhow::Context;
use hls_core::health::{ConnectionHealth, ConnectionState, HealthInputs, HealthSnapshot};

pub fn simulated_health(name: Option<&str>) -> anyhow::Result<HealthSnapshot> {
    match name.unwrap_or("healthy") {
        "healthy" => Ok(HealthInputs::healthy_fixture().snapshot()),
        "writer-lag" => Ok(HealthInputs::writer_lag_fixture().snapshot()),
        "interrupted" => Ok(HealthInputs::interrupted_fixture().snapshot()),
        other => anyhow::bail!("unknown simulated health state '{other}'"),
    }
}

pub fn rest_health(live_rest_ok: bool) -> HealthSnapshot {
    if live_rest_ok {
        return HealthInputs::healthy_fixture().snapshot();
    }

    HealthInputs {
        connection: ConnectionHealth {
            state: ConnectionState::Disconnected,
            reconnect_count: 1,
            gap_count: 1,
            ..ConnectionHealth::connected(1_000, 75_000)
        },
        last_message_age_ms: Some(75_000),
        gap_count: 1,
        ..HealthInputs::healthy_fixture()
    }
    .snapshot()
}

pub fn require_live_health(
    live: bool,
    simulate_health: Option<&str>,
    live_rest_ok: Option<bool>,
) -> anyhow::Result<Option<HealthSnapshot>> {
    if let Some(name) = simulate_health {
        return simulated_health(Some(name)).map(Some);
    }

    if live {
        let live_rest_ok = live_rest_ok.context("live REST status missing for live health")?;
        return Ok(Some(rest_health(live_rest_ok)));
    }

    Ok(None)
}
