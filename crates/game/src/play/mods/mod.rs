//! The mods that are in play in a match.
//!
//! Each mod has a file of its own here, named as the mod is: what it keeps
//! from one pitch to the next, its rules, and what it draws. The game does
//! not ask whether a mod is switched on. It asks the mods a question, or
//! tells them that something has happened, and each of those is a function
//! below that names the mods with a say in it, in the order they have it.
//! A mod that is off is not here at all, and so has nothing to say.

mod sudden_death;

use sudden_death::SuddenDeath;

use crate::menu::Game;
use crate::mods::Mod;

/// What each mod in play keeps. `None` is a mod that is off.
#[derive(Default)]
pub(crate) struct ModsInPlay {
    sudden_death: Option<SuddenDeath>,
}

impl ModsInPlay {
    /// The mods for a game that is about to start. Which are on does not
    /// change while a game is being played.
    pub fn for_game(game: &Game) -> ModsInPlay {
        let on = |which: Mod| game.mods.is_on(which);
        let rules = &game.rules;
        ModsInPlay {
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

    /// How many times over a run counts, for the mods that are in play
    /// here.
    pub fn worth_of_a_run(&self) -> u32 {
        self.sudden_death.as_ref().map_or(1, SuddenDeath::runs)
    }
}
