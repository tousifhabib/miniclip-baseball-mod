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

use bb_engine::display::Path;
use bb_engine::library::Library;
use bb_engine::math::{ColorTransform, Matrix};
use bb_engine::stage::Stage;

use crate::art;
use crate::look::{self, Rgb};
use crate::mods::About;
use crate::play::Parts;
use crate::play::overlay::{self, DARK, Words};
use crate::play::pitch::{Pitch, Quality};
use crate::rules::PitchRules;

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

/// The bar's width and height, and how far down the batting view its top
/// is, in pixels. It is centred under the plate.
const WIDTH: f32 = 180.0;
const HEIGHT: f32 = 8.0;
const TOP: f32 = 384.0;
/// The dark edge round the bar.
const EDGE: f32 = 2.0;
/// How far the marker moves for each frame, in pixels.
const PACE: f32 = 3.0;
/// How far along the bar the best moment to swing is.
const BEST_AT: f32 = 120.0;
/// The marker's width, and how far it stands out above and below the bar.
const MARKER_WIDTH: f32 = 2.0;
const MARKER_REACH: f32 = 3.0;
/// The size of the verdict's lettering, its own being 1, how far along the
/// bar its middle is, and how far above the bar its top.
const WORD_SIZE: f32 = 0.75;
const WORD_AT: f32 = 150.0;
const WORD_ABOVE: f32 = 19.0;
/// The depth the verdict is at in the bar's clip, which is over all of the
/// bar's blocks.
const WORD_DEPTH: u16 = 100;
/// The figures over the bar that say how far a swing on each colour sends
/// the ball, when it is being sent for a zinger: the size of their
/// lettering, how far above the bar their tops are, and how far apart their
/// middles.
const FEET_SIZE: f32 = 0.5;
const FEET_ABOVE: f32 = 12.0;
const FEET_APART: f32 = 24.0;

const BAR_COLOUR: Rgb = [0x0b, 0x3a, 0x5e];
const MARKER_COLOUR: Rgb = [0xff, 0xff, 0xff];
const MISS_COLOUR: Rgb = [0xff, 0x5a, 0x4a];

/// The colour a swing that meets the ball this well is shown in.
fn colour(quality: Quality) -> Rgb {
    match quality {
        Quality::Poor => [0xe2, 0x58, 0x2a],
        Quality::MediumPoor => [0xf2, 0x9a, 0x24],
        Quality::Medium => [0xf2, 0xd0, 0x24],
        Quality::Good => [0x37, 0xd4, 0x4a],
    }
}

/// The transform that paints a block one flat colour, this solid.
fn paint(colour: Rgb, alpha: f32) -> ColorTransform {
    let mut paint = look::tint(colour);
    paint.mult[3] = alpha;
    paint
}

/// The bar on the stage, for one pitch.
pub(crate) struct Indicator {
    pub timing: Timing,
    /// The clip every part of the bar is in.
    holder: Path,
    marker: Path,
    /// The verdict.
    words: Words,
    /// How far a swing on each colour sends the ball, when that is being
    /// told.
    feet: Vec<Words>,
    /// The bar's left end.
    left: f32,
    /// The step the best moment is in the middle of. Between two steps if
    /// the best is an even number of them.
    best: f32,
    swung: bool,
}

