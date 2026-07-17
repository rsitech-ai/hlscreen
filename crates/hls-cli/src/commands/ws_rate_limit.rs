use std::{collections::VecDeque, time::Duration};

pub(crate) const WS_OUTBOUND_RATE_WINDOW: Duration = Duration::from_secs(60);
pub(crate) const WS_OUTBOUND_RATE_BUDGET: usize = 1_900;

#[derive(Debug)]
pub(crate) struct RollingRateLimiter {
    budget: usize,
    window: Duration,
    reservations: VecDeque<RateReservation>,
}

#[derive(Clone, Copy, Debug)]
struct RateReservation {
    reserved_at: tokio::time::Instant,
    weight: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct InvalidRateLimitWeight {
    requested: usize,
    budget: usize,
}

impl std::fmt::Display for InvalidRateLimitWeight {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "rate-limit reservation weight {} must be between 1 and the configured budget {}",
            self.requested, self.budget
        )
    }
}

impl std::error::Error for InvalidRateLimitWeight {}

pub(crate) type RollingMessageRateLimiter = RollingRateLimiter;

impl Default for RollingRateLimiter {
    fn default() -> Self {
        Self::new(WS_OUTBOUND_RATE_BUDGET, WS_OUTBOUND_RATE_WINDOW)
    }
}

impl RollingRateLimiter {
    pub(crate) fn new(budget: usize, window: Duration) -> Self {
        assert!(budget > 0, "outbound message budget must be positive");
        assert!(!window.is_zero(), "outbound rate window must be positive");
        Self {
            budget,
            window,
            reservations: VecDeque::new(),
        }
    }

    pub(crate) fn next_available_at(
        &mut self,
        now: tokio::time::Instant,
    ) -> Option<tokio::time::Instant> {
        self.next_available_at_for(now, 1)
            .expect("unit reservations fit a positive rate-limit budget")
    }

    pub(crate) fn next_available_at_for(
        &mut self,
        now: tokio::time::Instant,
        weight: usize,
    ) -> Result<Option<tokio::time::Instant>, InvalidRateLimitWeight> {
        self.validate_weight(weight)?;
        self.prune(now);

        let reserved: usize = self
            .reservations
            .iter()
            .map(|reservation| reservation.weight)
            .sum();
        if reserved.saturating_add(weight) <= self.budget {
            return Ok(None);
        }

        let mut released = 0_usize;
        for reservation in &self.reservations {
            released = released.saturating_add(reservation.weight);
            if reserved.saturating_sub(released).saturating_add(weight) <= self.budget {
                return Ok(Some(
                    reservation
                        .reserved_at
                        .checked_add(self.window)
                        .expect("positive rolling-window deadline must fit Tokio Instant"),
                ));
            }
        }

        unreachable!("a validated reservation must fit after the rolling window drains")
    }

    pub(crate) fn record(&mut self, now: tokio::time::Instant) {
        self.record_weight(now, 1)
            .expect("unit reservations fit a positive rate-limit budget");
    }

    pub(crate) fn record_weight(
        &mut self,
        now: tokio::time::Instant,
        weight: usize,
    ) -> Result<(), InvalidRateLimitWeight> {
        self.validate_weight(weight)?;
        self.prune(now);
        self.reservations.push_back(RateReservation {
            reserved_at: now,
            weight,
        });
        Ok(())
    }

    fn prune(&mut self, now: tokio::time::Instant) {
        while self.reservations.front().is_some_and(|reservation| {
            now.saturating_duration_since(reservation.reserved_at) >= self.window
        }) {
            self.reservations.pop_front();
        }
    }

    fn validate_weight(&self, weight: usize) -> Result<(), InvalidRateLimitWeight> {
        if weight == 0 || weight > self.budget {
            return Err(InvalidRateLimitWeight {
                requested: weight,
                budget: self.budget,
            });
        }
        Ok(())
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

    #[test]
    fn weighted_reservations_release_in_true_rolling_order() {
        let started = tokio::time::Instant::now();
        let mut limiter = RollingRateLimiter::new(10, Duration::from_secs(60));

        limiter
            .record_weight(started, 4)
            .expect("first reservation");
        limiter
            .record_weight(started + Duration::from_secs(10), 3)
            .expect("second reservation");

        assert_eq!(
            limiter
                .next_available_at_for(started + Duration::from_secs(20), 5)
                .expect("valid weight"),
            Some(started + Duration::from_secs(60))
        );
        assert_eq!(
            limiter
                .next_available_at_for(started + Duration::from_secs(60), 7)
                .expect("exact-window release"),
            None
        );
        assert_eq!(
            limiter
                .next_available_at_for(started + Duration::from_secs(60), 8)
                .expect("second reservation still blocks"),
            Some(started + Duration::from_secs(70))
        );
    }

    #[test]
    fn weighted_reservations_reject_zero_and_over_budget_units() {
        let now = tokio::time::Instant::now();
        let mut limiter = RollingRateLimiter::new(10, Duration::from_secs(60));

        assert!(limiter.next_available_at_for(now, 0).is_err());
        assert!(limiter.next_available_at_for(now, 11).is_err());
        assert!(limiter.record_weight(now, 0).is_err());
        assert!(limiter.record_weight(now, 11).is_err());
    }
}
