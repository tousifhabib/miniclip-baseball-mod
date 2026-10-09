//! What the mods say about the score: how many strikes put a batter out,
//! what a run is worth, and what an out or a batter on base does to that.

use super::ModsInPlay;
use crate::play::mods::Line;
use crate::play::mods::clutch::Clutch;
use crate::play::mods::rally::Rally;
use crate::play::mods::sudden_death::SuddenDeath;

impl ModsInPlay {
    /// How many strikes put a batter out: the `usual` number, unless a mod
    /// says otherwise.
    pub fn strikes_allowed(&self, usual: u32) -> u32 {
        self.sudden_death
            .as_ref()
            .map_or(usual, SuddenDeath::strikes)
    }

    /// Whether the pitch with this number, the first being 1, is a golden
    /// ball.
    pub fn is_golden(&self, pitch: u32) -> bool {
        self.golden_ball
            .as_ref()
            .is_some_and(|golden| golden.is_gold(pitch))
    }

    /// Whether a pitch thrown now is in the clutch, given how the game
    /// stands.
    pub fn in_the_clutch(&self, one_out_left: bool, runner_in_reach_of_home: bool) -> bool {
        self.clutch.is_some() && Clutch::is_now(one_out_left, runner_in_reach_of_home)
    }

    /// Settles whether the pitch in hand is thrown in the clutch, and
    /// returns what the corner of the view says if it is.
    pub fn settle_the_clutch(&mut self, in_it: bool) -> Option<Line> {
        let clutch = self.clutch.as_mut()?;
        clutch.this_pitch = in_it;
        in_it.then(|| clutch.line())
    }

    /// Whether the pitch in hand was thrown in the clutch.
    pub fn clutch_this_pitch(&self) -> bool {
        self.clutch.as_ref().is_some_and(|clutch| clutch.this_pitch)
    }

    /// How many times over a run counts on the pitch about to be thrown,
    /// which is a golden ball or is not, and in the clutch or is not. Each
    /// mod that makes runs worth more multiplies what the others make of
    /// them.
    pub fn worth_of_a_run(&self, golden: bool, in_the_clutch: bool) -> u32 {
        let for_gold = match &self.golden_ball {
            Some(ball) if golden => ball.runs(),
            _ => 1,
        };
        let for_the_clutch = match &self.clutch {
            Some(clutch) if in_the_clutch => clutch.runs(),
            _ => 1,
        };
        for_gold
            * self.sudden_death.as_ref().map_or(1, SuddenDeath::runs)
            * self.rally.as_ref().map_or(1, Rally::worth)
            * for_the_clutch
    }

    /// Somebody has been put out: at the plate, or on the bases.
    pub fn somebody_is_out(&mut self) {
        if let Some(rally) = &mut self.rally {
            rally.broken();
        }
    }

    /// The batter has got to a base, or all the way round.
    pub fn the_batter_reached_base(&mut self) {
        if let Some(rally) = &mut self.rally {
            rally.kept_up();
        }
    }

    /// What the corner of the view says of a rally, while one is on.
    pub fn rally_line(&self) -> Option<Line> {
        self.rally.as_ref().and_then(Rally::line)
    }

    /// How many batters in a row have reached base, as far as it counts.
    pub fn in_a_row(&self) -> u32 {
        self.rally.as_ref().map_or(0, Rally::in_a_row)
    }
}
