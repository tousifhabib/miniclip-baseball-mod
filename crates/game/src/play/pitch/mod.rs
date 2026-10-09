//! The pitch: where it goes, and what a swing does to it.
//!
//! Nothing here touches the art. A pitch is worked out in full before it is
//! thrown, as a list of where the ball and its shadow are on each frame, so
//! that the player can be shown where it will cross before it leaves the
//! pitcher's hand.

#[cfg(test)]
pub(crate) mod properties;
mod window;

use crate::rng::Rng;
use crate::rules::{Band, MysteryRules, PitchRules, ThrowRules};
pub use window::{Quality, meets, nearness, widened};

/// A point of the batting view, in pixels.
pub type Point = (f32, f32);

/// What a mystery pitch turns out to be.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Fastball,
    ChangeUp,
    Curve,
}

impl Kind {
    pub const ALL: [Kind; 3] = [Kind::Fastball, Kind::ChangeUp, Kind::Curve];

    /// What the player is told it was.
    pub fn words(self) -> &'static str {
        match self {
            Kind::Fastball => "FASTBALL",
            Kind::ChangeUp => "CHANGE-UP",
            Kind::Curve => "CURVE",
        }
    }

    /// Changes the table a pitch is picked from so that it is one of this
    /// kind. A curve goes to the left or to the right.
    pub fn shape(self, table: &mut PitchRules, rules: &MysteryRules, to_left: bool) {
        match self {
            Kind::Fastball => table.speed = table.speed.times(rules.fast),
            Kind::ChangeUp => table.speed = table.speed.times(rules.slow),
            Kind::Curve => {
                let way = if to_left { -1.0 } else { 1.0 };
                table.swing.base += way * rules.curve_swing;
                table.dip.base += rules.curve_dip;
            }
        }
    }
}

/// The fixed points a pitch is drawn between, taken from the art.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mound {
    /// Where the ball and its shadow are as they leave the hand.
    pub ball: Point,
    pub shadow: Point,
    /// The points the art measures the ball's journey from.
    pub ball_from: Point,
    pub shadow_from: Point,
    /// The height on screen at which the shadow passes the batter.
    pub plate: f32,
    /// The strike zone: left, top, right, bottom.
    pub zone: [f32; 4],
}

/// Where the ball and its shadow are on one frame of a pitch.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sample {
    pub ball: Point,
    pub shadow: Point,
    /// The size to draw both at, 1 being the art's own.
    pub size: f32,
    /// How solid both are, from 1 down to 0 as they fade past the batter.
    pub alpha: f32,
}

impl Sample {
    /// Whether the ball can be hit on this frame: its shadow is in the band.
    pub fn in_band(&self, band: Band) -> bool {
        self.shadow.1 > band.top && self.shadow.1 < band.bottom
    }
}

/// One pitch, worked out from the hand to past the batter.
#[derive(Clone, Debug, PartialEq)]
pub struct Pitch {
    pub samples: Vec<Sample>,
    /// Where the ball is as its shadow passes the batter.
    pub crosses: Point,
    /// Whether that is inside the strike zone.
    pub in_zone: bool,
}

/// What the pitcher has decided to throw.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Choice {
    pub speed: f32,
    pub swing: f32,
    pub dip: f32,
    pub aim: Point,
}

impl Choice {
    /// Picks a pitch for this skill level.
    pub fn pick(rules: &PitchRules, throw: &ThrowRules, rng: &mut Rng) -> Choice {
        let speed = rules.speed.low + rng.below(rules.speed.high - rules.speed.low + 1);
        let mut curve = |curve: &crate::rules::Curve| {
            if curve.over == 0.0 {
                curve.base
            } else {
                curve.base + curve.over / (rng.below(curve.parts.max(1)) + 1) as f32
            }
        };
        let (swing, dip) = (curve(&rules.swing), curve(&rules.dip));
        let target = &rules.target;
        // He aims off to allow for the curve.
        let aim = (
            target.x + rng.below(target.width as u32) as f32 - swing * throw.swing_lead,
            target.y + rng.below(target.height as u32) as f32 + dip * throw.dip_lead,
        );
        Choice {
            speed: speed as f32,
            swing,
            dip,
            aim,
        }
    }
}

impl Pitch {
    /// Works out the whole of a pitch.
    pub fn throw(choice: &Choice, mound: &Mound, rules: &ThrowRules) -> Pitch {
        let (mut ball, mut shadow) = (mound.ball, mound.shadow);
        // The whole way each has to go, to the aim and to the ground under
        // it at the batter.
        let way = (
            choice.aim.0 - mound.ball_from.0,
            choice.aim.1 - mound.ball_from.1,
        );
        let shadow_way = (
            choice.aim.0 - mound.shadow_from.0,
            mound.plate - mound.shadow_from.1,
        );
        let mut pitch = Pitch {
            samples: Vec::new(),
            crosses: choice.aim,
            in_zone: false,
        };
        let mut alpha = 1.0f32;
        let mut crossed = false;
        // No pitch takes anything like this long: it is a guard against
        // numbers in a data file that would never bring the ball in.
        for _ in 0..3000 {
            // The nearer the ball, the faster it comes on.
            let come = (shadow.1 - mound.shadow_from.1).max(0.0);
            let step = come / rules.approach / choice.speed;
            ball.0 += way.0 * step + choice.swing;
            ball.1 += way.1 * step - choice.dip;
            shadow.0 += shadow_way.0 * step + choice.swing;
            shadow.1 += shadow_way.1 * step;
            if shadow.1 >= mound.plate {
                alpha -= rules.fade;
                if !crossed {
                    crossed = true;
                    pitch.crosses = ball;
                    let [left, top, right, bottom] = mound.zone;
                    pitch.in_zone =
                        (left..=right).contains(&ball.0) && (top..=bottom).contains(&ball.1);
                }
            }
            pitch.samples.push(Sample {
                ball,
                shadow,
                size: rules.size + come * rules.growth,
                alpha: alpha.max(0.0),
            });
            if alpha <= 0.0 {
                break;
            }
        }
        pitch
    }

    /// Makes a knuckleball of the pitch: the ball sways from side to side
    /// on its way in, `sway` pixels either way as it comes by the batter
    /// and less while it is far off and small, `turns` times there and
    /// back. `start` is where in a turn it sets off, from 0 to 1. Where
    /// the pitch crosses is worked out again.
    pub fn knuckle(&mut self, sway: f32, turns: f32, start: f32, mound: &Mound) {
        let Some(crossing) = self
            .samples
            .iter()
            .position(|sample| sample.shadow.1 >= mound.plate)
            .filter(|&step| step > 0)
        else {
            return;
        };
        let full_size = self.samples[crossing].size.max(0.001);
        for (step, sample) in self.samples.iter_mut().enumerate() {
            let gone = step as f32 / crossing as f32;
            // It leaves the hand straight, and takes a moment to start.
            let eased = (gone / 0.2).min(1.0);
            let turn = (start + turns * gone) * std::f32::consts::TAU;
            let aside = sway * (sample.size / full_size) * eased * turn.sin();
            sample.ball.0 += aside;
            sample.shadow.0 += aside;
        }
        let ball = self.samples[crossing].ball;
        let [left, top, right, bottom] = mound.zone;
        self.crosses = ball;
        self.in_zone = (left..=right).contains(&ball.0) && (top..=bottom).contains(&ball.1);
    }
}

#[cfg(test)]
pub(crate) mod tests;
