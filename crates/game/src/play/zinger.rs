//! The zinger hit: a mod that makes a home run of every ball the bat meets
//! in a match.
//!
//! A swing still has to be timed to meet the ball at all. What its timing
//! no longer decides is whether the ball gets out of the ground, only by how
//! much: the nearer the swing came to the best frame of its window, the
//! further beyond the wall the ball comes down. Where the ring was held
//! still sends the ball to one side or the other, but no longer lifts it or
//! drags on it, and a hit that would have gone foul is kept inside the
//! line.

use bb_engine::library::Library;
use bb_engine::stage::Stage;

use super::Parts;
use super::field::{Ball, Contact};
use super::overlay::{self, Words};
use super::pitch::{Point, nearness};
use crate::look::Rgb;
use crate::rules::{FieldRules, PitchRules, Rules};

/// How far inside a foul line a zinger is kept, in pixels of the field
/// where the lines are marked.
const INSIDE: f32 = 12.0;
/// How far down the view of the field the distance is written, which is
/// just under the home-run banner, and the size of its lettering, its own
/// being 1.
const TOLD_TOP: f32 = 230.0;
const TOLD_SIZE: f32 = 1.5;
const TOLD_COLOUR: Rgb = [0xff, 0xe2, 0x4a];

/// A ball hit for a zinger.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Zinger {
    /// How near the best the swing was timed, from 0 to 1.
    pub timed: f32,
    /// How far from home the ball comes down, as the field measures it.
    pub carry: f32,
    /// The same in feet, which is what the player is told.
    pub feet: u32,
    /// Frames the ball is in the air.
    pub hang: f32,
    /// The power it leaves the bat with in the batting view.
    pub power: f32,
}

impl Zinger {
    /// The zinger made by a swing that meets the ball this many frames
    /// after it began. `None` if it does not meet it.
    pub fn of(table: &PitchRules, frames_since_swing: u32, rules: &Rules) -> Option<Zinger> {
        let timed = nearness(table, frames_since_swing)?;
        let walls = rules.zinger.carry.at(timed);
        Some(Zinger {
            timed,
            carry: rules.field.wall * walls,
            feet: (rules.zinger.wall_feet * walls).round() as u32,
            // A whole number of them, so that it comes down where it
            // was sent and not a part of a frame further on.
            hang: rules.zinger.hang.at(timed).round(),
            power: rules.zinger.power.at(timed),
        })
    }

    /// What the bat did to the ball: met it level, wherever the ring was.
    pub fn contact(&self, aside: f32) -> Contact {
        Contact {
            power: self.power,
            under: 0.0,
            aside,
        }
    }

    /// The ball over the field, on its way towards `mark`.
    pub fn ball(&self, home: Point, mark: Point, rules: &FieldRules) -> Ball {
        Ball::sent(home, mark, self.carry, self.hang, rules)
    }