impl Indicator {
    /// Puts the bar into a batting view that has just been built, for the
    /// pitch about to be thrown. `feet` is given when hits are being sent
    /// for zingers: how far one goes that the bat meets this many frames
    /// into the swing.
    pub fn new(
        pitch: &Pitch,
        rules: &PitchRules,
        parts: &Parts,
        feet: Option<&dyn Fn(u32) -> Option<u32>>,
        stage: &mut Stage,
        library: &Library,
    ) -> Option<Indicator> {
        let holder = overlay::holder(parts, "timingBar", stage, library)?;
        let timing = Timing::of(pitch, rules);
        let left = parts.centre_x - WIDTH / 2.0;
        // That puts the verdict on the dirt beside the plate, where it can
        // be read.
        let words = Words::new(
            &holder,
            WORD_DEPTH,
            "verdict",
            (left + WORD_AT, TOP - WORD_ABOVE),
            WORD_SIZE,
            stage,
            library,
        )?;
        // With nothing to swing for, the marker just runs out with the
        // pitch.
        let best = timing
            .best()
            .map_or(pitch.samples.len() as f32, |(first, last)| {
                (first + last) as f32 / 2.0
            });
        let mut indicator = Indicator {
            timing,
            holder,
            marker: Path::new(),
            words,
            feet: Vec::new(),
            left,
            best,
            swung: false,
        };

        let mut depth = 0;
        let mut block = |stage: &mut Stage, left: f32, top: f32, width: f32, height: f32, paint| {
            depth += 1;
            let path = stage.attach(&indicator.holder, art::BLOCK, depth, "block", library)?;
            let block = stage.child_mut(&path)?;
            block.set_matrix(Matrix {
                a: width / art::BLOCK_SIDE,
                d: height / art::BLOCK_SIDE,
                tx: left,
                ty: top,
                ..Matrix::IDENTITY
            });
            block.set_color(paint);
            Some(path)
        };
        block(
            stage,
            left - EDGE,
            TOP - EDGE,
            WIDTH + EDGE * 2.0,
            HEIGHT + EDGE * 2.0,
            paint(DARK, 0.8),
        )?;
        block(stage, left, TOP, WIDTH, HEIGHT, paint(BAR_COLOUR, 1.0))?;
        for (first, last, quality) in indicator.timing.stretches() {
            // Each step has the bar from half a step before it to half a
            // step after.
            let from = indicator.across(first as f32 - 0.5).max(left);
            let to = indicator.across(last as f32 + 0.5).min(left + WIDTH);
            if to > from {
                block(
                    stage,
                    from,
                    TOP,
                    to - from,
                    HEIGHT,
                    paint(colour(quality), 1.0),
                )?;
            }
        }
        indicator.marker = block(
            stage,
            left - MARKER_WIDTH / 2.0,
            TOP - MARKER_REACH,
            MARKER_WIDTH,
            HEIGHT + MARKER_REACH * 2.0,
            paint(MARKER_COLOUR, 1.0),
        )?;
        if let Some(feet) = feet {
            indicator.label(feet, stage, library);
        }
        Some(indicator)
    }

    /// Writes over the bar how far a swing on each of its colours sends the
    /// ball at the most, each figure in the colour it speaks for and in the
    /// order the colours come. The figure for the best is over the best.
    fn label(&mut self, feet: &dyn Fn(u32) -> Option<u32>, stage: &mut Stage, library: &Library) {
        let stretches = self.timing.stretches();
        let Some(best) = self
            .timing
            .best()
            .and_then(|(first, _)| stretches.iter().position(|&(start, ..)| start == first))
        else {
            return;
        };
        for (index, &(first, last, quality)) in stretches.iter().enumerate() {
            let most = (first..=last)
                .filter_map(|step| feet(self.timing.frames(step)?))
                .max();
            let Some(most) = most else {
                continue;
            };
            let middle = self.left + BEST_AT + (index as f32 - best as f32) * FEET_APART;
            let depth = WORD_DEPTH + 2 + 2 * index as u16;
            let top = (middle, TOP - FEET_ABOVE);
            let Some(words) = Words::new(
                &self.holder,
                depth,
                "zoneFeet",
                top,
                FEET_SIZE,
                stage,
                library,
            ) else {
                continue;
            };
            words.say(&most.to_string(), colour(quality), stage);
            self.feet.push(words);
        }
    }

    /// How far across the batting view a step of the flight is on the bar.
    fn across(&self, step: f32) -> f32 {
        self.left + BEST_AT + (step - self.best) * PACE
    }

    /// Moves the marker to where a swing made now would begin: `step` steps
    /// into the ball's flight, or that many before it if below zero. It
    /// waits at the bar's left end until the time comes, and stops at its
    /// right.
    pub fn point(&mut self, step: i32, stage: &mut Stage) {
        // Once there has been a swing the marker stays where it was made.
        if self.swung {
            return;
        }
        let at = self.across(step as f32).clamp(self.left, self.left + WIDTH);
        if let Some(marker) = stage.child_mut(&self.marker) {
            marker.move_to(at - MARKER_WIDTH / 2.0, TOP - MARKER_REACH);
        }
    }

