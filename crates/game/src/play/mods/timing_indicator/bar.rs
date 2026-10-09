//! The timing bar as it is drawn under the batter: the window, the
//! marker that crosses it, and the word on a swing.

use bb_engine::display::Path;
use bb_engine::library::Library;
use bb_engine::math::{ColorTransform, Matrix};
use bb_engine::stage::Stage;

use super::Timing;
use crate::art;
use crate::look::{self, Rgb};
use crate::play::Parts;
use crate::play::overlay::{self, DARK, Words};
use crate::play::pitch::{Pitch, Quality};
use crate::rules::PitchRules;

const BAR_COLOUR: Rgb = [0x0b, 0x3a, 0x5e];
const MARKER_COLOUR: Rgb = [0xff, 0xff, 0xff];
const MISS_COLOUR: Rgb = [0xff, 0x5a, 0x4a];

/// The figures over the bar that say how far a swing on each colour sends
/// the ball, when it is being sent for a zinger: the size of their
/// lettering, how far above the bar their tops are, and how far apart their
/// middles.
const FEET_SIZE: f32 = 0.5;
const FEET_ABOVE: f32 = 12.0;
const FEET_APART: f32 = 24.0;

/// The depth the verdict is at in the bar's clip, which is over all of the
/// bar's blocks.
const WORD_DEPTH: u16 = 100;

/// The size of the verdict's lettering, its own being 1, how far along the
/// bar its middle is, and how far above the bar its top.
const WORD_SIZE: f32 = 0.75;
const WORD_AT: f32 = 150.0;
const WORD_ABOVE: f32 = 19.0;

/// The marker's width, and how far it stands out above and below the bar.
const MARKER_WIDTH: f32 = 2.0;
const MARKER_REACH: f32 = 3.0;

/// How far along the bar the best moment to swing is.
const BEST_AT: f32 = 120.0;

/// How far the marker moves for each frame, in pixels.
const PACE: f32 = 3.0;

/// The dark edge round the bar.
const EDGE: f32 = 2.0;

/// The bar's width and height, and how far down the batting view its top
/// is, in pixels. It is centred under the plate.
const WIDTH: f32 = 180.0;
const HEIGHT: f32 = 8.0;
const TOP: f32 = 384.0;

/// The colour a swing that meets the ball this well is shown in.
pub(super) fn colour(quality: Quality) -> Rgb {
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
