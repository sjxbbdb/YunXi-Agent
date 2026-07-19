use std::collections::BTreeSet;
use std::time::{Duration, Instant};

pub(crate) const DEFAULT_MIN_FRAME_INTERVAL: Duration = Duration::from_micros(33_334);

#[derive(Clone, Debug)]
pub(crate) struct RedrawScheduler {
    last_draw: Option<Instant>,
    min_frame_interval: Duration,
    pending: BTreeSet<RedrawReason>,
    immediate: bool,
    draw_count: u64,
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
        Self::new(DEFAULT_MIN_FRAME_INTERVAL)
    }
}

impl RedrawScheduler {
    pub(crate) fn new(min_frame_interval: Duration) -> Self {
        Self {
            last_draw: None,
            min_frame_interval,
            pending: BTreeSet::new(),
            immediate: false,
            draw_count: 0,
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
        self.draw_count = self.draw_count.saturating_add(1);
    }

    #[cfg(test)]
    pub(crate) fn pending_count(&self) -> usize {
        self.pending.len()
    }

    #[cfg(test)]
    pub(crate) fn draw_count(&self) -> u64 {
        self.draw_count
    }

    #[cfg(test)]
    pub(crate) fn min_frame_interval(&self) -> Duration {
        self.min_frame_interval
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_interval_is_strictly_bounded_to_30_fps() {
        let scheduler = RedrawScheduler::default();

        assert_eq!(
            scheduler.min_frame_interval(),
            Duration::from_micros(33_334)
        );
        assert!(scheduler.min_frame_interval() > Duration::from_secs(1) / 30);
    }

    #[test]
    fn throttles_1000_stream_deltas_below_30_fps_limit() {
        let mut scheduler = RedrawScheduler::default();
        let started_at = Instant::now();
        let test_window = Duration::from_secs(1);

        for millisecond in 0..1_000_u64 {
            let now = started_at + Duration::from_millis(millisecond);
            scheduler.request(RedrawReason::StreamDelta);
            if scheduler.should_draw(now) {
                scheduler.record_draw(now);
            }
        }

        let draw_count = scheduler.draw_count();
        let max_draws_at_30_fps = test_window.as_secs() * 30;
        assert!(draw_count <= max_draws_at_30_fps, "draw_count={draw_count}");
        assert!(draw_count < 100, "draw_count={draw_count}");
        assert_eq!(scheduler.pending_count(), 1);
    }

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
    fn immediate_reasons_bypass_stream_throttle() {
        let mut scheduler = RedrawScheduler::new(Duration::from_millis(40));
        let t0 = Instant::now();

        scheduler.request(RedrawReason::StreamDelta);
        assert!(scheduler.should_draw(t0));
        scheduler.record_draw(t0);

        for (millisecond, reason) in [
            RedrawReason::InputChanged,
            RedrawReason::ScrollChanged,
            RedrawReason::Resize,
            RedrawReason::Error,
            RedrawReason::CancelCurrentTurn,
        ]
        .into_iter()
        .enumerate()
        {
            let now = t0 + Duration::from_millis(millisecond as u64 + 1);
            assert_eq!(scheduler.request(reason), RedrawPriority::Immediate);
            assert!(scheduler.should_draw(now), "reason={reason:?}");
            scheduler.record_draw(now);
        }
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
