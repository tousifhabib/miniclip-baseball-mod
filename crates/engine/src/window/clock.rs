//! When the screen shows what is drawn: the moments it redraws, as near as
//! they can be told, when each frame of the game is first to be seen, and
//! so when a press was made by the frames the player was looking at.
//!
//! Waiting for a picture to draw on is waiting for the screen. The window
//! keeps two pictures: one on the screen, and one to draw the next frame
//! on. The one on the screen is free again once the screen has taken the
//! next in its place, which it does as it redraws. So a wait for a picture
//! that ends ends as the screen redraws, and what is handed to the screen
//! after it is seen at the redraw after that.
//!
//! The window could ask for its next picture as soon as it has handed one
//! over, and wait out most of a redraw for it. While it waits it takes
//! nothing in, so a click made then is not heard of until the wait is over.
//! Instead it sleeps until just before the picture will be free, taking in
//! clicks as they come, and wakes to wait the last moment for it. That
//! short wait still ends as the screen redraws, which keeps the clock true.

use std::time::{Duration, Instant};

/// A wait shorter than this was no wait: the picture was free already,
/// and when the screen took it is not known.
const NO_WAIT: Duration = Duration::from_micros(250);

/// How long before the screen will free a picture the window wakes to ask
/// for it.
const EARLY: Duration = Duration::from_millis(2);

pub(super) struct ScreenClock {
    /// How long a frame of the game lasts.
    frame: Duration,
    /// How long the screen takes to redraw, if it has said.
    redraw: Option<Duration>,
    /// A moment the screen redrew, if one is known.
    redrew: Option<Instant>,
    /// Whether the last wait for a picture ended as the screen redrew, so
    /// that the moments to come can be told from it.
    in_step: bool,
    /// When the picture handed over last is to be seen, which is when the
    /// one before it is freed.
    seen: Option<Instant>,
    /// The latest frame of the game handed to the screen, and when it was
    /// first to be seen.
    shown: Option<(u64, Instant)>,
}

impl ScreenClock {
    pub(super) fn new(frame: Duration) -> ScreenClock {
        ScreenClock {
            frame,
            redraw: None,
            redrew: None,
            in_step: false,
            seen: None,
            shown: None,
        }
    }

    /// The screen redraws this often, or has not said.
    pub(super) fn redraws_every(&mut self, redraw: Option<Duration>) {
        self.redraw = redraw.filter(|redraw| !redraw.is_zero());
    }

    /// A picture to draw on was asked for at `asked` and had at `had`.
    pub(super) fn waited(&mut self, asked: Instant, had: Instant) {
        self.in_step = had.saturating_duration_since(asked) >= NO_WAIT;
        if self.in_step {
            self.redrew = Some(had);
        }
    }

    /// A picture has been handed to the screen at `now`, with `frame` the
    /// latest frame of the game played.
    pub(super) fn handed_over(&mut self, frame: u64, now: Instant) {
        let seen = self.next_redraw(now);
        self.seen = Some(seen);
        // A frame shown on two redraws running was first seen on the
        // first.
        if self.shown.is_none_or(|(shown, _)| shown != frame) {
            self.shown = Some((frame, seen));
        }
    }

    /// The first moment after `now` that the screen redraws, by what is
    /// known of it. Not knowing, `now`.
    fn next_redraw(&self, now: Instant) -> Instant {
        let (Some(redrew), Some(every)) = (self.redrew, self.redraw) else {
            return now;
        };
        let since = now.saturating_duration_since(redrew);
        let redraws = since.as_nanos() / every.as_nanos() + 1;
        redrew + every * redraws as u32
    }

    /// When the window should wake to ask for its next picture: a moment
    /// before the screen frees it. `None` to ask at once, which it does
    /// while it does not know when that will be. A wait that then takes
    /// most of a redraw ends as the screen redraws, and the clock is in
    /// step again.
    pub(super) fn wake_at(&self) -> Option<Instant> {
        if !self.in_step || self.redraw.is_none() {
            return None;
        }
        self.seen?.checked_sub(EARLY)
    }

