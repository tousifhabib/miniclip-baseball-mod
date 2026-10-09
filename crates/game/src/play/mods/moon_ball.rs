//! Moon ball: every hit floats. It goes where it would have gone, and
//! takes several times as long over it, as you set.
//!
//! The mod keeps nothing as the game is played: how the ball flies is
//! settled in the numbers the game is played by, when it starts. The arcade
//! game is left as it is.

use crate::mods::{About, Setting};
use crate::rules::{FieldRules, MoonRules};

/// What the menu and the files know this mod by.
pub(crate) const ABOUT: About = About {
    key: "moon_ball",
    name: "MOON BALL",
    does: "EVERY HIT FLOATS: THE SAME FLIGHT, MANY TIMES SLOWER",
    setting: Some(Setting {
        name: "FLOAT",
        usual: 2,
        levels: |rules| rules.moon.slow.count(),
        words: |level, rules| format!("{}X", rules.moon.slow.at(level).unwrap_or(1.0)),
    }),
};

/// The numbers the ball flies by on the moon at this level, counting from
/// 1, given the ones it flies by as the game was: the same flight in every
/// way but the time it takes.
pub(crate) fn floated(rules: &MoonRules, level: u8, field: &FieldRules) -> FieldRules {
    let slow = rules.slow.at(level).unwrap_or(1.0).max(0.01);
    FieldRules {
        // It sets off this many times slower, along and up, and what
        // pulls it down and holds it back is as much weaker as keeps it
        // to the path it would have taken.
        pace: field.pace * slow,
        lift_share: field.lift_share / slow,
        gravity: field.gravity / (slow * slow),
        drag: field.drag / slow,
        bounce_cap: field.bounce_cap / slow,
        bounce_loss: field.bounce_loss / slow,
        ..field.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::play::field::{Ball, Contact, Happened};
    use crate::rules::Rules;

    #[test]
    fn a_moon_ball_goes_where_it_would_have_gone_and_takes_longer_over_it() {
        let rules = Rules::default();
        let (home, mark) = ((240.8, 336.85), (303.8, 168.7));
        // A well-timed hit, and one topped a little.
        for under in [0.0, -12.0] {
            let contact = Contact {
                power: 17.0,
                under,
                aside: 0.0,
            };
            let lands = |field: &FieldRules| {
                let mut ball = Ball::hit(home, mark, &contact, &rules.hit, field);
                let mut frames = 1;
                while ball.step(home, contact.miss(), field) != Happened::Landed {
                    frames += 1;
                }
                (ball.at, frames)
            };
            let (usual, quick) = lands(&rules.field);
            for (level, slow) in [(1, 1.5), (2, 2.0), (5, 5.0)] {
                let (floated, frames) = lands(&floated(&rules.moon, level, &rules.field));
                let off = (floated.0 - usual.0).hypot(floated.1 - usual.1);
                assert!(off < 6.0, "level {level}: {off} from {usual:?}");
                let longer = frames as f32 / quick as f32;
                assert!((longer - slow).abs() < 0.1, "level {level}: {longer}");
            }
        }
    }
}
