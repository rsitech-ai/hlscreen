use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfidenceLevel {
    High,
    Medium,
    Low,
    Untrusted,
}

impl ConfidenceLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
            Self::Untrusted => "untrusted",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfidenceReason {
    ReconnectGap,
    StaleQuote,
    SparseTrades,
    DuplicateEvents,
    ParserDrops,
    WriterBacklog,
    IncompleteWindow,
}

impl ConfidenceReason {
    fn penalty(self) -> u8 {
        match self {
            Self::ReconnectGap => 35,
            Self::StaleQuote => 25,
            Self::SparseTrades => 20,
            Self::DuplicateEvents => 10,
            Self::ParserDrops => 30,
            Self::WriterBacklog => 20,
            Self::IncompleteWindow => 20,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DataConfidenceSnapshot {
    pub symbol: String,
    pub score: u8,
    pub level: ConfidenceLevel,
    pub reasons: Vec<ConfidenceReason>,
    pub incomplete_windows: Vec<String>,
}

impl DataConfidenceSnapshot {
    pub fn new(symbol: impl Into<String>) -> Self {
        Self {
            symbol: symbol.into(),
            score: 100,
            level: ConfidenceLevel::High,
            reasons: Vec::new(),
            incomplete_windows: Vec::new(),
        }
    }

    pub fn with_reason(mut self, reason: ConfidenceReason) -> Self {
        self.add_reason(reason);
        self
    }

    pub fn with_incomplete_window(mut self, window: impl Into<String>) -> Self {
        self.add_reason(ConfidenceReason::IncompleteWindow);
        let window = window.into();
        if !self
            .incomplete_windows
            .iter()
            .any(|existing| existing == &window)
        {
            self.incomplete_windows.push(window);
        }
        self
    }

    pub fn has_reason(&self, reason: ConfidenceReason) -> bool {
        self.reasons.contains(&reason)
    }

    pub fn is_trusted(&self) -> bool {
        matches!(self.level, ConfidenceLevel::High | ConfidenceLevel::Medium)
    }

    fn add_reason(&mut self, reason: ConfidenceReason) {
        if self.has_reason(reason) {
            return;
        }
        self.reasons.push(reason);
        self.recompute();
    }

    fn recompute(&mut self) {
        let penalty: u16 = self
            .reasons
            .iter()
            .map(|reason| u16::from(reason.penalty()))
            .sum();
        self.score = 100_u16.saturating_sub(penalty).min(100) as u8;
        self.level = match self.score {
            90..=100 => ConfidenceLevel::High,
            70..=89 => ConfidenceLevel::Medium,
            30..=69 => ConfidenceLevel::Low,
            _ => ConfidenceLevel::Untrusted,
        };
    }
}