    /// When `now` is in frames, on the clock a press is timed by: its
    /// whole numbers are the moments the frames of those numbers were
    /// first to be seen. `None` until a frame has been shown.
    pub(super) fn moment(&self, now: Instant) -> Option<f64> {
        let (frame, seen) = self.shown?;
        let since = if now >= seen {
            (now - seen).as_secs_f64()
        } else {
            -(seen - now).as_secs_f64()
        };
        Some(frame as f64 + since / self.frame.as_secs_f64())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FRAME: Duration = Duration::from_micros(16_667);

    /// The moment `by` before `at`.
    fn before(at: Instant, by: Duration) -> Instant {
        at.checked_sub(by).unwrap()
    }

    fn ms(ms: f64) -> Duration {
        Duration::from_secs_f64(ms / 1000.0)
    }

    /// A clock for a screen that redraws as often as the game plays, and
    /// a moment it redrew.
    fn in_step() -> (ScreenClock, Instant) {
        let mut clock = ScreenClock::new(FRAME);
        clock.redraws_every(Some(FRAME));
        let start = Instant::now();
        // A wait of most of a redraw, which ended as the screen redrew.
        clock.waited(start, start + ms(14.0));
        (clock, start + ms(14.0))
    }

    #[test]
    fn what_is_handed_over_is_seen_at_the_next_redraw() {
        let (mut clock, redrew) = in_step();
        clock.handed_over(10, redrew + ms(3.0));
        assert_eq!(clock.shown, Some((10, redrew + FRAME)));
        // And it is woken a little before then for the next picture.
        assert_eq!(clock.wake_at(), Some(before(redrew + FRAME, EARLY)));
    }

    #[test]
    fn a_frame_shown_twice_was_first_seen_the_first_time() {
        let (mut clock, redrew) = in_step();
        clock.handed_over(10, redrew + ms(3.0));
        clock.waited(before(redrew + FRAME, EARLY), redrew + FRAME);
        clock.handed_over(10, redrew + FRAME + ms(3.0));
        assert_eq!(clock.shown, Some((10, redrew + FRAME)));
        // The picture to come is woken for all the same.
        assert_eq!(clock.wake_at(), Some(before(redrew + FRAME * 2, EARLY)));
    }

    #[test]
    fn a_press_is_timed_by_the_frame_on_the_screen() {
        let (mut clock, redrew) = in_step();
        clock.handed_over(10, redrew + ms(3.0));
        let seen = redrew + FRAME;
        let at = |moment| clock.moment(moment).unwrap();
        assert!((at(seen) - 10.0).abs() < 1e-9);
        assert!((at(seen + FRAME / 4) - 10.25).abs() < 1e-6);
        // Before frame 10 is seen, the ninth is still showing.
        assert!((at(before(seen, FRAME / 2)) - 9.5).abs() < 1e-6);
    }

    #[test]
    fn a_wait_that_was_no_wait_puts_the_clock_out_of_step_until_one_is() {
        let (mut clock, redrew) = in_step();
        clock.handed_over(10, redrew + ms(3.0));
        // Woken late, the picture was free already.
        clock.waited(redrew + FRAME + ms(1.0), redrew + FRAME + ms(1.1));
        assert_eq!(clock.wake_at(), None);
        // What was known of the screen still says when a frame is seen.
        clock.handed_over(11, redrew + FRAME + ms(4.0));
        assert_eq!(clock.shown, Some((11, redrew + FRAME * 2)));
        // Asked at once, the next picture is waited for, and the clock is
        // in step again from the moment that wait ends.
        let later = redrew + FRAME * 2 + ms(0.4);
        clock.waited(redrew + FRAME + ms(5.0), later);
        assert!(clock.wake_at().is_some());
        assert_eq!(clock.redrew, Some(later));
    }

    #[test]
    fn a_screen_that_has_not_said_how_often_it_redraws_is_taken_to_show_at_once() {
        let mut clock = ScreenClock::new(FRAME);
        let start = Instant::now();
        assert_eq!(clock.moment(start), None);
        clock.waited(start, start + ms(10.0));
        clock.handed_over(3, start + ms(12.0));
        assert_eq!(clock.shown, Some((3, start + ms(12.0))));
        assert!(clock.wake_at().is_none());
    }
}