    /// Marks a swing begun on `step`, and says how it was timed.
    pub fn swung(&mut self, step: usize, stage: &mut Stage) {
        self.point(step as i32, stage);
        self.swung = true;
        let Some(verdict) = self.timing.verdict(step) else {
            return;
        };
        let colour = self.timing.at(step).map_or(MISS_COLOUR, colour);
        self.words.say(verdict.words(), colour, stage);
        // The verdict takes the place of the figures.
        for feet in &self.feet {
            feet.hide(stage);
        }
    }

    /// Takes the bar off the stage, as the view changes to the field.
    pub fn put_away(self, stage: &mut Stage) {
        stage.remove(&self.holder);
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;
    use crate::play::pitch::properties::{any_choice, any_window};
    use crate::play::pitch::{Choice, Mound, meets};
    use crate::rules::Rules;
    use crate::settings::Difficulty;

    /// The fixed points as they are in the game's art.
    fn mound() -> Mound {
        Mound {
            ball: (286.1, 137.85),
            shadow: (286.1, 233.35),
            ball_from: (285.85, 137.8),
            shadow_from: (285.85, 230.35),
            plate: 351.0,
            zone: [246.2, 190.6, 341.1, 305.7],
        }
    }

    /// A pitch down the middle at this skill level's slowest.
    fn pitch(rules: &Rules, difficulty: Difficulty) -> Pitch {
        let choice = Choice {
            speed: rules.pitch.at(difficulty).speed.high as f32,
            swing: 0.0,
            dip: 0.0,
            aim: (295.0, 250.0),
        };
        Pitch::throw(&choice, &mound(), &rules.throw)
    }

    const LEVELS: [Difficulty; 3] = [Difficulty::Easy, Difficulty::Medium, Difficulty::Hard];

    #[test]
    fn a_swing_meets_the_ball_in_the_band_at_a_point_in_its_window() {
        let rules = Rules::default();
        for difficulty in LEVELS {
            let table = rules.pitch.at(difficulty);
            let pitch = pitch(&rules, difficulty);
            let mut met = 0;
            for step in 0..pitch.samples.len() {
                let Some((on, quality, power)) = pitch.swing_from(table, step) else {
                    continue;
                };
                met += 1;
                assert!(pitch.samples[on].in_band(table.band));
                assert_eq!(meets(table, (on - step) as u32), Some((quality, power)));
                // And on no frame before that one.
                for earlier in step..on {
                    let frames = (earlier - step) as u32;
                    assert!(
                        !pitch.samples[earlier].in_band(table.band)
                            || meets(table, frames).is_none()
                    );
                }
            }
            assert!(met > 0, "{difficulty:?}");
        }
    }

    #[test]
    fn the_steps_that_meet_the_ball_come_in_one_run_near_the_end() {
        let rules = Rules::default();
        for difficulty in LEVELS {
            let pitch = pitch(&rules, difficulty);
            let timing = Timing::of(&pitch, rules.pitch.at(difficulty));
            let stretches = timing.stretches();
            let (first, last) = (stretches[0].0, stretches.last().unwrap().1);
            for pair in stretches.windows(2) {
                assert_eq!(pair[0].1 + 1, pair[1].0, "{difficulty:?}: {stretches:?}");
            }
            assert!(first > pitch.samples.len() / 2, "{difficulty:?}");
            assert_eq!(timing.at(first - 1), None);
            assert_eq!(timing.at(last + 1), None);
            // Past the end of the pitch there is nothing to meet.
            assert_eq!(timing.at(pitch.samples.len()), None);
        }
    }

    #[test]
    fn the_best_moment_is_the_good_one_and_it_is_shorter_the_harder_the_level() {
        let rules = Rules::default();
        let best = |difficulty| {
            let pitch = pitch(&rules, difficulty);
            let timing = Timing::of(&pitch, rules.pitch.at(difficulty));
            let (first, last) = timing.best().unwrap();
            for step in first..=last {
                assert_eq!(timing.at(step), Some(Quality::Good));
            }
            last - first + 1
        };
        assert!(best(Difficulty::Easy) > best(Difficulty::Hard));
        assert_eq!(best(Difficulty::Hard), 1);
    }

    #[test]
    fn a_swing_is_judged_against_the_best_moment() {
        let rules = Rules::default();
        let pitch = pitch(&rules, Difficulty::Easy);
        let timing = Timing::of(&pitch, rules.pitch.at(Difficulty::Easy));
        let (first, last) = timing.best().unwrap();
        let stretches = timing.stretches();
        let (start, end) = (stretches[0].0, stretches.last().unwrap().1);
        assert_eq!(timing.verdict(first), Some(Verdict::Perfect));
        assert_eq!(timing.verdict(last), Some(Verdict::Perfect));
        assert_eq!(timing.verdict(0), Some(Verdict::TooEarly));
        assert_eq!(timing.verdict(end + 1), Some(Verdict::TooLate));
        // On easy a swing can be a little out either way and still meet
        // the ball.
        assert!(start < first && last < end);
        assert_eq!(timing.verdict(start), Some(Verdict::Early));
        assert_eq!(timing.verdict(end), Some(Verdict::Late));
    }

    #[test]
    fn a_pitch_no_swing_can_meet_has_no_best_moment() {
        let rules = Rules::default();
        let mut table = rules.pitch.at(Difficulty::Easy).clone();
        table.window.clear();
        let timing = Timing::of(&pitch(&rules, Difficulty::Easy), &table);
        assert_eq!(timing.best(), None);
        assert!(timing.stretches().is_empty());
        assert_eq!(timing.verdict(10), None);
    }

    /// Any pitch, and a table to swing at it by: one of the levels' own,
    /// with any window in place of its own.
    fn any_pitch() -> impl Strategy<Value = (Pitch, PitchRules)> {
        let level = prop::sample::select(&LEVELS[..]);
        (any_choice(), any_window(), level).prop_map(|(choice, window, level)| {
            let rules = Rules::default();
            let table = PitchRules {
                window,
                ..rules.pitch.at(level).clone()
            };
            (Pitch::throw(&choice, &mound(), &rules.throw), table)
        })
    }

    proptest! {
        #[test]
        fn a_step_is_coloured_on_the_bar_exactly_when_a_swing_begun_on_it_would_meet_the_ball(
            (pitch, table) in any_pitch(),
        ) {
            let timing = Timing::of(&pitch, &table);
            let stretches = timing.stretches();
            // The steps of the pitch, and a couple past the end of it.
            for step in 0..pitch.samples.len() + 2 {
                let swing = pitch.swing_from(&table, step);
                let met = swing.map(|(_, quality, _)| quality);
                // The colours the bar has for the step: one, and the right
                // one, or none.
                let coloured: Vec<Quality> = stretches
                    .iter()
                    .filter(|&&(first, last, _)| (first..=last).contains(&step))
                    .map(|&(.., quality)| quality)
                    .collect();
                prop_assert_eq!(coloured, Vec::from_iter(met), "step {}", step);
                prop_assert_eq!(timing.at(step), met);
                let frames = swing.map(|(on, ..)| (on - step) as u32);
                prop_assert_eq!(timing.frames(step), frames);
            }
            // Each stretch of colour is as long as it can be: the next is
            // further on, and if it touches it is of another colour.
            for pair in stretches.windows(2) {
                let ((_, last, colour), (first, _, next)) = (pair[0], pair[1]);
                prop_assert!(last < first && (last + 1 < first || colour != next), "{:?}", pair);
            }
        }

        #[test]
        fn the_best_moment_is_the_first_of_the_longest_stretches_that_meet_the_ball_best(
            (pitch, table) in any_pitch(),
        ) {
            let timing = Timing::of(&pitch, &table);
            let stretches = timing.stretches();
            let Some((first, last)) = timing.best() else {
                // No best moment is no moment at all.
                prop_assert!(stretches.is_empty(), "{:?}", stretches);
                return Ok(());
            };
            let top = stretches.iter().map(|&(.., quality)| quality).max();
            prop_assert!(top.is_some_and(|top| stretches.contains(&(first, last, top))));
            for (from, to, quality) in stretches {
                if Some(quality) == top {
                    // None as good is longer, and none as long is sooner.
                    prop_assert!(to - from <= last - first);
                    prop_assert!(to - from < last - first || from >= first);
                }
            }
        }
    }
}
