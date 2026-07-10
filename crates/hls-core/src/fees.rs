use serde::{Deserialize, Serialize};

use crate::{HlsError, HlsResult};

const MAX_RATE_HUNDREDTHS_BPS: u32 = 1_000_000;
const MAX_RATIO_HUNDREDTHS: u32 = 10_000;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FeeProfile {
    pub name: String,
    pub maker_fee_hundredths_bps: u32,
    pub taker_fee_hundredths_bps: u32,
    #[serde(default = "default_taker_fill_ratio_hundredths")]
    pub taker_fill_ratio_hundredths: u32,
    pub slippage_buffer_hundredths_bps: u32,
    pub max_tradeable_round_trip_hundredths_bps: u32,
    pub max_costly_round_trip_hundredths_bps: u32,
}

impl FeeProfile {
    pub fn new_hundredths_bps(
        name: impl Into<String>,
        maker_fee_hundredths_bps: u32,
        taker_fee_hundredths_bps: u32,
        slippage_buffer_hundredths_bps: u32,
        max_tradeable_round_trip_hundredths_bps: u32,
        max_costly_round_trip_hundredths_bps: u32,
    ) -> HlsResult<Self> {
        let profile = Self {
            name: name.into(),
            maker_fee_hundredths_bps,
            taker_fee_hundredths_bps,
            taker_fill_ratio_hundredths: default_taker_fill_ratio_hundredths(),
            slippage_buffer_hundredths_bps,
            max_tradeable_round_trip_hundredths_bps,
            max_costly_round_trip_hundredths_bps,
        };
        profile.validate()?;
        Ok(profile)
    }

    pub fn with_taker_fill_ratio_hundredths(
        mut self,
        taker_fill_ratio_hundredths: u32,
    ) -> HlsResult<Self> {
        self.taker_fill_ratio_hundredths = taker_fill_ratio_hundredths;
        self.validate()?;
        Ok(self)
    }

    pub fn validate(&self) -> HlsResult<()> {
        validate_name(&self.name)?;
        validate_rate("maker fee", self.maker_fee_hundredths_bps)?;
        validate_rate("taker fee", self.taker_fee_hundredths_bps)?;
        validate_ratio("taker fill ratio", self.taker_fill_ratio_hundredths)?;
        validate_rate("slippage buffer", self.slippage_buffer_hundredths_bps)?;
        validate_rate(
            "max tradeable round-trip threshold",
            self.max_tradeable_round_trip_hundredths_bps,
        )?;
        validate_rate(
            "max costly round-trip threshold",
            self.max_costly_round_trip_hundredths_bps,
        )?;
        if self.max_tradeable_round_trip_hundredths_bps > self.max_costly_round_trip_hundredths_bps
        {
            return Err(HlsError::Config(
                "fee profile thresholds must be ordered tradeable <= costly".to_owned(),
            ));
        }
        Ok(())
    }

    pub fn maker_fee_bps(&self) -> f64 {
        hundredths_bps_to_bps(self.maker_fee_hundredths_bps)
    }

    pub fn taker_fee_bps(&self) -> f64 {
        hundredths_bps_to_bps(self.taker_fee_hundredths_bps)
    }

    pub fn taker_fill_ratio(&self) -> f64 {
        f64::from(self.taker_fill_ratio_hundredths) / f64::from(MAX_RATIO_HUNDREDTHS)
    }

    pub fn slippage_buffer_bps(&self) -> f64 {
        hundredths_bps_to_bps(self.slippage_buffer_hundredths_bps)
    }

    pub fn max_tradeable_round_trip_bps(&self) -> f64 {
        hundredths_bps_to_bps(self.max_tradeable_round_trip_hundredths_bps)
    }

    pub fn max_costly_round_trip_bps(&self) -> f64 {
        hundredths_bps_to_bps(self.max_costly_round_trip_hundredths_bps)
    }

    pub fn round_trip_taker_cost_bps(&self) -> f64 {
        hundredths_bps_to_bps(
            self.taker_fee_hundredths_bps
                .saturating_mul(2)
                .saturating_add(self.slippage_buffer_hundredths_bps),
        )
    }

    pub fn blended_fee_bps(&self) -> f64 {
        let taker_ratio = self.taker_fill_ratio();
        let maker_ratio = 1.0 - taker_ratio;
        self.taker_fee_bps() * taker_ratio + self.maker_fee_bps() * maker_ratio
    }

    pub fn round_trip_blended_cost_bps(&self) -> f64 {
        2.0 * self.blended_fee_bps() + self.slippage_buffer_bps()
    }
}

fn validate_name(name: &str) -> HlsResult<()> {
    if name.trim().is_empty() {
        return Err(HlsError::Config(
            "fee profile name cannot be empty".to_owned(),
        ));
    }
    if !name
        .chars()
        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-' || ch == '_')
    {
        return Err(HlsError::Config(
            "fee profile name must contain lowercase ascii letters, digits, '-' or '_'".to_owned(),
        ));
    }
    Ok(())
}

fn validate_rate(label: &str, rate_hundredths_bps: u32) -> HlsResult<()> {
    if rate_hundredths_bps > MAX_RATE_HUNDREDTHS_BPS {
        return Err(HlsError::Config(format!(
            "{label} must be at most 10000 basis points"
        )));
    }
    Ok(())
}

fn validate_ratio(label: &str, ratio_hundredths: u32) -> HlsResult<()> {
    if ratio_hundredths > MAX_RATIO_HUNDREDTHS {
        return Err(HlsError::Config(format!(
            "{label} must be between 0 and 10000 hundredths"
        )));
    }
    Ok(())
}

fn hundredths_bps_to_bps(value: u32) -> f64 {
    f64::from(value) / 100.0
}

fn default_taker_fill_ratio_hundredths() -> u32 {
    MAX_RATIO_HUNDREDTHS
}
