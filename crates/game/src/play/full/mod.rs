//! A full match: every innings, against another side.
//!
//! Only the player's own innings are played. The other side's are made up
//! as they come round. This keeps both sides' scores, says when the match
//! is over and who has won it, and what the board between innings has to
//! tell.

pub(crate) mod ending;
mod halves;
mod line_score;
mod names;
#[cfg(test)]
mod properties;
mod sides;
mod them;

use super::book::Book;
use super::field::Ground;
use super::paper;
use crate::rng::Rng;
use crate::rules::{FullMatchRules, StealRules};
use crate::settings::Difficulty;
pub use line_score::{
    COLUMNS, Cell, Line, Report, cells_of, hits_words, ordinal, runs_words, shown_of,
};
pub use sides::Batted;
pub(crate) use them::Them;

/// How things stand when the player's side is out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Next {
    /// The other side has batted, and the player's side comes in again.
    Bat,
    Won,
    Lost,
}

#[derive(Clone, Debug)]
pub struct FullMatch {
    /// The player's side is at home, and bats second in each innings.
    home: bool,
    rules: FullMatchRules,
    difficulty: Difficulty,
    /// How many innings' worth of runs the other side is given in each of
    /// its own.
    worth: u32,
    /// The runs each side made in each half it has finished.
    ours: Vec<u32>,
    theirs: Vec<u32>,
    /// The side batting last had won before its last half, which was never
    /// played.
    unneeded: bool,
    over: bool,
    /// What the other side's innings are drawn from.
    rng: Rng,
    /// What their innings are played out on paper from, the field they are
    /// played on, and whose turn it is to bat for them.
    seed: u64,
    ground: Ground,
    their_turn: usize,
    /// What the other side's runners steal bases by, with that mod on.
    steals: Option<StealRules>,
    /// Who the other side is, if it has a name.
    them: Option<names::Named>,
    /// Every pitch to every batter of both sides.
    pub book: Book,
}

impl FullMatch {
    /// A match about to begin, with the player's side at home or away. If
    /// it is at home the other side has batted already when this returns.
    /// `zinger` is whether every ball the player hits is a home run,
    /// `steals` what the other side's runners steal bases by, if they do,
    /// and `ground` the field the match is played on.
    pub fn new(
        home: bool,
        rules: &FullMatchRules,
        difficulty: Difficulty,
        zinger: bool,
        steals: Option<StealRules>,
        ground: Ground,
        seed: u64,
    ) -> FullMatch {
        let mut full = FullMatch {
            home,
            rules: rules.clone(),
            difficulty,
            worth: if zinger { rules.zinger_innings } else { 1 },
            ours: Vec::new(),
            theirs: Vec::new(),
            unneeded: false,
            over: false,
            rng: Rng::new(seed),
            seed,
            ground,
            their_turn: 0,
            steals,
            them: None,
            book: Book::default(),
        };
        if home {
            let made = full.made();
            full.their_half(made, false);
        }
        full
    }

    /// The other side bats for `made` runs. `winning` is whether that wins
    /// them the match, which ends the moment the last run is in. The half
    /// is played out on paper and goes in the book.
    fn their_half(&mut self, made: u32, winning: bool) {
        let wanted = paper::Wanted {
            made,
            winning,
            innings: self.theirs.len() as u32 + 1,
            first_up: self.their_turn,
        };
        self.their_turn = paper::half_into(
            &mut self.book.theirs,
            wanted,
            self.seed,
            &self.rules.their_batting,
            self.steals.as_ref(),
            &self.ground,
        );
        self.theirs.push(made);
    }

    /// The field the match is played on.
    pub fn ground(&self) -> &Ground {
        &self.ground
    }

    /// The runs the other side makes in an innings left to run its course.
    fn made(&mut self) -> u32 {
        paper::runs_wanted(&self.rules, self.difficulty, self.worth, &mut self.rng)
    }

    pub fn at_home(&self) -> bool {
        self.home
    }

    /// The runs the player's side made in the halves it has finished.
    pub fn ours(&self) -> u32 {
        self.ours.iter().sum()
    }

