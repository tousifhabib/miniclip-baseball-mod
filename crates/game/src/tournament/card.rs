//! The card of a finished fixture: what is kept of a match once it is
//! over, which is enough to say who won it and to add its figures to
//! every other match's.
//!
//! A card has each side's runs innings by innings and the figures of each
//! of its nine places in the order. It has not every pitch, so where each
//! ball came down, how each swing was timed and what each batter did turn
//! by turn are not to be had from it.

use serde::{Deserialize, Serialize};

use crate::play::book::{Figures, ORDER, Side};
use crate::play::full::FullMatch;
use crate::play::paper;

/// What one side did in a fixture.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SideCard {
    /// Which side it is, by its place in the draw.
    pub side: usize,
    /// The runs it made in each half it batted in.
    pub runs: Vec<u32>,
    /// What each place in its order did.
    pub places: [Figures; ORDER],
    /// The runners it left on base.
    pub left: u32,
    /// The errors it made in the field.
    pub errors: u32,
    /// How many of it were put out, and how many outs it had in an
    /// innings.
    pub outs: u32,
    pub outs_an_innings: u32,
}

impl SideCard {
    /// The card of a side that made `made` in its halves, from its part of
    /// the match's book.
    pub fn of(side: usize, made: &[u32], book: &Side, outs_an_innings: u32) -> SideCard {
        SideCard {
            side,
            runs: made.to_vec(),
            places: std::array::from_fn(|order| book.figures_of(order)),
            left: book.left.iter().sum(),
            errors: book.errors,
            outs: book.outs(),
            outs_an_innings,
        }
    }

    /// The runs it made in the match. These are its score, which runs
    /// that were no batter's may be part of, and not the runs of its
    /// batters added up.
    pub fn total(&self) -> u32 {
        self.runs.iter().sum()
    }

    /// The figures of the whole side.
    pub fn figures(&self) -> Figures {
        let mut all: Figures = self.places.iter().copied().sum();
        all.left = self.left;
        all
    }
}

/// The card of a fixture: the side at home and the visitors.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Card {
    /// The number the fixture has in its tournament.
    pub fixture: usize,
    pub home: SideCard,
    pub away: SideCard,
    /// The side at home had won before its last half, which was never
    /// played.
    pub unneeded: bool,
}

