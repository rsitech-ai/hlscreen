use std::collections::VecDeque;

use hls_core::alerts::{AlertAction, AlertEvent, AlertSeverity};

use crate::theme::truncate_chars;

pub const MAX_TUI_ALERT_ROWS: usize = 64;
pub const MAX_TUI_ALERT_BYTES: usize = 32 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TuiAlertRecord {
    pub triggered_at_ms: i64,
    pub severity: AlertSeverity,
    pub playbook_id: String,
    pub rule_id: String,
    pub symbol: String,
    pub reason: String,
    pub action: AlertAction,
}

impl TuiAlertRecord {
    fn from_event(event: AlertEvent) -> Option<Self> {
        if event.action != AlertAction::LocalOnly {
            return None;
        }
        Some(Self {
            triggered_at_ms: event.triggered_at_ms,
            severity: event.severity,
            playbook_id: truncate_chars(&event.playbook_id, 64),
            rule_id: truncate_chars(&event.rule_id, 64),
            symbol: truncate_chars(&event.symbol, 32),
            reason: truncate_chars(&event.reason, 256),
            action: event.action,
        })
    }

    fn byte_size(&self) -> usize {
        std::mem::size_of::<Self>()
            .saturating_add(self.playbook_id.capacity())
            .saturating_add(self.rule_id.capacity())
            .saturating_add(self.symbol.capacity())
            .saturating_add(self.reason.capacity())
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BoundedAlertHistory {
    records: VecDeque<TuiAlertRecord>,
    bytes: usize,
}

impl BoundedAlertHistory {
    pub fn push(&mut self, event: AlertEvent) -> bool {
        let Some(record) = TuiAlertRecord::from_event(event) else {
            return false;
        };
        self.bytes = self.bytes.saturating_add(record.byte_size());
        self.records.push_front(record);
        while self.records.len() > MAX_TUI_ALERT_ROWS || self.bytes > MAX_TUI_ALERT_BYTES {
            let Some(removed) = self.records.pop_back() else {
                break;
            };
            self.bytes = self.bytes.saturating_sub(removed.byte_size());
        }
        true
    }

    pub fn extend(&mut self, events: impl IntoIterator<Item = AlertEvent>) {
        for event in events {
            self.push(event);
        }
    }

    pub fn records(&self) -> &VecDeque<TuiAlertRecord> {
        &self.records
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn bytes(&self) -> usize {
        self.bytes
    }
}