    /// The other side's runs.
    pub fn theirs(&self) -> u32 {
        self.theirs.iter().sum()
    }

    /// The innings the player's side is batting in, or bats in next,
    /// counting from 1.
    pub fn innings(&self) -> u32 {
        self.ours.len() as u32 + 1
    }

    /// Whether the innings in hand is the last there has to be, or one
    /// added to it.
    fn last(&self, innings: u32) -> bool {
        innings >= self.rules.innings
    }

    /// Whether the player's side, batting now, wins the match the moment it
    /// is ahead: at home, with every innings there has to be all but
    /// played.
    pub fn sudden(&self) -> bool {
        self.home && !self.over && self.last(self.innings())
    }

    /// The player's side is out, having made `runs` in this half. The
    /// other side's next half is made up, and this says how things stand
    /// after it.
    pub fn side_out(&mut self, runs: u32) -> Next {
        self.ours.push(runs);
        let innings = self.ours.len() as u32;
        let last = self.last(innings);
        let next = if self.home {
            self.after_the_bottom(innings, last)
        } else {
            self.after_the_top(last)
        };
        self.over = next != Next::Bat;
        next
    }

    /// At home, the innings is over. Unless that decides the match, the
    /// visitors bat in the top of the next.
    fn after_the_bottom(&mut self, innings: u32, last: bool) -> Next {
        let (ours, theirs) = (self.ours(), self.theirs());
        if ending::decided(last, theirs, ours) {
            return if ours > theirs { Next::Won } else { Next::Lost };
        }
        let made = self.made();
        self.their_half(made, false);
        // Ahead with only the bottom of the last innings to come, the home
        // side has no need of it.
        if ending::home_has_no_need_to_bat(self.last(innings + 1), self.theirs(), ours) {
            self.unneeded = true;
            return Next::Won;
        }
        Next::Bat
    }

    /// Away, the home side has the bottom of the innings to bat in, unless
    /// it is the last and they are ahead already.
    fn after_the_top(&mut self, last: bool) -> Next {
        let ours = self.ours();
        if ending::home_has_no_need_to_bat(last, ours, self.theirs()) {
            self.unneeded = true;
            return Next::Lost;
        }
        // In the last innings they stop as soon as they are ahead.
        let drawn = self.made();
        let (made, winning) = ending::home_makes(last, ours, self.theirs(), drawn);
        self.their_half(made, winning);
        if !ending::decided(last, ours, self.theirs()) {
            return Next::Bat;
        }
        if self.theirs() > ours {
            Next::Lost
        } else {
            Next::Won
        }
    }

    /// The player's side, batting last, has gone ahead with `runs` in this
    /// half so far. That is the match.
    pub fn walked_off(&mut self, runs: u32) {
        self.ours.push(runs);
        self.over = true;
    }

    /// Which half the player's side bats in, and of which innings.
    pub fn batting_in(&self) -> String {
        let half = if self.home { "bottom" } else { "top" };
        format!("{half} of innings {}", self.innings())
    }

    /// The same, as short as the corner of the batting view needs it.
    pub fn half_words(&self) -> String {
        let half = if self.home { "BOT" } else { "TOP" };
        format!("{half} {}", ordinal(self.innings()))
    }

    /// Every innings of both sides, for a script to read.
    pub fn describe(&self) -> String {
        let list = |made: &[u32], at_home: bool| {
            let mut all: Vec<String> = made.iter().map(u32::to_string).collect();
            if at_home && self.unneeded {
                all.push("x".to_owned());
            }
            if all.is_empty() {
                "-".to_owned()
            } else {
                all.join(" ")
            }
        };
        let (away, home) = if self.home {
            (&self.theirs, &self.ours)
        } else {
            (&self.ours, &self.theirs)
        };
        format!(
            "{}, visitors {}, home side {}",
            if self.home { "at home" } else { "away" },
            list(away, false),
            list(home, true)
        )
    }
}

#[cfg(test)]
mod tests;
