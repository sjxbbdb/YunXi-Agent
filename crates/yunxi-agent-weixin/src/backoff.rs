use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WeixinBackoff {
    base: Duration,
    max: Duration,
    attempt: u32,
}

impl WeixinBackoff {
    pub fn new(base: Duration, max: Duration) -> Self {
        assert!(!base.is_zero(), "base backoff must be non-zero");
        assert!(max >= base, "max backoff must be >= base");
        Self {
            base,
            max,
            attempt: 0,
        }
    }

    pub fn reset(&mut self) {
        self.attempt = 0;
    }

    pub fn next_delay(&mut self) -> Duration {
        let shift = self.attempt.min(10);
        let multiplier = 1_u32.checked_shl(shift).unwrap_or(u32::MAX);
        let without_jitter = self.base.saturating_mul(multiplier).min(self.max);
        self.attempt = self.attempt.saturating_add(1);
        without_jitter
            .saturating_add(jitter(without_jitter))
            .min(self.max)
    }
}

impl Default for WeixinBackoff {
    fn default() -> Self {
        Self::new(Duration::from_millis(250), Duration::from_secs(15))
    }
}

fn jitter(duration: Duration) -> Duration {
    let millis = duration.as_millis() as u64;
    if millis < 10 {
        return Duration::ZERO;
    }
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos() as u64;
    let bound = (millis / 5).max(1);
    Duration::from_millis(nanos % bound)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_is_bounded_and_resettable() {
        let mut backoff = WeixinBackoff::new(Duration::from_millis(10), Duration::from_millis(80));
        let first = backoff.next_delay();
        let second = backoff.next_delay();
        assert!(first >= Duration::from_millis(10));
        assert!(second >= Duration::from_millis(20));
        for _ in 0..20 {
            assert!(backoff.next_delay() <= Duration::from_millis(80));
        }
        backoff.reset();
        assert!(backoff.next_delay() <= Duration::from_millis(12));
    }
}
