use std::collections::HashSet;

use hls_core::{HlsError, HlsResult};
use serde_json::json;

pub const OFFICIAL_WS_SUBSCRIPTION_LIMIT: usize = 1_000;
const DEFAULT_SUBSCRIPTION_HEADROOM: usize = 20;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum StreamKind {
    AllMids,
    Trades,
    Bbo,
    ActiveAssetCtx,
    Candle1m,
    L2Book,
}

impl StreamKind {
    fn is_global(self) -> bool {
        matches!(self, Self::AllMids)
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct PlannedSubscription {
    symbol: Option<String>,
    stream: StreamKind,
}

#[derive(Clone, Debug)]
pub struct SubscriptionPlan {
    symbols: Vec<String>,
    streams: Vec<StreamKind>,
    subscriptions: Vec<PlannedSubscription>,
    max_subscriptions: usize,
}

impl SubscriptionPlan {
    pub fn new(symbols: Vec<String>) -> Self {
        let mut plan = Self {
            symbols: deduplicate_symbols(symbols),
            streams: vec![
                StreamKind::Trades,
                StreamKind::Bbo,
                StreamKind::ActiveAssetCtx,
                StreamKind::Candle1m,
            ],
            subscriptions: Vec::new(),
            max_subscriptions: OFFICIAL_WS_SUBSCRIPTION_LIMIT - DEFAULT_SUBSCRIPTION_HEADROOM,
        };
        plan.rebuild_uniform_subscriptions();
        plan
    }

    pub fn tiered(
        symbols: Vec<String>,
        trade_limit: usize,
        bbo_limit: usize,
        selected_l2: Option<String>,
    ) -> Self {
        let symbols = deduplicate_symbols(symbols);
        let mut subscriptions = Vec::with_capacity(
            1 + symbols.len() + trade_limit.min(symbols.len()) + bbo_limit.min(symbols.len()) + 1,
        );
        subscriptions.push(PlannedSubscription {
            symbol: None,
            stream: StreamKind::AllMids,
        });
        subscriptions.extend(symbols.iter().cloned().map(|symbol| PlannedSubscription {
            symbol: Some(symbol),
            stream: StreamKind::Candle1m,
        }));
        subscriptions.extend(symbols.iter().take(trade_limit).cloned().map(|symbol| {
            PlannedSubscription {
                symbol: Some(symbol),
                stream: StreamKind::Trades,
            }
        }));
        subscriptions.extend(symbols.iter().take(bbo_limit).cloned().map(|symbol| {
            PlannedSubscription {
                symbol: Some(symbol),
                stream: StreamKind::Bbo,
            }
        }));
        if let Some(symbol) = selected_l2 {
            subscriptions.push(PlannedSubscription {
                symbol: Some(symbol),
                stream: StreamKind::L2Book,
            });
        }

        let streams = [
            StreamKind::AllMids,
            StreamKind::Candle1m,
            StreamKind::Trades,
            StreamKind::Bbo,
            StreamKind::L2Book,
        ]
        .into_iter()
        .filter(|stream| {
            subscriptions
                .iter()
                .any(|subscription| subscription.stream == *stream)
        })
        .collect();

        Self {
            symbols,
            streams,
            subscriptions,
            max_subscriptions: OFFICIAL_WS_SUBSCRIPTION_LIMIT - DEFAULT_SUBSCRIPTION_HEADROOM,
        }
    }

    pub fn with_streams(mut self, streams: impl IntoIterator<Item = StreamKind>) -> Self {
        self.streams = streams.into_iter().collect();
        self.rebuild_uniform_subscriptions();
        self
    }

    pub fn with_max_subscriptions(mut self, max_subscriptions: usize) -> Self {
        self.max_subscriptions = max_subscriptions;
        self
    }

    pub fn subscription_count(&self) -> usize {
        self.subscriptions.len()
    }

    pub fn stream_count(&self, stream: StreamKind) -> usize {
        self.subscriptions
            .iter()
            .filter(|subscription| subscription.stream == stream)
            .count()
    }

    pub fn symbols(&self) -> &[String] {
        &self.symbols
    }

    pub fn streams(&self) -> &[StreamKind] {
        &self.streams
    }

    pub fn per_symbol_stream_count(&self) -> usize {
        self.streams
            .iter()
            .filter(|stream| !stream.is_global())
            .count()
    }

    pub fn global_stream_count(&self) -> usize {
        self.streams
            .iter()
            .filter(|stream| stream.is_global())
            .count()
    }

    pub fn validate(&self) -> HlsResult<()> {
        if self.symbols.is_empty() {
            return Err(HlsError::Config(
                "at least one live symbol is required".to_owned(),
            ));
        }
        if self.max_subscriptions == 0 {
            return Err(HlsError::Config(
                "max subscriptions must be greater than zero".to_owned(),
            ));
        }
        if self.max_subscriptions > OFFICIAL_WS_SUBSCRIPTION_LIMIT {
            return Err(HlsError::Config(format!(
                "subscription ceiling {} exceeds the official limit of {OFFICIAL_WS_SUBSCRIPTION_LIMIT}",
                self.max_subscriptions
            )));
        }

        let known_symbols = self
            .symbols
            .iter()
            .map(String::as_str)
            .collect::<HashSet<_>>();
        let mut unique = HashSet::with_capacity(self.subscriptions.len());
        for subscription in &self.subscriptions {
            match subscription.symbol.as_deref() {
                None if !subscription.stream.is_global() => {
                    return Err(HlsError::Config(format!(
                        "stream {:?} requires a symbol",
                        subscription.stream
                    )));
                }
                Some("") => {
                    return Err(HlsError::Config(
                        "subscription symbols must not be empty".to_owned(),
                    ));
                }
                Some(symbol) if !known_symbols.contains(symbol) => {
                    return Err(HlsError::Config(format!(
                        "subscription symbol '{symbol}' is outside the selected universe"
                    )));
                }
                _ => {}
            }
            if !unique.insert(subscription) {
                return Err(HlsError::Config(
                    "duplicate websocket subscription in plan".to_owned(),
                ));
            }
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
        self.subscriptions
            .iter()
            .map(|subscription| {
                subscribe_message(
                    subscription.symbol.as_deref().unwrap_or_default(),
                    subscription.stream,
                )
            })
            .collect()
    }

    fn rebuild_uniform_subscriptions(&mut self) {
        self.subscriptions.clear();
        for stream in &self.streams {
            if stream.is_global() {
                self.subscriptions.push(PlannedSubscription {
                    symbol: None,
                    stream: *stream,
                });
            } else {
                self.subscriptions
                    .extend(
                        self.symbols
                            .iter()
                            .cloned()
                            .map(|symbol| PlannedSubscription {
                                symbol: Some(symbol),
                                stream: *stream,
                            }),
                    );
            }
        }
    }
}

pub fn subscribe_message(symbol: &str, stream: StreamKind) -> HlsResult<String> {
    subscription_message("subscribe", subscription_payload(symbol, stream))
}

pub fn unsubscribe_message(symbol: &str, stream: StreamKind) -> HlsResult<String> {
    subscription_message("unsubscribe", subscription_payload(symbol, stream))
}

fn subscription_payload(symbol: &str, stream: StreamKind) -> serde_json::Value {
    match stream {
        StreamKind::AllMids => json!({ "type": "allMids" }),
        StreamKind::Trades => json!({ "type": "trades", "coin": symbol }),
        StreamKind::Bbo => json!({ "type": "bbo", "coin": symbol }),
        StreamKind::ActiveAssetCtx => json!({ "type": "activeAssetCtx", "coin": symbol }),
        StreamKind::Candle1m => json!({ "type": "candle", "coin": symbol, "interval": "1m" }),
        StreamKind::L2Book => json!({ "type": "l2Book", "coin": symbol }),
    }
}

fn subscription_message(method: &str, subscription: serde_json::Value) -> HlsResult<String> {
    serde_json::to_string(&json!({
        "method": method,
        "subscription": subscription,
    }))
    .map_err(|err| HlsError::Parse(format!("serialize subscription message: {err}")))
}

fn deduplicate_symbols(symbols: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::with_capacity(symbols.len());
    symbols
        .into_iter()
        .filter(|symbol| seen.insert(symbol.clone()))
        .collect()
}

pub fn ping_message() -> &'static str {
    r#"{"method":"ping"}"#
}
