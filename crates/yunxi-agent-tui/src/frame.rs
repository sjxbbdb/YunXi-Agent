use std::collections::BTreeSet;
use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
pub(crate) struct RedrawScheduler {
    last_draw: Option<Instant>,
    min_frame_interval: Duration,
    pending: BTreeSet<RedrawReason>,
    immediate: bool,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum RedrawReason {
    InputChanged,
    StreamDelta,
    StreamFinalized,
    ScrollChanged,
    Resize,
    StatusChanged,
    ControlChanged,
    Error,
    CancelCurrentTurn,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RedrawPriority {
    Immediate,
    NextFrame,
    Coalesced,
}

impl RedrawReason {
    pub(crate) fn priority(self) -> RedrawPriority {
        match self {
            Self::InputChanged
            | Self::ScrollChanged
            | Self::Resize
            | Self::Error
            | Self::CancelCurrentTurn => RedrawPriority::Immediate,
            Self::StreamFinalized | Self::ControlChanged => RedrawPriority::NextFrame,
            Self::StreamDelta | Self::StatusChanged => RedrawPriority::Coalesced,
        }
    }
}

impl Default for RedrawScheduler {
    fn default() -> Self {
        Self::new(Duration::from_millis(33))
    }
}

impl RedrawScheduler {
    pub(crate) fn new(min_frame_interval: Duration) -> Self {
        Self {
            last_draw: None,
            min_frame_interval,
            pending: BTreeSet::new(),
            immediate: false,
        }
    }

    pub(crate) fn request(&mut self, reason: RedrawReason) -> RedrawPriority {
        let priority = reason.priority();
        self.pending.insert(reason);
        self.immediate |= priority == RedrawPriority::Immediate;
        priority
    }

    pub(crate) fn should_draw(&self, now: Instant) -> bool {
        if self.pending.is_empty() {
            return false;
        }
        if self.immediate
            || self
                .pending
                .iter()
                .any(|reason| reason.priority() == RedrawPriority::NextFrame)
        {
            return true;
        }
        self.last_draw
            .map(|last_draw| now.saturating_duration_since(last_draw) >= self.min_frame_interval)
            .unwrap_or(true)
    }

    pub(crate) fn record_draw(&mut self, now: Instant) {
        self.last_draw = Some(now);
        self.pending.clear();
        self.immediate = false;
    }

    #[cfg(test)]
    pub(crate) fn pending_count(&self) -> usize {
        self.pending.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coalesces_high_rate_stream_deltas_until_interval_elapses() {
        let mut scheduler = RedrawScheduler::new(Duration::from_millis(40));
        let t0 = Instant::now();

        scheduler.request(RedrawReason::StreamDelta);
        assert!(scheduler.should_draw(t0));
        scheduler.record_draw(t0);

        for _ in 0..100 {
            scheduler.request(RedrawReason::StreamDelta);
        }
        assert_eq!(scheduler.pending_count(), 1);
        assert!(!scheduler.should_draw(t0 + Duration::from_millis(20)));
        assert!(scheduler.should_draw(t0 + Duration::from_millis(40)));
    }

    #[test]
    fn resize_and_cancel_bypass_stream_throttle() {
        let mut scheduler = RedrawScheduler::new(Duration::from_millis(40));
        let t0 = Instant::now();

        scheduler.request(RedrawReason::StreamDelta);
        assert!(scheduler.should_draw(t0));
        scheduler.record_draw(t0);

        scheduler.request(RedrawReason::Resize);
        assert!(scheduler.should_draw(t0 + Duration::from_millis(1)));
        scheduler.record_draw(t0 + Duration::from_millis(1));

        scheduler.request(RedrawReason::CancelCurrentTurn);
        assert!(scheduler.should_draw(t0 + Duration::from_millis(2)));
    }

    #[test]
    fn final_and_control_state_draw_on_next_tick_and_clear_pending_reasons() {
        let mut scheduler = RedrawScheduler::new(Duration::from_millis(40));
        let t0 = Instant::now();
        scheduler.request(RedrawReason::StreamDelta);
        scheduler.record_draw(t0);

        assert_eq!(
            scheduler.request(RedrawReason::StreamFinalized),
            RedrawPriority::NextFrame
        );
        scheduler.request(RedrawReason::ControlChanged);
        assert!(scheduler.should_draw(t0 + Duration::from_millis(1)));
        scheduler.record_draw(t0 + Duration::from_millis(1));
        assert_eq!(scheduler.pending_count(), 0);
    }
}
