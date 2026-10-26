//! Measuring the delay. A marker crosses a bar, again and again, at the
//! pace the timing bar's does, and the player clicks as it reaches a line
//! on it. How late the clicks come after the marker was shown there is
//! the delay.
//!
//! Ten clicks make a measurement, and their middle one is taken, so that a
//! click or two well off the line does not throw it out. If they are
//! spread too wide to say anything, nothing is taken.

use super::Delay;

/// How far the marker moves each frame, how long the bar is, and how far
/// along it the line is: as the timing bar has them, so that the marker
/// is seen to move as it does there.
pub const PACE: f32 = 3.0;
pub const LENGTH: f32 = 180.0;
pub const LINE: f32 = 120.0;

/// The frames of one crossing: the marker's run along the bar, and then a
/// wait before the next, while it is out of sight.
const RUN: u64 = (LENGTH / PACE) as u64;
const ROUND: u64 = RUN + 30;
/// The frame of a crossing the marker is on the line.
const ON_THE_LINE: u64 = (LINE / PACE) as u64;

/// How many clicks make a measurement.
pub const CLICKS: usize = 10;
/// A click further than this many frames from the marker being on the
/// line was not meant for it, and is not counted.
const FURTHEST: f64 = 20.0;
/// The clicks of a measurement are too spread to be trusted if half of
/// them are further than this many frames from the middle one.
const SPREAD: f64 = 3.0;

/// What became of a click.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Tap {
    /// It was counted, coming this many frames after the marker was on
    /// the line.
    Counted(f64),
    /// It was too far from the line to be counted.
    TooFar,
    /// It was the last of a measurement, which found this delay.
    Measured(Delay),
    /// It was the last of a measurement whose clicks were too spread.
    TooSpread,
}

/// The clicks being measured.
#[derive(Clone, Debug)]
pub struct Taps {
    /// The frame the marker set off on its first crossing.
    began: u64,
    /// How late each click of this measurement came, in frames. Those of
    /// one that is over are kept until the next click, to be shown.
    late: Vec<f64>,
}

impl Taps {
    /// The marker sets off on `frame`.
    pub fn new(frame: u64) -> Taps {
        Taps {
            began: frame,
            late: Vec::new(),
        }
    }

    /// How far along the bar the marker is on `frame`, or `None` while it
    /// waits out of sight to cross again.
    pub fn marker(&self, frame: u64) -> Option<f32> {
        let into = frame.checked_sub(self.began)? % ROUND;
        (into <= RUN).then_some(into as f32 * PACE)
    }

    /// How many clicks this measurement has so far, or had, if it is over.
    pub fn counted(&self) -> usize {
        self.late.len()
    }

    /// How late each click of this measurement came, in frames, or of the
    /// last if it is over.
    pub fn late(&self) -> &[f64] {
        &self.late
    }

    /// A click made at `at`, in frames on the clock a press is timed by,
    /// in a game played at `frame_rate` frames a second.
    pub fn tap(&mut self, at: f64, frame_rate: f64) -> Tap {
        // The crossing it was nearest the line of.
        let first = (self.began + ON_THE_LINE) as f64;
        let crossings = ((at - first) / ROUND as f64).round().max(0.0);
        let late = at - (first + crossings * ROUND as f64);
        if late.abs() > FURTHEST {
            return Tap::TooFar;
        }
        // The first click after a measurement begins the next.
        if self.late.len() == CLICKS {
            self.late.clear();
        }
        self.late.push(late);
        if self.late.len() < CLICKS {
            return Tap::Counted(late);
        }
        let middle = middle(&self.late);
        let off: Vec<f64> = self.late.iter().map(|late| (late - middle).abs()).collect();
        if self::middle(&off) > SPREAD {
            return Tap::TooSpread;
        }
        Tap::Measured(Delay::of(as_delay(middle, frame_rate)))
    }
}

/// The delay, in thousandths of a second, of a player whose clicks come
/// `late` frames after what they are aimed at was shown.
///
/// A click is taken as aimed at the frame showing `delay` before it was
/// made (see `aimed_at`). A player clicks as a frame comes up, and a click
/// meant as the marker reached the line comes about `late` frames after;
/// the half frame keeps one that is a little earlier or later than usual
/// on that same frame.
pub fn as_delay(late: f64, frame_rate: f64) -> f32 {
    ((late - 0.5) * 1000.0 / frame_rate) as f32
}

