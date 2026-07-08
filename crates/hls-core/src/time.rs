use std::time::{Duration, SystemTime, UNIX_EPOCH};

use time::{
    OffsetDateTime, PrimitiveDateTime, format_description, format_description::well_known::Rfc3339,
};

use crate::error::{HlsError, HlsResult};

pub fn duration_to_millis(duration: Duration) -> u64 {
    match u64::try_from(duration.as_millis()) {
        Ok(value) => value,
        Err(_) => u64::MAX,
    }
}

pub fn now_millis() -> HlsResult<u128> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| HlsError::Time(format!("system clock is before UNIX epoch: {err}")))?
        .as_millis())
}

pub fn parse_rfc3339_millis(input: &str) -> HlsResult<i128> {
    let parsed = OffsetDateTime::parse(input, &Rfc3339)
        .map_err(|err| HlsError::Time(format!("invalid RFC3339 timestamp '{input}': {err}")))?;

    Ok(parsed.unix_timestamp_nanos() / 1_000_000)
}

pub fn parse_utc_datetime_millis(input: &str) -> HlsResult<i128> {
    let format = format_description::parse_borrowed::<3>(
        "[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond]",
    )
    .map_err(|err| HlsError::Time(format!("invalid timestamp format description: {err}")))?;
    let parsed = PrimitiveDateTime::parse(input, &format).map_err(|err| {
        HlsError::Time(format!(
            "invalid UTC timestamp without timezone '{input}': {err}"
        ))
    })?;

    Ok(parsed.assume_utc().unix_timestamp_nanos() / 1_000_000)
}
