//! How the match stands: how many strikes put a batter out, what a run
//! is worth on the pitch that is coming, and whether it is over.

use super::Match;
use super::phase::Outcome;
use crate::game::Game;
use crate::play::mode::Mode;
use crate::play::runners::Count;

impl Match {
    /// How many strikes put a batter out: three, unless a mod says
    /// otherwise.
    pub(crate) fn strikes_allowed(&self, game: &Game) -> u32 {
        self.mods.strikes_allowed(game.rules.count.strikes)
    }

    /// How many a run counts for on the pitch about to be thrown, which is
    /// a golden ball or is not.
    pub(super) fn worth_of_a_run(&self, golden: bool) -> u32 {
        self.mods.worth_of_a_run(golden, self.in_the_clutch())
    }

    /// Whether the pitch about to be thrown is one the clutch mod makes
    /// runs count for more on: the side has one out left, and a runner is
    /// on second or third.
    pub(super) fn in_the_clutch(&self) -> bool {
        let one_out_left = self.outs + 1 == self.max_outs;
        let runner_in_reach_of_home =
            self.runners.on_base(2).is_some() || self.runners.on_base(3).is_some();
        self.mods
            .in_the_clutch(one_out_left, runner_in_reach_of_home)
    }

    /// How the match stands, if it is over.
    pub(super) fn outcome(&self) -> Option<Outcome> {
        match &self.mode {
            Mode::Arcade(arcade) => (arcade.left == 0).then_some(Outcome::ArcadeOver),
            Mode::Full(full) => {
                // Batting last with every innings all but played, to be
                // ahead is to have won. Otherwise the side bats until it is
                // out, and what that comes to is worked out once the other
                // side has batted.
                if full.sudden() && self.score > full.theirs() {
                    Some(Outcome::Won)
                } else {
                    (self.outs >= self.max_outs).then_some(Outcome::Interval)
                }
            }
            Mode::LastInnings => {
                let level = self.target - 1;
                if self.score >= self.target {
                    Some(Outcome::Won)
                } else if self.outs < self.max_outs {
                    None
                } else if self.score == level {
                    Some(Outcome::Tied)
                } else {
                    Some(Outcome::Lost)
                }
            }
        }
    }

    /// The arcade game's points with the skill level counted in.
    pub fn arcade_score(&self, game: &Game) -> Option<u32> {
        let arcade = self.mode.arcade()?;
        let times = *game.rules.arcade.multiplier.at(game.settings.difficulty);
        Some(arcade.points * times)
    }

    /// The batter's turn is over: the next one starts with a clean count.
    pub(crate) fn clear_count(&mut self) {
        self.count = Count::default();
    }
}
