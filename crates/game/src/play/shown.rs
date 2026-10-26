//! What the batting view showed on each of the last few frames of a pitch,
//! so that a click can be taken as made on the frame the player was
//! looking at, and not the later one it reached the game on.

use std::collections::VecDeque;

use bb_engine::stage::Stage;

use crate::delay::{self, Delay};
use crate::play::pitch::Point;

/// How many frames back a click can be taken as made: half a second.
const KEPT: usize = 30;

/// What one frame of the batting view showed of the pitch.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Seen {
    /// The step of the ball's flight a swing begun on the frame begins
    /// on: the step the flight had come to, or, before the ball was
    /// thrown, the frames of the wind-up still to go, below nought.
    pub step: i32,
    /// Where the ring was.
    pub aim: Point,
}

/// What the batting view showed, frame by frame, for the last few frames.
#[derive(Debug, Default)]
pub(crate) struct Shown {
    frames: VecDeque<(u64, Seen)>,
}

impl Shown {
    /// The view showed `seen` on `frame`, the frames coming in order.
    pub fn showed(&mut self, frame: u64, seen: Seen) {
        if self.frames.len() == KEPT {
            self.frames.pop_front();
        }
        self.frames.push_back((frame, seen));
    }

    /// What the view showed on `frame`. One further back than is kept is
    /// taken to be the first that is, and one that has not been shown yet
    /// the last. `None` if nothing has been shown.
    pub fn on(&self, frame: i64) -> Option<Seen> {
        let (first, _) = self.frames.front()?;
        let back = frame.saturating_sub(*first as i64).max(0) as usize;
        let at = back.min(self.frames.len() - 1);
        self.frames.get(at).map(|&(_, seen)| seen)
    }

    /// What the view showed when the click being kept was made, for a
    /// player whose clicks reach the game `delay` late. `None` if the
    /// window did not say when it was made, as a script does not: such a
    /// click is taken as made on the frame it reaches.
    pub fn when_clicked(&self, stage: &Stage, delay: Delay) -> Option<Seen> {
        let at = stage.pointer.went_down_at?;
        let frame_rate = stage.library().manifest.stage.frame_rate;
        self.on(delay::aimed_at(at, delay.frames(frame_rate)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seen(step: i32) -> Seen {
        Seen {
            step,
            aim: (step as f32, 0.0),
        }
    }

    #[test]
    fn a_frame_is_found_by_its_number() {
        let mut shown = Shown::default();
        assert_eq!(shown.on(5), None);
        for frame in 100..110 {
            shown.showed(frame, seen(frame as i32 - 105));
        }
        assert_eq!(shown.on(103), Some(seen(-2)));
        assert_eq!(shown.on(109), Some(seen(4)));
        // Not shown yet: the last there is.
        assert_eq!(shown.on(112), Some(seen(4)));
        // Further back than there is: the first.
        assert_eq!(shown.on(90), Some(seen(-5)));
    }

    #[test]
    fn only_the_last_half_second_is_kept() {
        let mut shown = Shown::default();
        for frame in 0..100 {
            shown.showed(frame, seen(frame as i32));
        }
        assert_eq!(shown.frames.len(), KEPT);
        assert_eq!(shown.on(0), Some(seen(100 - KEPT as i32)));
        assert_eq!(shown.on(99), Some(seen(99)));
    }
}
