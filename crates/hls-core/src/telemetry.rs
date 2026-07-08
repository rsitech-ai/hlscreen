#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LatencySample {
    exchange_ts_ms: u64,
    recv_ts_ms: u64,
    feature_done_ts_ms: u64,
    render_done_ts_ms: u64,
}

impl LatencySample {
    pub fn new(
        exchange_ts_ms: u64,
        recv_ts_ms: u64,
        feature_done_ts_ms: u64,
        render_done_ts_ms: u64,
    ) -> Self {
        Self {
            exchange_ts_ms,
            recv_ts_ms,
            feature_done_ts_ms,
            render_done_ts_ms,
        }
    }

    fn data_lag_ms(self) -> u64 {
        self.recv_ts_ms.saturating_sub(self.exchange_ts_ms)
    }

    fn feature_lag_ms(self) -> u64 {
        self.feature_done_ts_ms.saturating_sub(self.exchange_ts_ms)
    }

    fn render_lag_ms(self) -> u64 {
        self.render_done_ts_ms.saturating_sub(self.exchange_ts_ms)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TelemetryWindow {
    samples: Vec<LatencySample>,
}

impl TelemetryWindow {
    pub fn from_samples(samples: Vec<LatencySample>) -> Self {
        Self { samples }
    }

    pub fn count(&self) -> usize {
        self.samples.len()
    }

    pub fn data_lag_ms_p50(&self) -> Option<u64> {
        percentile(self.samples.iter().map(|sample| sample.data_lag_ms()), 50)
    }

    pub fn data_lag_ms_p95(&self) -> Option<u64> {
        percentile(self.samples.iter().map(|sample| sample.data_lag_ms()), 95)
    }

    pub fn feature_lag_ms_p95(&self) -> Option<u64> {
        percentile(
            self.samples.iter().map(|sample| sample.feature_lag_ms()),
            95,
        )
    }

    pub fn render_lag_ms_p95(&self) -> Option<u64> {
        percentile(self.samples.iter().map(|sample| sample.render_lag_ms()), 95)
    }
}

fn percentile(values: impl Iterator<Item = u64>, percentile: usize) -> Option<u64> {
    let mut values: Vec<u64> = values.collect();
    if values.is_empty() {
        return None;
    }

    values.sort_unstable();
    let rank = ((percentile as f64 / 100.0) * values.len() as f64).ceil() as usize;
    let index = rank.saturating_sub(1).min(values.len() - 1);
    values.get(index).copied()
}
