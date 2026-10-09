//! The mods that are in play in a match.
//!
//! Each mod has a file of its own here, named as the mod is: what it keeps
//! from one pitch to the next, its rules, and what it draws. The game does
//! not ask whether a mod is switched on. It asks the mods a question, or
//! tells them that something has happened, and each of those is a function
//! below that names the mods with a say in it, in the order they have it.
//! A mod that is off is not here at all, and so has nothing to say.

mod clutch;
mod golden_ball;
mod heat_check;
mod rally;
mod sudden_death;

use clutch::Clutch;
pub(crate) use golden_ball::GoldenBall;
use heat_check::HeatCheck;
use rally::Rally;
use sudden_death::SuddenDeath;

use crate::look::Rgb;
use crate::menu::Game;
use crate::mods::Mod;
use crate::rules::PitchRules;

/// A line a mod writes in the corner of the batting view: what it is
/// called on the stage, what it says, and in what colour.
pub(crate) struct Line {
    pub name: &'static str,
    pub words: String,
    pub colour: Rgb,
}

/// What each mod in play keeps. `None` is a mod that is off.
#[derive(Default)]
pub(crate) struct ModsInPlay {
    clutch: Option<Clutch>,
    golden_ball: Option<GoldenBall>,
    heat_check: Option<HeatCheck>,
    rally: Option<Rally>,
    sudden_death: Option<SuddenDeath>,
}

impl ModsInPlay {
    /// The mods for a game that is about to start. Which are on does not
    /// change while a game is being played. `arcade` is whether it is the
    /// arcade game, which has no runs, outs, runners or fielders, and so no
    /// place for the mods that act on those.
    pub fn for_game(game: &Game, arcade: bool) -> ModsInPlay {
        let on = |which: Mod| game.mods.is_on(which);
        let rules = &game.rules;
        ModsInPlay {
            clutch: (on(Mod::Clutch) && !arcade).then(|| Clutch::new(&rules.clutch)),
            golden_ball: (on(Mod::GoldenBall) && !arcade).then(|| GoldenBall::new(&rules.golden)),
            heat_check: on(Mod::HeatCheck).then(|| HeatCheck::new(&rules.heat)),
            rally: (on(Mod::Rally) && !arcade).then(|| Rally::new(&rules.rally)),
            sudden_death: on(Mod::SuddenDeath).then(|| SuddenDeath::new(&rules.sudden_death)),
        }
    }

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

    /// Takes in the runs scored since the last pitch and makes the coming
    /// one faster by the heat that is on. Returns what the corner of the
    /// view says of it.
    pub fn heat_the_pitch(&mut self, score: u32, table: &mut PitchRules) -> Option<Line> {
        let heat = self.heat_check.as_mut()?;
        heat.warm(score, table);
        heat.line()
    }

    /// How much heat is on.
    pub fn heat(&self) -> u32 {
        self.heat_check.as_ref().map_or(0, HeatCheck::heat)
    }

    /// A strike has been called on the batter, swung at or not.
    pub fn a_strike_was_called(&mut self) {
        self.cool();
    }

    /// A foul has counted as a strike against the batter.
    pub fn a_foul_took_a_strike(&mut self) {
        self.cool();
    }

    fn cool(&mut self) {
        if let Some(heat) = &mut self.heat_check {
            heat.cool();
        }
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
