use std::{collections::VecDeque, time::Duration};

pub(crate) const WS_OUTBOUND_RATE_WINDOW: Duration = Duration::from_secs(60);
pub(crate) const WS_OUTBOUND_RATE_BUDGET: usize = 1_900;

#[derive(Debug)]
pub(crate) struct RollingMessageRateLimiter {
    budget: usize,
    window: Duration,
    sent_at: VecDeque<tokio::time::Instant>,
}

impl Default for RollingMessageRateLimiter {
    fn default() -> Self {
        Self::new(WS_OUTBOUND_RATE_BUDGET, WS_OUTBOUND_RATE_WINDOW)
    }
}

impl RollingMessageRateLimiter {
    pub(crate) fn new(budget: usize, window: Duration) -> Self {
        assert!(budget > 0, "outbound message budget must be positive");
        assert!(!window.is_zero(), "outbound rate window must be positive");
        Self {
            budget,
            window,
            sent_at: VecDeque::new(),
        }
    }

    pub(crate) fn next_available_at(
        &mut self,
        now: tokio::time::Instant,
    ) -> Option<tokio::time::Instant> {
        self.prune(now);
        let blocking_index = self.sent_at.len().checked_sub(self.budget)?;
        self.sent_at
            .get(blocking_index)
            .and_then(|sent_at| sent_at.checked_add(self.window))
    }

    pub(crate) fn record(&mut self, now: tokio::time::Instant) {
        self.prune(now);
        self.sent_at.push_back(now);
    }

    fn prune(&mut self, now: tokio::time::Instant) {
        while self
            .sent_at
            .front()
            .is_some_and(|sent_at| now.saturating_duration_since(*sent_at) >= self.window)
        {
            self.sent_at.pop_front();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rolling_window_defers_only_messages_beyond_the_budget() {
        let started = tokio::time::Instant::now();
        let mut limiter = RollingMessageRateLimiter::new(3, Duration::from_secs(60));

        for _ in 0..3 {
            assert_eq!(limiter.next_available_at(started), None);
            limiter.record(started);
        }
        assert_eq!(
            limiter.next_available_at(started + Duration::from_secs(1)),
            Some(started + Duration::from_secs(60))
        );
        assert_eq!(
            limiter.next_available_at(started + Duration::from_secs(60)),
            None
        );
    }
}
