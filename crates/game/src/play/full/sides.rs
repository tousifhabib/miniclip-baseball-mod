//! A full match read as its two sides: the visitors and the side at home,
//! whichever of them is the player's.

use super::FullMatch;
use crate::play::book::Side;

/// One side of a full match, as it batted.
pub struct Batted<'a> {
    /// Whether it is the player's side.
    pub ours: bool,
    /// The runs it made in each half it batted in.
    pub runs: &'a [u32],
    /// Its part of the book.
    pub book: &'a Side,
}

impl FullMatch {
    fn batted(&self, ours: bool) -> Batted<'_> {
        let (runs, book) = if ours {
            (&self.ours, &self.book.ours)
        } else {
            (&self.theirs, &self.book.theirs)
        };
        Batted { ours, runs, book }
    }

    /// The side that batted first in each innings.
    pub fn visitors(&self) -> Batted<'_> {
        self.batted(!self.home)
    }

    /// The side that batted second.
    pub fn home_side(&self) -> Batted<'_> {
        self.batted(self.home)
    }

    /// Whether the side at home had won before its last half, which was
    /// never played.
    pub fn unneeded(&self) -> bool {
        self.unneeded
    }

    /// Whether the match has been played to its end.
    pub fn is_over(&self) -> bool {
        self.over
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::play::field::Ground;
    use crate::rules::Rules;
    use crate::settings::Difficulty;

    fn begun(home: bool) -> FullMatch {
        let mut rules = Rules::default().full_match;
        rules.innings = 1;
        let ground = Ground::default();
        FullMatch::new(home, &rules, Difficulty::Hard, false, None, ground, 3)
    }

    #[test]
    fn at_home_the_players_side_is_the_home_side_and_away_it_is_the_visitors() {
        let at_home = begun(true);
        assert!(at_home.home_side().ours && !at_home.visitors().ours);
        // The visitors have batted already, and the home side not yet.
        assert_eq!(at_home.visitors().runs.len(), 1);
        assert!(at_home.home_side().runs.is_empty());
        assert!(!at_home.visitors().book.turns.is_empty());
        let away = begun(false);
        assert!(away.visitors().ours && !away.home_side().ours);
    }

    #[test]
    fn a_match_is_over_when_it_has_been_won_or_lost() {
        let mut full = begun(false);
        assert!(!full.is_over() && !full.unneeded());
        // A hundred runs in the only innings are more than they can make.
        full.side_out(100);
        assert!(full.is_over() && !full.unneeded());
        assert_eq!(full.visitors().runs, [100]);
        assert_eq!(full.home_side().runs.len(), 1);
    }
}
