//! A tournament: full matches, one after another, against sides with
//! names, until one of them has won it.
//!
//! Everything here is sums. None of it touches what the game is drawn on,
//! so a tournament can be drawn, played out and added up with no game
//! running at all.
//!
//! A tournament is what was chosen for it, the sides in the order they
//! were drawn, and the card of every fixture played so far. Who meets whom
//! next, how the tables stand and who has won are all worked out from
//! those, so that nothing is kept twice to disagree with itself.
//!
//! The shapes a tournament comes in are in `format`, its sides and the
//! drawing of them in `sides`, who is to meet whom in `schedule`, what
//! each thing that draws numbers draws them from in `seeds`, and what a
//! side's strength does to its runs in `strength`. What is kept of a
//! fixture once it is over is its `card`, and a fixture the player has no
//! part in is played `on_paper`. How the sides of a group or a league
//! stand is in `table`, the fixtures as far as they are known in
//! `fixtures`, the playing of a tournament through in `progress`, and how
//! one is kept from one run to the next in `kept`. What it has to say for
//! itself, set out in rows for a page to write, is in `stats`, in a few
//! lines for the menu in `brief`, and in one for a script to read in
//! `describe`.

mod brief;
pub mod card;
mod describe;
pub mod fixtures;
pub mod format;
mod kept;
pub mod on_paper;
pub mod progress;
#[cfg(test)]
mod properties;
pub mod schedule;
pub mod seeds;
pub mod sides;
pub mod stats;
pub mod strength;
pub mod table;
#[cfg(test)]
mod testing;
#[cfg(test)]
mod tests;

use crate::rng::Rng;
use crate::rules::TournamentRules;
use crate::settings::Difficulty;
pub use brief::Brief;
pub use card::{Card, SideCard};
pub use fixtures::Fixture;
pub use format::{Format, Round};
pub use on_paper::{OnPaper, Paper};
pub use progress::{End, Misfit, ToPlay};
pub use sides::Entrant;
pub use table::Row;

/// What was chosen when a tournament was begun.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Setup {
    pub format: Format,
    /// The innings each side has in a match, before any that a level
    /// match adds.
    pub innings: u32,
    /// The skill level every match of it is played at.
    pub skill: Difficulty,
}

/// A tournament, as far as it has got.
#[derive(Clone, Debug, PartialEq)]
pub struct Tournament {
    setup: Setup,
    /// What its chances are all worked out from.
    seed: u64,
    /// Its sides, in the order of the draw. A side is known everywhere by
    /// its place in this.
    sides: Vec<Entrant>,
    /// The place of the player's own side.
    player: usize,
    /// The card of every fixture played so far, in the order they were
    /// played.
    played: Vec<Card>,
    /// How many fixtures the player has begun, finished or not.
    begun: u32,
}

impl Tournament {
    /// Draws a tournament. `None` if the rules have too few sides for it.
    pub fn new(
        setup: Setup,
        player: Entrant,
        rules: &TournamentRules,
        seed: u64,
    ) -> Option<Tournament> {
        let mut rng = Rng::new(seeds::of_the_draw(seed));
        let (sides, player) = sides::draw(setup.format.sides(), player, rules, &mut rng)?;
        Some(Tournament {
            setup,
            seed,
            sides,
            player,
            played: Vec::new(),
            begun: 0,
        })
    }

    pub fn setup(&self) -> Setup {
        self.setup
    }

    /// The sides, in the order of the draw.
    pub fn sides(&self) -> &[Entrant] {
        &self.sides
    }

    /// The place the player's own side has in the draw.
    pub fn player(&self) -> usize {
        self.player
    }

    /// The card of every fixture played so far, in the order they were
    /// played.
    pub fn cards(&self) -> &[Card] {
        &self.played
    }

    /// The card of the fixture with this number, if it has been played.
    pub fn card(&self, fixture: usize) -> Option<&Card> {
        self.played.iter().find(|card| card.fixture == fixture)
    }

    /// Which group a side is in, if the tournament has groups.
    pub fn group_of(&self, side: usize) -> Option<usize> {
        let format = self.setup.format;
        (0..format.groups()).find(|&group| schedule::places(format, group).contains(&side))
    }

    /// How a group stands, the side at the top first. A league is the one
    /// group it has, and a cup has none.
    pub fn table(&self, group: usize) -> Vec<Row> {
        let format = self.setup.format;
        let sides: Vec<usize> = schedule::places(format, group).collect();
        let ties = schedule::ties(format);
        // Only the group's own ties: a final between two of its sides is
        // no part of its table.
        let its_own = |card: &&Card| {
            let tie = ties.get(card.fixture);
            tie.is_some_and(|tie| tie.group == Some(group))
        };
        table::table(&sides, self.played.iter().filter(its_own))
    }
}
