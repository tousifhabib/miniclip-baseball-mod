//! The records of a tournament so far: the most of this and the longest
//! of that there has been in one match, and who did it.

use crate::play::full::ordinal;
use crate::tournament::{Card, SideCard, Tournament, schedule};

/// A record: what it is of, what it stands at, and who set it and when.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Record {
    pub what: &'static str,
    pub stands_at: String,
    pub by: String,
}

/// The record so far of one thing, as the cards are gone through.
struct Best {
    what: &'static str,
    most: u32,
    by: String,
    /// What the number is a number of, written after it.
    of: &'static str,
}

impl Best {
    fn new(what: &'static str, of: &'static str) -> Best {
        Best {
            what,
            most: 0,
            by: String::new(),
            of,
        }
    }

    /// Takes in a try at the record. The first to get to a number keeps
    /// it until it is beaten.
    fn tried(&mut self, count: u32, by: impl FnOnce() -> String) {
        if count > self.most {
            self.most = count;
            self.by = by();
        }
    }

    fn record(self) -> Option<Record> {
        (self.most > 0).then(|| Record {
            what: self.what,
            stands_at: format!("{}{}", self.most, self.of),
            by: self.by,
        })
    }
}

/// The records there are so far, each set in one match. A record of
/// nothing, with no home run hit yet or no match gone past its innings,
/// is not among them.
pub fn records(tournament: &Tournament) -> Vec<Record> {
    let format = tournament.setup().format;
    let ties = schedule::ties(format);
    let mut score = Best::new("HIGHEST SCORE", "");
    let mut win = Best::new("BIGGEST WIN", " RUNS");
    let mut innings = Best::new("MOST RUNS IN AN INNINGS", "");
    let mut both = Best::new("MOST RUNS BY BOTH SIDES", "");
    let mut longest = Best::new("LONGEST MATCH", " INNINGS");
    let mut hit = Best::new("LONGEST HIT", " FT");
    let mut hits = Best::new("MOST HITS BY A SIDE", "");
    let mut home_runs = Best::new("MOST HOME RUNS BY A SIDE", "");
    let mut strikeouts = Best::new("MOST STRIKEOUTS THROWN", "");
    let mut batted_in = Best::new("MOST RUNS BATTED IN BY A BATTER", "");
    for card in tournament.cards() {
        let round = ties
            .get(card.fixture)
            .map_or(String::new(), |tie| format.round(tie.round).words());
        let short = |side: &SideCard| tournament.short_of(side.side).to_owned();
        // The match as its record is told: the side whose record it is
        // first, then the other, then the round.
        let by = |side: &SideCard, other: &SideCard| {
            format!("{} v {}, {round}", short(side), short(other))
        };
        let scores = |card: &Card| {
            let (home, away) = (&card.home, &card.away);
            format!(
                "{} {} - {} {}, {round}",
                short(home),
                home.total(),
                away.total(),
                short(away)
            )
        };
        let (home, away) = (card.home.total(), card.away.total());
        win.tried(home.abs_diff(away), || scores(card));
        both.tried(home + away, || scores(card));
        if card.innings() > tournament.setup().innings {
            longest.tried(card.innings(), || scores(card));
        }
        for (side, other) in [(&card.home, &card.away), (&card.away, &card.home)] {
            let figures = side.figures();
            score.tried(side.total(), || by(side, other));
            hits.tried(figures.hits, || by(side, other));
            home_runs.tried(figures.home_runs, || by(side, other));
            // The side in the field threw the strikeouts of the side at
            // bat.
            strikeouts.tried(figures.strikeouts, || by(other, side));
            for (number, runs) in (1..).zip(&side.runs) {
                let when = || {
                    let innings = ordinal(number);
                    format!(
                        "{} v {} IN THE {innings}, {round}",
                        short(side),
                        short(other)
                    )
                };
                innings.tried(*runs, when);
            }
            for (place, batter) in (1..).zip(&side.places) {
                let who = || format!("{} {place} v {}, {round}", short(side), short(other));
                hit.tried(batter.longest, who);
                batted_in.tried(batter.runs_in, who);
            }
        }
    }
    let all = [
        score, win, innings, both, longest, hit, hits, home_runs, strikeouts, batted_in,
    ];
    all.into_iter().filter_map(Best::record).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::Rules;
    use crate::tournament::Format;
    use crate::tournament::testing::{drawn, played_out};

    #[test]
    fn before_a_ball_is_thrown_there_are_no_records() {
        assert!(records(&drawn(Format::League, 3, 6)).is_empty());
    }

    #[test]
    fn a_record_is_the_most_there_has_been_in_any_one_match() {
        let done = played_out(drawn(Format::League, 3, 6), &Rules::default(), |_| {});
        let records = records(&done);
        let of = |what: &str| {
            let record = records.iter().find(|record| record.what == what);
            record.unwrap_or_else(|| panic!("no record of {what}"))
        };
        let number = |record: &Record| -> u32 {
            let figures = record.stands_at.split(' ').next().expect("a number");
            figures.parse().expect("a number")
        };
        let cards = done.cards();
        let sides = || cards.iter().flat_map(|card| [&card.home, &card.away]);
        let highest = sides().map(SideCard::total).max().expect("a score");
        assert_eq!(number(of("HIGHEST SCORE")), highest);
        let widest = cards
            .iter()
            .map(|card| card.home.total().abs_diff(card.away.total()));
        assert_eq!(number(of("BIGGEST WIN")), widest.max().expect("a win"));
        assert!(of("BIGGEST WIN").stands_at.ends_with(" RUNS"));
        let most = sides().flat_map(|side| side.runs.iter().copied()).max();
        assert_eq!(Some(number(of("MOST RUNS IN AN INNINGS"))), most);
        assert!(of("MOST RUNS IN AN INNINGS").by.contains(" IN THE "));
        let far = sides().map(|side| side.figures().longest).max();
        assert_eq!(Some(number(of("LONGEST HIT"))), far);
        assert!(of("LONGEST HIT").stands_at.ends_with(" FT"));
        // Every record says who set it and in which round.
        for record in &records {
            assert!(record.by.contains(" ROUND "), "{record:?}");
        }
        // A match that went past its innings is the longest there was.
        let long = cards.iter().map(Card::innings).max().expect("a match");
        let has = records.iter().find(|record| record.what == "LONGEST MATCH");
        assert_eq!(has.map(number), (long > 3).then_some(long));
    }
}
