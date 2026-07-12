use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
pub(crate) struct FrameScheduler {
    last_draw: Option<Instant>,
    min_frame_interval: Duration,
    dirty: bool,
    force_draw: bool,
}

impl Default for FrameScheduler {
    fn default() -> Self {
        Self::new(Duration::from_millis(33))
    }
}

impl FrameScheduler {
    pub(crate) fn new(min_frame_interval: Duration) -> Self {
        Self {
            last_draw: None,
            min_frame_interval,
            dirty: false,
            force_draw: false,
        }
    }

    pub(crate) fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub(crate) fn force(&mut self) {
        self.dirty = true;
        self.force_draw = true;
    }

    pub(crate) fn should_draw(&self, now: Instant) -> bool {
        if !self.dirty {
            return false;
        }
        if self.force_draw {
            return true;
        }
        self.last_draw
            .map(|last_draw| now.saturating_duration_since(last_draw) >= self.min_frame_interval)
            .unwrap_or(true)
    }

    pub(crate) fn record_draw(&mut self, now: Instant) {
        self.last_draw = Some(now);
        self.dirty = false;
        self.force_draw = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn throttles_dirty_frames_until_interval_elapses() {
        let mut scheduler = FrameScheduler::new(Duration::from_millis(40));
        let t0 = Instant::now();

        scheduler.mark_dirty();
        assert!(scheduler.should_draw(t0));
        scheduler.record_draw(t0);

        scheduler.mark_dirty();
        assert!(!scheduler.should_draw(t0 + Duration::from_millis(20)));
        assert!(scheduler.should_draw(t0 + Duration::from_millis(40)));
    }

    #[test]
    fn force_draw_bypasses_throttle() {
        let mut scheduler = FrameScheduler::new(Duration::from_millis(40));
        let t0 = Instant::now();

        scheduler.mark_dirty();
        assert!(scheduler.should_draw(t0));
        scheduler.record_draw(t0);

        scheduler.force();
        assert!(scheduler.should_draw(t0 + Duration::from_millis(1)));
    }
}
