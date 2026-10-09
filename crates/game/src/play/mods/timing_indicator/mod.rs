//! The timing indicator: a bar in the batting view that shows when to swing.
//!
//! A swing meets the ball only if it began the right number of frames
//! before the ball comes by, and a pitch is worked out in full before it is
//! thrown. So what a swing begun on each frame of a pitch would come to is
//! known before the ball leaves the pitcher's hand, and the bar can show it:
//! the frames are laid out from left to right, the ones that meet the ball
//! are coloured by how well, and a marker runs along them as the frames go
//! by. The best moment is always at the same place on the bar and the marker
//! always moves at the same pace, whatever the pitch.

mod bar;

use crate::mods::About;
use crate::play::pitch::{Pitch, Quality};
use crate::rules::PitchRules;
pub(crate) use bar::Indicator;

/// What the menu and the files know this mod by.
pub(crate) const ABOUT: About = About {
    key: "timing_indicator",
    name: "TIMING INDICATOR",
    does: "A BAR THAT SHOWS WHEN TO SWING",
    setting: None,
};

/// The mod, in play. It keeps nothing from pitch to pitch: the bar is put up
/// afresh for each.
pub(crate) struct TimingIndicator;

/// What a swing begun on each step of a pitch's flight comes to.
#[derive(Clone, Debug, PartialEq)]
pub struct Timing {
    /// For each step, how well a swing begun on it meets the ball, and how
    /// many frames into the swing. `None` is a miss.
    by_step: Vec<Option<(Quality, u32)>>,
    /// The steps to swing on for the best this pitch allows: the first and
    /// the last.
    best: Option<(usize, usize)>,
}

/// What the bar says of a swing: where it came against the best moment, and
/// whether it met the ball at all.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// Before the best moment, and a miss.
    TooEarly,
    /// Before the best moment, but the ball was met.
    Early,
    Perfect,
    Late,
    TooLate,
}

impl Verdict {
    fn words(self) -> &'static str {
        match self {
            Verdict::TooEarly => "TOO EARLY",
            Verdict::Early => "EARLY",
            Verdict::Perfect => "PERFECT",
            Verdict::Late => "LATE",
            Verdict::TooLate => "TOO LATE",
        }
    }
}

impl Timing {
    pub fn of(pitch: &Pitch, rules: &PitchRules) -> Timing {
        let by_step = (0..pitch.samples.len())
            .map(|step| {
                let (on, quality, _) = pitch.swing_from(rules, step)?;
                Some((quality, (on - step) as u32))
            })
            .collect();
        let mut timing = Timing {
            by_step,
            best: None,
        };
        // The longest stretch of the best the pitch allows, and the first
        // such if there are two as long.
        let top = timing.stretches().into_iter().map(|(.., quality)| quality);
        if let Some(top) = top.max() {
            timing.best = timing
                .stretches()
                .into_iter()
                .filter(|&(.., quality)| quality == top)
                .rev()
                .max_by_key(|&(first, last, _)| last - first)
                .map(|(first, last, _)| (first, last));
        }
        timing
    }

    /// How well a swing begun on `step` meets the ball, if it does.
    pub fn at(&self, step: usize) -> Option<Quality> {
        self.met(step).map(|(quality, _)| quality)
    }

    /// How many frames into a swing begun on `step` the bat meets the ball,
    /// if it does.
    pub fn frames(&self, step: usize) -> Option<u32> {
        self.met(step).map(|(_, frames)| frames)
    }

    fn met(&self, step: usize) -> Option<(Quality, u32)> {
        self.by_step.get(step).copied().flatten()
    }

    /// The first and last of the steps to swing on for the best this pitch
    /// allows. `None` if no swing can meet it.
    pub fn best(&self) -> Option<(usize, usize)> {
        self.best
    }

    /// The runs of steps that meet the ball equally well: the first step,
    /// the last, and how well.
    pub fn stretches(&self) -> Vec<(usize, usize, Quality)> {
        let mut stretches: Vec<(usize, usize, Quality)> = Vec::new();
        for (step, met) in self.by_step.iter().enumerate() {
            let Some((quality, _)) = *met else {
                continue;
            };
            match stretches.last_mut() {
                Some((_, last, same)) if *last + 1 == step && *same == quality => *last = step,
                _ => stretches.push((step, step, quality)),
            }
        }
        stretches
    }

    /// What to say of a swing begun on `step`. `None` if no swing could
    /// have met this pitch, so that there was no moment to be early or late
    /// for.
    pub fn verdict(&self, step: usize) -> Option<Verdict> {
        let (first, last) = self.best?;
        let met = self.at(step).is_some();
        Some(if step < first {
            if met {
                Verdict::Early
            } else {
                Verdict::TooEarly
            }
        } else if step > last {
            if met { Verdict::Late } else { Verdict::TooLate }
        } else {
            Verdict::Perfect
        })
    }
}

#[cfg(test)]
mod tests;
