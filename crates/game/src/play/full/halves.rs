//! A full match as the match plays it: the half that has just closed,
//! what goes in the book, and batting again.

use bb_engine::library::Library;

use super::{FullMatch, Next};
use crate::art;
use crate::game::Game;
use crate::play::book::{End, Hit, ORDER, Pitch, Thrown};
use crate::play::{AtBat, Match, Mode, Outcome, Phase};
use crate::rng::mixed_with;

impl Match {
    /// A full match, with the player's side at home or away, in place of
    /// the last innings of one.
    pub fn new_full(game: &Game, home: bool, seed: u64, library: &Library) -> Match {
        let mut played = Match::new(game, seed, library);
        let full = FullMatch::new(
            home,
            &game.rules.full_match,
            game.settings.difficulty,
            played.mods.every_hit_is_a_home_run(),
            played
                .mods
                .runners_steal()
                .then(|| game.rules.steal.clone()),
            art::ground(library, &game.rules),
            seed ^ mixed_with::THEIR_INNINGS,
        );
        played.target = full.theirs() + 1;
        played.mode = Mode::Full(Box::new(full));
        played
    }

    /// In a full match, says who the other side is: its name in full, and
    /// in the few letters a scoreboard has room for.
    pub fn call_them(&mut self, name: &str, short: &str) {
        if let Some(full) = self.mode.full_mut() {
            full.call_them(name, short);
        }
    }

    /// The full match being played, if that is the game.
    pub fn full(&self) -> Option<&FullMatch> {
        self.mode.full()
    }

    /// In a full match, the half the player's side was batting in is over.
    /// It is written up and the other side has its turn, after which what
    /// was only a side being out may be the match won or lost.
    pub(crate) fn close_half(&mut self, outcome: Outcome) -> Outcome {
        let by_order = self.runners.runs_by_order(&self.tally);
        let left = self.runners.on_the_bases() as u32;
        let Some(full) = self.mode.full_mut() else {
            return outcome;
        };
        // The book is made up for the half: who made the runs, and how many
        // were left on base.
        let ours = &mut full.book.ours;
        ours.abandon();
        ours.left.push(left);
        for (order, runs) in ours.runs.iter_mut().enumerate() {
            *runs = by_order.get(order).copied().unwrap_or(0);
        }
        let runs = self.score.saturating_sub(full.ours());
        if outcome == Outcome::Won {
            full.walked_off(runs);
            return Outcome::Won;
        }
        match full.side_out(runs) {
            Next::Bat => Outcome::Interval,
            Next::Won => Outcome::Won,
            Next::Lost => Outcome::Lost,
        }
    }

    /// A pitch is on its way. In a full match the batter is written into
    /// the book, if he is not there already, and how things stand is kept
    /// for when it is known what came of the pitch.
    pub(crate) fn book_thrown(&mut self) {
        self.thrown_at = (self.score, self.outs);
        let order = self
            .runners
            .batter()
            .map(|batter| self.runners[batter].order);
        // A runner who has set off to steal is still the runner from his
        // base.
        let on = [1, 2, 3].map(|base| self.runners.has_one_from(base));
        if let (Some(full), Some(order)) = (self.mode.full_mut(), order) {
            let innings = full.innings();
            full.book
                .ours
                .come_up(innings, order % ORDER, self.outs, on);
        }
    }

    /// Writes the pitch into a full match's book, now that it is known how
    /// it ended.
    pub(crate) fn book_pitch(&mut self, at_bat: &AtBat, thrown: Thrown) {
        if let Some(full) = self.mode.full_mut() {
            full.book.ours.pitch(Pitch {
                in_zone: at_bat.pitch.in_zone,
                thrown,
                off: at_bat.swing_off,
                quality: at_bat.met,
            });
        }
    }

    /// The batter's turn is over: a full match's book is told how, and
    /// works out the runs that came in on the pitch and the outs that were
    /// made, a play that put out more than were left to get counting for no
    /// more than those.
    pub(crate) fn book_end(&mut self, end: End, ball: Option<Hit>) {
        let (score, outs) = self.thrown_at;
        let runs_in = self.score.saturating_sub(score);
        let outs_made = self.outs.min(self.max_outs).saturating_sub(outs);
        if let Some(full) = self.mode.full_mut() {
            full.book.ours.close(end, ball, runs_in, outs_made);
        }
    }

    /// The board has been read: the player's side comes in to bat, with
    /// nobody out and nobody on base.
    pub fn bat_again(&mut self) {
        let Some(full) = self.mode.full() else {
            return;
        };
        self.target = full.theirs() + 1;
        // What each batter made is kept under his place in the order.
        for runner in std::mem::take(&mut self.runners) {
            if self.tally.len() <= runner.order {
                self.tally.resize(runner.order + 1, 0);
            }
            self.tally[runner.order] += runner.runs;
        }
        self.outs_before += self.outs.min(self.max_outs);
        self.outs = 0;
        self.clear_count();
        self.announce = false;
        self.at = None;
        self.cues.clear();
        self.phase = Phase::Arriving;
    }
}
