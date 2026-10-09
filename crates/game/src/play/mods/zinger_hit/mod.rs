//! The zinger hit: a mod that makes a home run of every ball the bat meets.
//!
//! A swing still has to be timed to meet the ball at all. What its timing
//! no longer decides is whether the ball gets out of the ground, only by how
//! much: the nearer the swing came to the best frame of its window, the
//! further beyond the wall the ball comes down. Holding the ring on the ball
//! adds a little more. Holding it below the ball skies the hit, which then
//! hangs in the air a long time before it drops, and holding it above
//! drives the hit low and fast. To one side or the other still sends the
//! ball that way, but a hit that would have gone foul is kept inside the
//! line.
//!
//! The home run is called when the ball comes down, not when it crosses the
//! wall. Until then it is drawn large with a trail behind it, a marker
//! shows where it will land, and the distance it has gone is counted up.

mod place;
mod show;

use crate::mods::About;
use crate::play::Parts;
use crate::play::field::{Ball, Contact, distance, reach};
use crate::play::pitch::{Point, nearness};
use crate::rules::{FieldRules, HitRules, PitchRules, Rules};
use crate::settings::Difficulty;
pub(crate) use show::Show;

/// What the menu and the files know this mod by.
pub(crate) const ABOUT: About = About {
    key: "zinger_hit",
    name: "ZINGER HIT",
    does: "EVERY HIT IS A HOME RUN, BIGGER THE BETTER TIMED",
    setting: None,
};

/// The mod, in play: the longest zinger of the game, and the record its
/// zingers have to beat. What a zinger is made of is below.
#[derive(Default)]
pub(crate) struct ZingerHit {
    /// The longest zinger of this game, in feet. Nought if there has been
    /// none.
    longest: u32,
    /// The longest there has ever been, as far as this game knows.
    record: u32,
}

impl ZingerHit {
    pub fn longest(&self) -> u32 {
        self.longest
    }

    pub fn record(&self) -> u32 {
        self.record
    }

    /// Tells the game the record its zingers have to beat.
    pub fn set_record(&mut self, feet: u32) {
        self.record = feet;
    }

    /// A zinger has gone `feet`: it is counted. Returns whether it is a new
    /// record.
    pub fn count(&mut self, feet: u32) -> bool {
        self.longest = self.longest.max(feet);
        let record = feet > self.record;
        if record {
            self.record = feet;
        }
        record
    }
}

/// How far inside a foul line a zinger is kept, in pixels of the field
/// where the lines are marked.
const INSIDE: f32 = 12.0;
/// Frames of the view of the field for which a zinger is still inside the
/// wall, at the least, so that it is seen to go over.
const SEEN: f32 = 20.0;
/// How many times the height of the wall a zinger is over it by, at the
/// least, however low it was driven.
const CLEAR_BY: f32 = 1.6;

/// A ball hit for a zinger.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Zinger {
    /// How near the best the swing was timed, from 0 to 1.
    pub timed: f32,
    /// How near the ball the ring was held, from 0 to 1.
    pub aimed: f32,
    /// How far the ring's height made the hit a skied one, up to 1, or one
    /// driven low, down to -1. A ball hit level is 0.
    pub shape: f32,
    /// How far from home the ball comes down, as the field measures it, and
    /// the same with the distance to the wall as 1.
    pub carry: f32,
    pub walls: f32,
    /// The same in feet, which is what the player is told.
    pub feet: u32,
    /// Frames the ball is in the air, and how high it goes.
    pub hang: f32,
    pub peak: f32,
    /// The power it leaves the bat with in the batting view, and how far
    /// off the ball's height the ring counts as having been there.
    pub power: f32,
    tilt: f32,
}

impl Zinger {
    /// The zinger made by a swing that meets the ball this many frames
    /// after it began. `None` if it does not meet it. `ring` is how far to
    /// the right of the ball and how far below it the ring was held, and
    /// `home` is where the ball sets off from over the field.
    pub fn of(
        table: &PitchRules,
        frames_since_swing: u32,
        ring: Point,
        home: Point,
        difficulty: Difficulty,
        rules: &Rules,
    ) -> Option<Zinger> {
        let timed = nearness(table, frames_since_swing)?;
        let zinger = &rules.zinger;
        let aimed = 1.0 - (distance((0.0, 0.0), ring) / zinger.aim_reach.max(0.001)).min(1.0);
        let (worst, best) = (zinger.carry_worst, *zinger.carry_best.at(difficulty));
        let walls = worst + (best - worst - zinger.aim) * timed + zinger.aim * aimed;
        let carry = rules.field.wall * walls;

        let tilt = ring.1.clamp(-zinger.shape_reach, zinger.shape_reach);
        let shape = tilt / zinger.shape_reach.max(0.001);
        let (full, by) = if shape >= 0.0 {
            (zinger.sky, shape)
        } else {
            (zinger.drive, -shape)
        };
        let blend = |level: f32, full: f32| level + (full - level) * by;
        // How much of its flight is behind it as it comes to the wall.
        let from = reach(home, home);
        let inside = ((rules.field.wall - from) / (carry - from)).clamp(0.05, 0.95);
        Some(Zinger {
            timed,
            aimed,
            shape,
            carry,
            walls,
            feet: (zinger.wall_feet * walls).round() as u32,
            // A whole number of frames, so that it comes down where it was
            // sent and not a part of a frame further on, and enough of
            // them that the view has changed to the field before it is
            // over the wall.
            hang: (zinger.hang.at(timed) * blend(1.0, full.hang))
                .max((rules.hit.watch as f32 + SEEN) / inside)
                .round(),
            peak: blend(zinger.level_peak, full.peak)
                .max(CLEAR_BY * rules.field.clear / (4.0 * inside * (1.0 - inside))),
            power: zinger.power.at(timed),
            tilt,
        })
    }

    /// What the bat did to the ball: met it level as far as the air's drag
    /// on it goes, wherever the ring was.
    pub fn contact(&self, aside: f32) -> Contact {
        Contact {
            power: self.power,
            under: 0.0,
            aside,
        }
    }

    /// How hard the ball leaves the bat upwards in the batting view, where
    /// a skied hit is seen to go up steeply.
    pub fn lift(&self, rules: &HitRules) -> f32 {
        Contact {
            power: self.power,
            under: self.tilt,
            aside: 0.0,
        }
        .lift(rules)
    }

    /// The ball over the field, on its way towards `mark`.
    pub fn ball(&self, home: Point, mark: Point) -> Ball {
        Ball::sent(home, mark, self.carry, self.hang, self.peak)
    }

    /// The sounds of the hit: the bat, and what the crowd makes of it. The
    /// better it was timed the more they make of it.
    pub fn hit_sounds(&self) -> (&'static str, &'static [&'static str]) {
        if self.timed >= 1.0 {
            ("batHit_good", &["crowd_bigClap", "crowd_smallCheer"])
        } else if self.timed >= 0.5 {
            ("batHit_good", &["crowd_bigClap"])
        } else {
            ("batHit_medium", &["crowd_smallCheer"])
        }
    }
}

/// The nearest place to `aim`, where the art's pointer shows a hit going,
/// that sends the ball fair with room to spare.
pub(crate) fn fair(aim: f32, parts: &Parts, rules: &FieldRules) -> f32 {
    // Where the pointer is for a hit that goes this far across the field.
    let pointer = |across: f32| parts.centre_x + (across - parts.field_mark.0) * rules.aim_share;
    aim.max(pointer(parts.foul.0 + INSIDE))
        .min(pointer(parts.foul.1 - INSIDE))
}

#[cfg(test)]
mod tests;
