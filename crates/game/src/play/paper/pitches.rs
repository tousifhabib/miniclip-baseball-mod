//! One batter's pitches on paper: balls and strikes until he walks, is
//! struck out or puts the ball in play.

use super::play::Play;
use crate::play::book::{Pitch, Thrown};
use crate::rng::Rng;

/// How much of that the singles take, and the home runs, the doubles and
/// triples taking it as it is.
const SINGLES_LEAN: f32 = 0.3;
const HOME_RUNS_LEAN: f32 = 1.8;

/// How many pitches a batter may see before the next he swings at is put in
/// play.
const MOST_PITCHES: usize = 12;

/// What a ball put in play comes to, before it is known what the runners
/// do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Struck {
    GroundOut,
    FlyOut,
    Single,
    Double,
    Triple,
    HomeRun,
}

/// Why a playing of an innings was no good.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Miss {
    TooFew,
    TooMany,
}

impl Play<'_> {
    /// The pitches of a turn, and what the last was struck for if it was
    /// put in play.
    pub(super) fn pitches(&self, rng: &mut Rng) -> (Vec<Pitch>, Option<Struck>) {
        let rules = self.rules;
        let mut pitches = Vec::new();
        let (mut balls, mut strikes) = (0, 0);
        loop {
            let in_zone = rng.chance(rules.zone);
            let which = usize::from(!in_zone);
            let tired = pitches.len() >= MOST_PITCHES;
            let swung = tired || rng.chance(rules.swing[which]);
            let thrown = if !swung {
                if in_zone {
                    Thrown::Called
                } else {
                    Thrown::Ball
                }
            } else if !tired && !rng.chance(rules.contact[which]) {
                Thrown::Swinging
            } else if !tired && rng.chance(rules.foul) {
                Thrown::Foul
            } else {
                Thrown::InPlay
            };
            pitches.push(Pitch {
                in_zone,
                thrown,
                off: None,
                quality: None,
            });
            match thrown {
                Thrown::Ball => balls += 1,
                Thrown::Called | Thrown::Swinging => strikes += 1,
                // A foul is a strike, but never the last one.
                Thrown::Foul => strikes = (strikes + 1).min(2),
                Thrown::InPlay => return (pitches, Some(self.struck(rng))),
            }
            if balls == 4 || strikes == 3 {
                return (pitches, None);
            }
        }
    }

    /// What a ball put in play comes to.
    fn struck(&self, rng: &mut Rng) -> Struck {
        let rules = self.rules;
        // Runs in number come of long hits more than of many short ones.
        let lean = |by: f32| self.lean.powf(by);
        let chances = [
            (Struck::GroundOut, rules.ground_out),
            (Struck::FlyOut, rules.fly_out),
            (Struck::Single, rules.single * lean(SINGLES_LEAN)),
            (Struck::Double, rules.double * lean(1.0)),
            (Struck::Triple, rules.triple * lean(1.0)),
            (Struck::HomeRun, rules.home_run * lean(HOME_RUNS_LEAN)),
        ];
        let all: f32 = chances.iter().map(|(_, chance)| chance.max(0.0)).sum();
        let mut left = rng.unit() * all;
        for (struck, chance) in chances {
            left -= chance.max(0.0);
            if left < 0.0 {
                return struck;
            }
        }
        Struck::GroundOut
    }

    /// Four balls: the batter takes first, and everyone he pushes moves up.
    pub(super) fn walk(&mut self, order: usize, home: &mut Vec<usize>) {
        let mut coming = Some(order);
        for base in &mut self.bases {
            match coming {
                Some(_) => coming = std::mem::replace(base, coming),
                None => break,
            }
        }
        home.extend(coming);
    }
}