    /// Writes how far the ball went under the home-run banner. It stays up
    /// until the view is built again for the next pitch.
    pub(crate) fn tell(&self, parts: &Parts, stage: &mut Stage, library: &Library) {
        let Some(holder) = overlay::holder(parts, "zinger", stage, library) else {
            return;
        };
        let top = (parts.centre_x, TOLD_TOP);
        if let Some(words) = Words::new(&holder, 1, "zingerFeet", top, TOLD_SIZE, stage, library) {
            words.say(&format!("{} FT", self.feet), TOLD_COLOUR, stage);
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
mod tests {
    use super::*;
    use crate::play::field::{Happened, reach};
    use crate::settings::Difficulty;

    /// The fixed points of the field as they are in the game's art: home,
    /// how far up the field a hit is aimed, and the two foul lines there.
    const HOME: Point = (240.8, 336.85);
    const MARK: f32 = 168.7;
    const FOUL: (f32, f32) = (-54.65, 638.3);

    const LEVELS: [Difficulty; 3] = [Difficulty::Easy, Difficulty::Medium, Difficulty::Hard];

    /// Every zinger the rules allow: one for each frame of each level's
    /// window.
    fn zingers(rules: &Rules) -> Vec<Zinger> {
        LEVELS
            .into_iter()
            .flat_map(|difficulty| {
                let table = rules.pitch.at(difficulty);
                table
                    .window
                    .iter()
                    .map(|&(frames, ..)| Zinger::of(table, frames, rules).unwrap())
            })
            .collect()
    }

    /// Places across the field to send a ball, from one foul line to the
    /// other.
    fn across() -> impl Iterator<Item = f32> {
        (0..=20).map(|step| FOUL.0 + (FOUL.1 - FOUL.0) * step as f32 / 20.0)
    }

    #[test]
    fn a_miss_is_no_zinger() {
        let rules = Rules::default();
        let easy = rules.pitch.at(Difficulty::Easy);
        assert_eq!(Zinger::of(easy, 6, &rules), None);
        assert!(Zinger::of(easy, 7, &rules).is_some());
    }

    #[test]
    fn every_zinger_clears_the_wall_wherever_it_is_sent() {
        let rules = Rules::default();
        for zinger in zingers(&rules) {
            for across in across() {
                let mut ball = zinger.ball(HOME, (across, MARK), &rules.field);
                let mut frames = 1;
                let happened = loop {
                    match ball.step(HOME, 0.0, &rules.field) {
                        Happened::Nothing => frames += 1,
                        happened => break happened,
                    }
                };
                assert_eq!(happened, Happened::Cleared, "{zinger:?} towards {across}");
                // The view has changed to the field by then, so that the
                // ball is seen to go.
                assert!(frames > rules.hit.watch, "{zinger:?} towards {across}");
            }
        }
    }

    #[test]
    fn every_zinger_is_down_before_the_home_run_has_been_shown() {
        let rules = Rules::default();
        for zinger in zingers(&rules) {
            for across in across() {
                let mut ball = zinger.ball(HOME, (across, MARK), &rules.field);
                let mut over_wall = None;
                let mut frames = 1;
                while ball.step(HOME, 0.0, &rules.field) != Happened::Landed {
                    if ball.walled && over_wall.is_none() {
                        over_wall = Some(frames);
                    }
                    frames += 1;
                }
                let since = frames - over_wall.unwrap();
                assert!(since < 116, "{zinger:?} towards {across}: {since}");
                let far = reach(HOME, ball.at);
                assert!((far - zinger.carry).abs() < 1.0, "{zinger:?}: {far}");
            }
        }
    }

    #[test]
    fn the_nearer_the_best_the_swing_the_further_the_ball_goes() {
        let rules = Rules::default();
        for difficulty in LEVELS {
            let table = rules.pitch.at(difficulty);
            let mut all: Vec<Zinger> = table
                .window
                .iter()
                .map(|&(frames, ..)| Zinger::of(table, frames, &rules).unwrap())
                .collect();
            all.sort_by(|a, b| a.timed.total_cmp(&b.timed));
            for pair in all.windows(2) {
                if pair[0].timed < pair[1].timed {
                    assert!(pair[0].feet < pair[1].feet, "{difficulty:?}: {pair:?}");
                    assert!(pair[0].carry < pair[1].carry, "{difficulty:?}: {pair:?}");
                } else {
                    assert_eq!(pair[0].feet, pair[1].feet);
                }
            }
            // From just over the wall to twice as far.
            let (worst, best) = (all[0], all[all.len() - 1]);
            assert_eq!((worst.timed, best.timed), (0.0, 1.0));
            assert_eq!((worst.feet, best.feet), (440, 800));
            assert!(worst.carry > rules.field.wall);
        }
    }

    #[test]
    fn where_the_ring_was_held_neither_lifts_the_ball_nor_drags_on_it() {
        let rules = Rules::default();
        let zinger = Zinger::of(rules.pitch.at(Difficulty::Easy), 11, &rules).unwrap();
        let contact = zinger.contact(30.0);
        assert_eq!(
            (contact.under, contact.miss(), contact.aside),
            (0.0, 0.0, 30.0)
        );
    }
}
