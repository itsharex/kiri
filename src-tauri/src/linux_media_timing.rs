//! Pure timing and queue invariants shared by Linux media tests.

use std::time::Duration;

/// Presentation timestamps use a monotonic clock, never the number of frames
/// delivered by a variable-rate screen source. Keep one pending image until
/// its successor (or stop) gives it an exact duration.
#[derive(Debug)]
pub(super) struct FrameTimeline {
    interval: Duration,
    pending: Duration,
}

impl FrameTimeline {
    pub(super) fn new(fps: u32) -> Self {
        Self {
            interval: Duration::from_nanos(1_000_000_000 / u64::from(fps.max(1))),
            pending: Duration::ZERO,
        }
    }

    pub(super) fn advance(&mut self, elapsed: Duration) -> Option<(Duration, Duration)> {
        let duration = elapsed.checked_sub(self.pending)?;
        // Round into fixed frame slots rather than measuring an interval from
        // the last accepted frame. A 30 fps source can deliver at 33, 67, 100
        // ms after scheduling jitter; requiring another full 33.333 ms after
        // each accepted sample would discard every other otherwise valid frame.
        let slot = |time: Duration| {
            (time.as_nanos() + self.interval.as_nanos() / 2) / self.interval.as_nanos()
        };
        if duration.is_zero() || slot(elapsed) <= slot(self.pending) {
            return None;
        }
        let previous = self.pending;
        self.pending = elapsed;
        Some((previous, duration))
    }

    pub(super) fn finish(&self, elapsed: Duration) -> (Duration, Duration) {
        (
            self.pending,
            elapsed
                .saturating_sub(self.pending)
                .max(Duration::from_nanos(1)),
        )
    }
}

pub(super) fn queue_has_room(queued_bytes: u64, frame_bytes: u64, capacity: u64) -> bool {
    frame_bytes > 0
        && queued_bytes
            .checked_add(frame_bytes)
            .is_some_and(|next| next <= capacity)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_screen_keeps_its_wall_clock_duration() {
        let timeline = FrameTimeline::new(30);
        assert_eq!(
            timeline.finish(Duration::from_secs(8)),
            (Duration::ZERO, Duration::from_secs(8))
        );
    }

    #[test]
    fn missing_frames_do_not_speed_up_the_recording() {
        let mut timeline = FrameTimeline::new(30);
        assert_eq!(
            timeline.advance(Duration::from_millis(100)),
            Some((Duration::ZERO, Duration::from_millis(100)))
        );
        assert_eq!(
            timeline.advance(Duration::from_millis(1100)),
            Some((Duration::from_millis(100), Duration::from_secs(1)))
        );
        assert_eq!(
            timeline.finish(Duration::from_millis(1300)),
            (Duration::from_millis(1100), Duration::from_millis(200))
        );
    }

    #[test]
    fn nominal_thirty_fps_survives_normal_delivery_jitter() {
        let mut timeline = FrameTimeline::new(30);
        let mut total = Duration::ZERO;
        for millis in [33, 67, 100, 133, 167, 200] {
            let (_, duration) = timeline
                .advance(Duration::from_millis(millis))
                .expect("a nominal 30 fps frame must not be dropped");
            total += duration;
        }
        assert_eq!(total, Duration::from_millis(200));
    }

    #[test]
    fn high_refresh_frames_do_not_make_nonmonotonic_timestamps() {
        let mut timeline = FrameTimeline::new(30);
        assert!(timeline.advance(Duration::from_millis(7)).is_none());
        assert!(timeline.advance(Duration::from_millis(14)).is_none());
        assert_eq!(
            timeline.advance(Duration::from_millis(35)),
            Some((Duration::ZERO, Duration::from_millis(35)))
        );
        assert!(timeline.advance(Duration::from_millis(30)).is_none());
    }

    #[test]
    fn appsrc_accepts_at_most_two_complete_frames() {
        assert!(queue_has_room(0, 100, 200));
        assert!(queue_has_room(100, 100, 200));
        assert!(!queue_has_room(101, 100, 200));
        assert!(!queue_has_room(200, 100, 200));
        assert!(!queue_has_room(u64::MAX, 1, u64::MAX));
        assert!(!queue_has_room(0, 0, 200));
    }
}
