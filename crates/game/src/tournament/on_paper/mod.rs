//! A whole match on paper: a fixture neither side of which is the
//! player's.
//!
//! Each side makes its runs as the other side of a full match does, by
//! the skill level's chances leant by its own strength, and has each half
//! played out on paper to come to them. The match ends as a match does.

use super::card::{Card, SideCard};
use super::{seeds, strength};
use crate::play::book::Side;
use crate::play::field::Ground;
use crate::play::full::ending;
use crate::play::paper;
use crate::rng::Rng;
use crate::rules::{FullMatchRules, Rules, StealRules};
use crate::settings::Difficulty;

/// How many innings past the ones it has to have a match on paper may go
/// before a coin settles it. Sides that made the same in every innings
/// would otherwise never be done.
const MOST_MORE: u32 = 30;

/// What the mods that are on do to a match on paper. They are the ones
/// that change what the other side of a full match does.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OnPaper {
    /// With every ball the player hits a home run, the other side is
    /// given more innings' worth of runs in each of its own, and so is
    /// each side here.
    pub every_hit_is_a_home_run: bool,
    /// Runners try for a base now and then.
    pub runners_steal: bool,
}

/// What a match on paper is played by.
#[derive(Clone, Copy, Debug)]
pub struct Paper<'a> {
    pub rules: &'a Rules,
    pub skill: Difficulty,
    /// The innings each side has, before any that a level match adds.
    pub innings: u32,
    pub mods: OnPaper,
    /// The field it is played on.
    pub ground: &'a Ground,
}

/// One side of the match as it goes.
struct Batting {
    side: usize,
    /// A full match's rules, with its runs leant by the side's strength.
    rules: FullMatchRules,
    /// What its runs are drawn from, and what its halves are played out
    /// from.
    rng: Rng,
    seed: u64,
    runs: Vec<u32>,
    book: Side,
    up: usize,
}

impl Batting {
    fn new(side: (usize, f32), at_home: bool, by: &Paper<'_>, seed: u64) -> Batting {
        let (side, strength) = side;
        let seed = seeds::of_a_side_on_paper(seed, at_home);
        Batting {
            side,
            rules: strength::match_rules(&by.rules.full_match, strength, by.innings),
            rng: Rng::new(seed),
            seed,
            runs: Vec::new(),
            book: Side::default(),
            up: 0,
        }
    }

    fn total(&self) -> u32 {
        self.runs.iter().sum()
    }

    /// The runs its next half comes to if it is left to run its course.
    fn drawn(&mut self, by: &Paper<'_>) -> u32 {
        let worth = if by.mods.every_hit_is_a_home_run {
            self.rules.zinger_innings
        } else {
            1
        };
        paper::runs_wanted(&self.rules, by.skill, worth, &mut self.rng)
    }

    /// Bats for `made` runs, which win the match there and then if
    /// `winning`.
    fn bat(&mut self, made: u32, winning: bool, steals: Option<&StealRules>, ground: &Ground) {
        let wanted = paper::Wanted {
            made,
            winning,
            innings: self.runs.len() as u32 + 1,
            first_up: self.up,
        };
        let batting = &self.rules.their_batting;
        self.up = paper::half_into(&mut self.book, wanted, self.seed, batting, steals, ground);
        self.runs.push(made);
    }

    fn card(&self) -> SideCard {
        SideCard::of(self.side, &self.runs, &self.book, paper::OUTS)
    }
}

/// Plays the fixture with this number on paper, between a side at home
/// and a side away, each known by its place in the draw and its strength,
/// and gives its card. `seed` is the fixture's own.
pub fn played(
    fixture: usize,
    home: (usize, f32),
    away: (usize, f32),
    by: &Paper<'_>,
    seed: u64,
) -> Card {
    let mut home = Batting::new(home, true, by, seed);
    let mut away = Batting::new(away, false, by, seed);
    let steals = by.mods.runners_steal.then_some(&by.rules.steal);
    let mut unneeded = false;
    for innings in 1.. {
        let last = innings >= by.innings;
        if innings > by.innings + MOST_MORE {
            // Still level, and it has gone on long enough: a coin gives
            // one of them the run that wins it.
            let visitors = home.rng.below(2) == 0;
            away.bat(u32::from(visitors), false, steals, by.ground);
            home.bat(u32::from(!visitors), !visitors, steals, by.ground);
            break;
        }
        let drawn = away.drawn(by);
        away.bat(drawn, false, steals, by.ground);
        if ending::home_has_no_need_to_bat(last, away.total(), home.total()) {
            unneeded = true;
            break;
        }
        let drawn = home.drawn(by);
        let (made, winning) = ending::home_makes(last, away.total(), home.total(), drawn);
        home.bat(made, winning, steals, by.ground);
        if ending::decided(last, away.total(), home.total()) {
            break;
        }
    }
    Card {
        fixture,
        home: home.card(),
        away: away.card(),
        unneeded,
    }
}

#[cfg(test)]
mod tests;
