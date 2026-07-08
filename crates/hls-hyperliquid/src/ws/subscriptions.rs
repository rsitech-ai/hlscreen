use hls_core::{HlsError, HlsResult};
use serde_json::json;

const OFFICIAL_WS_SUBSCRIPTION_LIMIT: usize = 1_000;
const DEFAULT_SUBSCRIPTION_HEADROOM: usize = 20;

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
            max_subscriptions: OFFICIAL_WS_SUBSCRIPTION_LIMIT - DEFAULT_SUBSCRIPTION_HEADROOM,
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

    pub fn symbols(&self) -> &[String] {
        &self.symbols
    }

    pub fn streams(&self) -> &[StreamKind] {
        &self.streams
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

    pub fn subscribe_messages(&self) -> HlsResult<Vec<String>> {
        self.validate()?;

        self.symbols
            .iter()
            .flat_map(|symbol| {
                self.streams
                    .iter()
                    .map(move |stream| subscribe_message(symbol, *stream))
            })
            .collect()
    }
}

pub fn subscribe_message(symbol: &str, stream: StreamKind) -> HlsResult<String> {
    let subscription = match stream {
        StreamKind::Trades => json!({ "type": "trades", "coin": symbol }),
        StreamKind::Bbo => json!({ "type": "bbo", "coin": symbol }),
        StreamKind::ActiveAssetCtx => json!({ "type": "activeAssetCtx", "coin": symbol }),
        StreamKind::Candle1m => json!({ "type": "candle", "coin": symbol, "interval": "1m" }),
    };

    serde_json::to_string(&json!({
        "method": "subscribe",
        "subscription": subscription,
    }))
    .map_err(|err| HlsError::Parse(format!("serialize subscription message: {err}")))
}

pub fn ping_message() -> &'static str {
    r#"{"method":"ping"}"#
}
