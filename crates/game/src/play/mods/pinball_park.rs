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
use crate::rules::FieldRules;

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
mod tests {
    use proptest::prelude::*;

    use super::*;
    use crate::play::field::Ground;
    use crate::rules::Rules;

    const HOME: Point = (240.8, 336.85);

    fn rolling(at: Point, speed: Point) -> Ball {
        Ball {
            at,
            speed,
            height: 0.0,
            lift: 0.0,
            fall: 0.03,
            bounced: true,
            walled: false,
        }
    }

    fn fast(of: Point) -> f32 {
        (of.0 * of.0 + of.1 * of.1).sqrt()
    }

    #[test]
    fn the_wall_faces_home() {
        // Straight up the field from home, it faces straight back down.
        let facing = towards_home(HOME, (240.8, 120.0));
        assert!(facing.1 > 0.99, "{facing:?}");
        assert!(facing.0.abs() < 0.1, "{facing:?}");
    }

    #[test]
    fn a_ball_comes_off_a_cushion_at_the_angle_it_went_in_with_less_speed() {
        // Up and to the right, into a cushion that faces straight down.
        let before = rolling((300.0, 120.0), (1.0, -2.0));
        let ball = turned_back(before, (0.0, 1.0), 0.9);
        assert_eq!(ball.at, before.at);
        assert!((ball.speed.0 - 0.9).abs() < 1e-5 && (ball.speed.1 - 1.8).abs() < 1e-5);
        assert!((fast(ball.speed) - fast(before.speed) * 0.9).abs() < 1e-5);
        assert!(!ball.walled);
        // One already going away from the cushion is left to go.
        let leaving = rolling((300.0, 120.0), (1.0, 2.0));
        assert_eq!(turned_back(leaving, (0.0, 1.0), 0.9).speed, leaving.speed);
    }

    #[test]
    fn a_ball_bouncing_about_never_gets_past_the_wall() {
        let mut rules = Rules::default().field;
        (rules.wall_bounce, rules.bounce_run, rules.bounce_lift) = (0.95, 0.95, 0.95);
        // What the art has for the middle of the field and the foul lines.
        let (mark, foul) = ((303.8, 168.7), (-54.65, 638.3));
        let mut ball = rolling((300.0, 200.0), (2.3, -1.7));
        ball.lift = 1.0;
        let (mut turned, mut furthest) = (0, 0.0f32);
        for _ in 0..3000 {
            let before = ball;
            let happened = ball.step(HOME, 0.0, &rules);
            let happened = rebound_between(&mut ball, before, happened, mark, foul, &rules);
            turned += u32::from(happened == Happened::HitWall);
            furthest = furthest.max(reach(HOME, ball.at));
        }
        assert!(turned >= 4, "{turned}");
        assert!(furthest < rules.wall + 15.0, "{furthest}");
    }

    #[test]
    fn a_ball_that_has_not_bounced_still_clears_the_wall() {
        let rules = Rules::default().field;
        let mut before = rolling((300.0, 120.0), (0.5, -1.5));
        before.bounced = false;
        before.height = 60.0;
        let mut ball = before;
        let happened = ball.step(HOME, 0.0, &rules);
        assert_eq!(happened, Happened::Cleared);
        let mark = (303.8, 168.7);
        let kept = rebound_between(&mut ball, before, happened, mark, (-54.65, 638.3), &rules);
        assert_eq!(kept, Happened::Cleared);
        // One that has bounced is turned back however high it is.
        before.bounced = true;
        let mut ball = before;
        let happened = ball.step(HOME, 0.0, &rules);
        assert_eq!(happened, Happened::Cleared);
        let kept = rebound_between(&mut ball, before, happened, mark, (-54.65, 638.3), &rules);
        assert_eq!(kept, Happened::HitWall);
        assert!(ball.speed.1 > 0.0);
    }

    /// `rebound`, for a field whose fixed points are these.
    fn rebound_between(
        ball: &mut Ball,
        before: Ball,
        happened: Happened,
        mark: Point,
        foul: (f32, f32),
        rules: &FieldRules,
    ) -> Happened {
        let park = Park {
            home: HOME,
            mark,
            foul,
        };
        rebound(ball, before, happened, &park, rules)
    }

    /// How far a point is to the fair side of each of a park's foul lines.
    /// Less than nought is over the line, in foul ground.
    fn fair_by(park: &Park, at: Point) -> [f32; 2] {
        let straight = (park.mark.0 - park.home.0, park.mark.1 - park.home.1);
        [park.foul.0, park.foul.1].map(|line| {
            let along = (line - park.home.0, park.mark.1 - park.home.1);
            let off = cross(along, (at.0 - park.home.0, at.1 - park.home.1));
            off * cross(along, straight).signum() / along.0.hypot(along.1)
        })
    }

    proptest! {
        #[test]
        fn a_ball_that_is_down_in_fair_ground_stays_between_the_lines_and_inside_the_wall(
            // Where it is: clear of the lines and short of the wall.
            (across, far) in (0.05f32..0.95, 100.0f32..780.0),
            // Which way it is going, how fast, and how hard it is on its
            // way up.
            (way, pace, lift) in (0.0f32..std::f32::consts::TAU, 0.5f32..6.0, 0.0f32..3.0),
            // The share of its speed it keeps when it bounces.
            keeps in 0.5f32..0.95,
        ) {
            let ground = Ground::default();
            let mut rules = Rules::default().field;
            (rules.wall_bounce, rules.bounce_run, rules.bounce_lift) = (keeps, keeps, keeps);
            // The field as the art has it, with the point up its middle
            // that hits are aimed by.
            let park = Park {
                home: ground.home,
                mark: (303.8, ground.mark_y),
                foul: ground.foul,
            };
            let speed = (way.cos() * pace, way.sin() * pace);
            let mut ball = rolling(ground.point(across, far), speed);
            ball.lift = lift;
            for frame in 0..3000 {
                let before = ball;
                let happened = ball.step(park.home, 0.0, &rules);
                let happened = rebound(&mut ball, before, happened, &park, &rules);
                // A ball that has bounced is never over the wall, however
                // high it hops.
                prop_assert_ne!(happened, Happened::Cleared);
                let far = reach(park.home, ball.at);
                prop_assert!(far < rules.wall, "{} off on frame {}", far, frame);
                let fair = fair_by(&park, ball.at);
                prop_assert!(fair[0] >= 0.0 && fair[1] >= 0.0, "{:?} on frame {}", fair, frame);
            }
        }
    }
}
