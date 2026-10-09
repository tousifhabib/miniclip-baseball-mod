//! The mods that are in play in a match.
//!
//! Each mod has a file of its own here, named as the mod is: what it keeps
//! from one pitch to the next, its rules, and what it draws. The game does
//! not ask whether a mod is switched on. It asks the mods a question, or
//! tells them that something has happened, and each of those is a function
//! below that names the mods with a say in it, in the order they have it.
//! A mod that is off is not here at all, and so has nothing to say.

mod golden_ball;
mod sudden_death;

pub(crate) use golden_ball::GoldenBall;
use sudden_death::SuddenDeath;

use crate::look::Rgb;
use crate::menu::Game;
use crate::mods::Mod;

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
    golden_ball: Option<GoldenBall>,
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
            golden_ball: (on(Mod::GoldenBall) && !arcade).then(|| GoldenBall::new(&rules.golden)),
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

    /// How many times over a run counts on a pitch that is a golden ball or
    /// is not, for the mods that are in play here. Each mod that makes runs
    /// worth more multiplies what the others make of them.
    pub fn worth_of_a_run(&self, golden: bool) -> u32 {
        let for_gold = match &self.golden_ball {
            Some(ball) if golden => ball.runs(),
            _ => 1,
        };
        for_gold * self.sudden_death.as_ref().map_or(1, SuddenDeath::runs)
    }
}
