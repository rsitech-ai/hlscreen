use hls_core::{HlsError, HlsResult};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StreamKind {
    Trades,
    Bbo,
    ActiveAssetCtx,
    Candle1m,
}

#[derive(Clone, Debug)]
pub struct SubscriptionPlan {
    symbols: Vec<String>,
    streams: Vec<StreamKind>,
    max_subscriptions: usize,
}

impl SubscriptionPlan {
    pub fn new(symbols: Vec<String>) -> Self {
        Self {
            symbols,
            streams: vec![
                StreamKind::Trades,
                StreamKind::Bbo,
                StreamKind::ActiveAssetCtx,
                StreamKind::Candle1m,
            ],
            max_subscriptions: 500,
        }
    }

    pub fn with_streams(mut self, streams: impl IntoIterator<Item = StreamKind>) -> Self {
        self.streams = streams.into_iter().collect();
        self
    }

    pub fn with_max_subscriptions(mut self, max_subscriptions: usize) -> Self {
        self.max_subscriptions = max_subscriptions;
        self
    }

    pub fn subscription_count(&self) -> usize {
        self.symbols.len() * self.streams.len()
    }

    pub fn validate(&self) -> HlsResult<()> {
        if self.symbols.is_empty() {
            return Err(HlsError::Config(
                "at least one live symbol is required".to_owned(),
            ));
        }

        let count = self.subscription_count();
        if count > self.max_subscriptions {
            return Err(HlsError::Config(format!(
                "subscription budget exceeded: requested {count}, max {}",
                self.max_subscriptions
            )));
        }

        Ok(())
    }
}