impl Card {
    /// The card of a full match that has been played to its end, between
    /// the player's side and another, each known by its place in the draw.
    /// `our_outs` is how many outs the player's side had in an innings.
    /// `None` if the match is not over.
    pub fn of(
        full: &FullMatch,
        fixture: usize,
        ours: usize,
        theirs: usize,
        our_outs: u32,
    ) -> Option<Card> {
        if !full.is_over() {
            return None;
        }
        let (home, away) = (full.home_side(), full.visitors());
        // The other side's innings were played on paper, by the outs a
        // side has there.
        let card = |batted: &crate::play::full::Batted<'_>| {
            let (side, outs) = if batted.ours {
                (ours, our_outs)
            } else {
                (theirs, paper::OUTS)
            };
            SideCard::of(side, batted.runs, batted.book, outs)
        };
        Some(Card {
            fixture,
            home: card(&home),
            away: card(&away),
            unneeded: full.unneeded(),
        })
    }

    /// Which side won, by its place in the draw.
    pub fn winner(&self) -> usize {
        if self.home.total() > self.away.total() {
            self.home.side
        } else {
            self.away.side
        }
    }

    /// Which side lost.
    pub fn loser(&self) -> usize {
        if self.winner() == self.home.side {
            self.away.side
        } else {
            self.home.side
        }
    }

    /// One side's part of the card, if it played in the fixture.
    pub fn of_side(&self, side: usize) -> Option<&SideCard> {
        [&self.home, &self.away]
            .into_iter()
            .find(|card| card.side == side)
    }

    /// How many innings the match went to.
    pub fn innings(&self) -> u32 {
        self.away.runs.len() as u32
    }

    /// Whether the card is one a match could have left: two sides, one of
    /// them ahead, the visitors having batted in every innings and the
    /// side at home in every one too, or in all but a last it had no need
    /// of.
    pub fn is_sound(&self) -> bool {
        let (home, away) = (&self.home, &self.away);
        let halves = home.runs.len() + usize::from(self.unneeded);
        home.side != away.side
            && home.total() != away.total()
            && !away.runs.is_empty()
            && halves == away.runs.len()
            && (!self.unneeded || home.total() > away.total())
            && home.outs_an_innings > 0
            && away.outs_an_innings > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::play::field::Ground;
    use crate::rules::Rules;
    use crate::settings::Difficulty;

    /// A full match of two innings played to its end, in which the
    /// player's side, at home or away, makes a run in every half it bats
    /// in, played on paper as the other side's are.
    fn finished(home: bool, seed: u64) -> FullMatch {
        let mut rules = Rules::default().full_match;
        rules.innings = 2;
        let ground = Ground::default();
        let mut full = FullMatch::new(home, &rules, Difficulty::Easy, false, None, ground, seed);
        let mut up = 0;
        while !full.is_over() {
            let wanted = paper::Wanted {
                made: 1,
                winning: false,
                innings: full.innings(),
                first_up: up,
            };
            let ground = Ground::default();
            let ours = &mut full.book.ours;
            up = paper::half_into(ours, wanted, seed, &rules.their_batting, None, &ground);
            full.side_out(1);
            assert!(full.innings() < 60, "the match never ended");
        }
        full
    }

    #[test]
    fn a_card_says_what_the_board_of_the_match_says() {
        for (home, seed) in [(true, 1), (false, 1), (true, 2), (false, 2), (true, 5)] {
            let full = finished(home, seed);
            let card = Card::of(&full, 4, 2, 6, 3).expect("a match that is over");
            assert!(card.is_sound(), "{card:?}");
            assert_eq!(card.fixture, 4);
            // The player's side is at place 2 of the draw, and at home
            // or away as it was.
            let (ours, theirs) = if home {
                (&card.home, &card.away)
            } else {
                (&card.away, &card.home)
            };
            assert_eq!((ours.side, theirs.side), (2, 6));
            assert_eq!((ours.total(), theirs.total()), (full.ours(), full.theirs()));
            let [visitors, at_home] = full.lines();
            assert_eq!(card.away.figures().hits, visitors.hits);
            assert_eq!(card.home.figures().hits, at_home.hits);
            assert_eq!(card.unneeded, full.unneeded());
            assert_eq!(card.innings(), full.visitors().runs.len() as u32);
            // Whoever is ahead won.
            let won = if full.ours() > full.theirs() { 2 } else { 6 };
            assert_eq!((card.winner(), card.loser()), (won, 8 - won));
            assert_eq!(card.of_side(6), Some(theirs));
            assert_eq!(card.of_side(3), None);
        }
    }

    #[test]
    fn a_sides_card_adds_up_to_what_its_book_says_of_it() {
        let full = finished(false, 9);
        let card = Card::of(&full, 0, 0, 1, 3).expect("a match that is over");
        for (side, book) in [
            (&card.away, &full.book.ours),
            (&card.home, &full.book.theirs),
        ] {
            assert_eq!(side.figures(), book.figures());
            assert_eq!(side.outs, book.outs());
            assert_eq!(side.outs_an_innings, 3);
        }
    }

    #[test]
    fn a_match_still_going_on_has_no_card() {
        let rules = Rules::default().full_match;
        let ground = Ground::default();
        let full = FullMatch::new(true, &rules, Difficulty::Easy, false, None, ground, 1);
        assert_eq!(Card::of(&full, 0, 0, 1, 3), None);
    }

    #[test]
    fn a_card_no_match_could_have_left_is_not_sound() {
        let sound = Card::of(&finished(true, 1), 0, 0, 1, 3).expect("a match that is over");
        let wrong = |change: &dyn Fn(&mut Card)| {
            let mut card = sound.clone();
            change(&mut card);
            !card.is_sound()
        };
        assert!(sound.is_sound());
        assert!(wrong(&|card| card.away.side = card.home.side));
        assert!(wrong(&|card| card.away.runs.push(0)));
        assert!(wrong(&|card| card.unneeded = !card.unneeded));
        assert!(wrong(&|card| card.home.outs_an_innings = 0));
        // Level is not over.
        assert!(wrong(&|card| {
            let (home, away) = (card.home.total(), card.away.total());
            if home > away {
                card.away.runs[0] += home - away;
            } else {
                card.home.runs[0] += away - home;
            }
        }));
    }
}