/// The middle of some numbers: the one in the middle once they are put in
/// order, or halfway between the two there for an even count.
fn middle(numbers: &[f64]) -> f64 {
    let mut sorted = numbers.to_vec();
    sorted.sort_by(f64::total_cmp);
    let half = sorted.len() / 2;
    if sorted.is_empty() {
        0.0
    } else if sorted.len() % 2 == 1 {
        sorted[half]
    } else {
        (sorted[half - 1] + sorted[half]) / 2.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::delay::aimed_at;

    /// The frames the marker is on the line, for a measurement begun on
    /// frame 100.
    fn on_the_line(crossing: u64) -> f64 {
        (100 + ON_THE_LINE + crossing * ROUND) as f64
    }

    #[test]
    fn the_marker_crosses_at_the_timing_bars_pace_and_waits_out_of_sight() {
        let taps = Taps::new(100);
        assert_eq!(taps.marker(99), None);
        assert_eq!(taps.marker(100), Some(0.0));
        assert_eq!(taps.marker(100 + ON_THE_LINE), Some(LINE));
        assert_eq!(taps.marker(100 + RUN), Some(LENGTH));
        assert_eq!(taps.marker(100 + RUN + 1), None);
        assert_eq!(taps.marker(100 + ROUND + ON_THE_LINE), Some(LINE));
    }

    #[test]
    fn ten_clicks_find_the_delay_that_puts_them_on_the_frame_they_were_meant_for() {
        let mut taps = Taps::new(100);
        // Clicks that come two and a half frames late, give or take.
        let wobble = [0.3, -0.2, 0.1, 0.0, -0.4, 0.2, 0.4, -0.1, 0.05, -0.3];
        let mut found = None;
        for (crossing, wobble) in wobble.iter().enumerate() {
            let at = on_the_line(crossing as u64) + 2.5 + wobble;
            match taps.tap(at, 60.0) {
                Tap::Counted(late) => assert!((late - 2.5 - wobble).abs() < 1e-9),
                Tap::Measured(delay) => found = Some(delay),
                other => panic!("{other:?}"),
            }
        }
        let delay = found.expect("a measurement");
        assert_eq!(delay.ms(), 34.0);
        // Every one of those clicks is taken as made on the frame the
        // marker was on the line.
        for (crossing, wobble) in wobble.iter().enumerate() {
            let at = on_the_line(crossing as u64) + 2.5 + wobble;
            let meant = on_the_line(crossing as u64) as i64;
            assert_eq!(aimed_at(at, delay.frames(60.0)), meant, "{wobble}");
        }
        // The next click begins the next measurement.
        assert_eq!(taps.counted(), CLICKS);
        taps.tap(on_the_line(20), 60.0);
        assert_eq!(taps.late(), [0.0]);
    }

    #[test]
    fn a_click_far_from_the_line_is_not_counted() {
        let mut taps = Taps::new(100);
        assert_eq!(taps.tap(on_the_line(0) + 25.0, 60.0), Tap::TooFar);
        assert_eq!(taps.tap(on_the_line(2) - 25.0, 60.0), Tap::TooFar);
        assert_eq!(taps.counted(), 0);
        // Early clicks count as early, against the crossing they were for.
        assert_eq!(taps.tap(on_the_line(3) - 2.0, 60.0), Tap::Counted(-2.0));
    }

    #[test]
    fn a_click_or_two_well_off_does_not_throw_the_measurement_out() {
        let mut taps = Taps::new(100);
        let late = [3.0, 3.0, 3.0, 15.0, 3.0, 3.0, -12.0, 3.0, 3.0, 3.0];
        let mut found = None;
        for (crossing, late) in late.iter().enumerate() {
            if let Tap::Measured(delay) = taps.tap(on_the_line(crossing as u64) + late, 60.0) {
                found = Some(delay);
            }
        }
        assert_eq!(found, Some(Delay::of(as_delay(3.0, 60.0))));
    }

    #[test]
    fn clicks_spread_too_wide_measure_nothing() {
        let mut taps = Taps::new(100);
        let late = [-8.0, 9.0, -6.0, 7.0, 0.0, -9.0, 8.0, -7.0, 6.0, 1.0];
        let mut last = None;
        for (crossing, late) in late.iter().enumerate() {
            last = Some(taps.tap(on_the_line(crossing as u64) + late, 60.0));
        }
        assert_eq!(last, Some(Tap::TooSpread));
        // A click too far off the line to count does not begin another.
        assert_eq!(taps.tap(on_the_line(20) + 30.0, 60.0), Tap::TooFar);
        assert_eq!(taps.counted(), CLICKS);
    }

    #[test]
    fn the_middle_of_some_numbers_is_the_one_in_the_middle() {
        assert_eq!(middle(&[3.0, 1.0, 2.0]), 2.0);
        assert_eq!(middle(&[4.0, 1.0, 2.0, 3.0]), 2.5);
        assert_eq!(middle(&[]), 0.0);
    }
}
