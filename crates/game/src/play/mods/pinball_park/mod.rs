//! The pinball park: a mod in which the ball loses little when it bounces,
//! and cannot get out of the field except over the wall on the fly.
//!
//! How much the ball keeps of its speed is a matter of the numbers the game
//! is played by, which the mod changes for a match. What is here is the
//! rest of it: the wall sends a ball back as a cushion would, at the angle
//! it came in at and not straight back the way it came, a ball that has
//! bounced never goes over the wall however high it hops, and the foul
//! lines are cushions too once the ball has been down.

use crate::mods::{About, Setting};
use crate::play::Parts;
use crate::play::field::{Ball, Happened, reach};
use crate::play::pitch::Point;
use crate::rules::{FieldRules, PinballRules};

/// What the menu and the files know this mod by.
pub(crate) const ABOUT: About = About {
    key: "pinball_park",
    name: "PINBALL PARK",
    does: "THE BALL BOUNCES OFF THE WALL AND THE GROUND, AND ON",
    setting: Some(Setting {
        name: "BOUNCE",
        usual: 3,
        levels: |rules| rules.pinball.keeps.count(),
        words: |level, rules| {
            format!(
                "{:.0}%",
                rules.pinball.keeps.at(level).unwrap_or(0.0) * 100.0
            )
        },
    }),
};

/// The mod, in play. It keeps nothing: how bouncy the park is is settled in
/// the rules the game is played by, when it starts.
pub(crate) struct PinballPark;

/// The numbers the ball flies by in a pinball park at this level, counting
/// from 1, given the ones it flies by as the game was.
pub(crate) fn bouncy(rules: &PinballRules, level: u8, field: &FieldRules) -> FieldRules {
    let keeps = rules.keeps.at(level).unwrap_or(field.bounce_run);
    FieldRules {
        bounce_run: keeps,
        bounce_lift: keeps,
        wall_bounce: keeps,
        bounce_cap: rules.hop,
        ..field.clone()
    }
}

/// The fixed points of the field that the ball is kept in by.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Park {
    pub home: Point,
    /// The point up the middle of the field that hits are aimed by.
    pub mark: Point,
    /// How far across the field each foul line is, at that point's
    /// distance up it.
    pub foul: (f32, f32),
}

impl Park {
    pub fn of(parts: &Parts) -> Park {
        Park {
            home: parts.home,
            mark: parts.field_mark,
            foul: parts.foul,
        }
    }
}

/// Looks at a step the ball has just taken, from `before`, and turns it
/// back if it was on its way out. Returns what happened on the step, which
/// is that it hit the wall if it was turned back by anything.
pub(crate) fn rebound(
    ball: &mut Ball,
    before: Ball,
    happened: Happened,
    park: &Park,
    rules: &FieldRules,
) -> Happened {
    let keeps = rules.wall_bounce;
    match happened {
        // On the fly, a ball over the wall is over it.
        Happened::Cleared if !before.bounced => return Happened::Cleared,
        Happened::Cleared | Happened::HitWall => {
            // The wall is wherever the field measures the same distance
            // from home, so it faces the way that distance falls fastest.
            let facing = towards_home(park.home, before.at);
            *ball = turned_back(before, facing, keeps);
            return Happened::HitWall;
        }
        Happened::Nothing | Happened::Landed => {}
    }
    if !before.bounced {
        return happened;
    }
    // The foul lines run from home through the marks the art has for them.
    let straight = (park.mark.0 - park.home.0, park.mark.1 - park.home.1);
    for line in [park.foul.0, park.foul.1] {
        let along = (line - park.home.0, park.mark.1 - park.home.1);
        // A field with no foul lines has them out of all reckoning.
        if along.0.abs() > 1.0e6 {
            continue;
        }
        let side = |point: Point| cross(along, (point.0 - park.home.0, point.1 - park.home.1));
        let fair = cross(along, straight);
        if side(ball.at) * fair < 0.0 && side(before.at) * fair >= 0.0 {
            // Square to the line, on the fair side of it.
            let square = unit((-along.1, along.0));
            let facing = if cross(along, square) * fair > 0.0 {
                square
            } else {
                (-square.0, -square.1)
            };
            let down = ball.height;
            *ball = turned_back(before, facing, keeps);
            ball.height = down;
            return Happened::HitWall;
        }
    }
    happened
}

/// The ball as it was before a step that took it out, sent back off a
/// cushion that faces `facing`, with this share of its speed.
fn turned_back(before: Ball, facing: Point, keeps: f32) -> Ball {
    let into = before.speed.0 * facing.0 + before.speed.1 * facing.1;
    let mut ball = before;
    // Only what was carrying it into the cushion is turned round.
    if into < 0.0 {
        ball.speed = (
            (before.speed.0 - 2.0 * into * facing.0) * keeps,
            (before.speed.1 - 2.0 * into * facing.1) * keeps,
        );
    }
    // It is in the field still, and can come to the wall again.
    ball.walled = false;
    ball
}

/// The way the wall faces at a point of it: the way in which the field's
/// measure of distance from home falls fastest.
fn towards_home(home: Point, at: Point) -> Point {
    let here = reach(home, at);
    let slope = (
        reach(home, (at.0 + 1.0, at.1)) - here,
        reach(home, (at.0, at.1 + 1.0)) - here,
    );
    unit((-slope.0, -slope.1))
}

fn cross(a: Point, b: Point) -> f32 {
    a.0 * b.1 - a.1 * b.0
}

fn unit(of: Point) -> Point {
    let long = (of.0 * of.0 + of.1 * of.1).sqrt().max(1e-6);
    (of.0 / long, of.1 / long)
}

#[cfg(test)]
mod tests;
