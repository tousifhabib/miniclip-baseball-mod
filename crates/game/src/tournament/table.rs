//! A table: how the sides of a group or a league stand, and in what
//! order.

use super::card::Card;

/// How one side stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Row {
    /// Which side it is, by its place in the draw.
    pub side: usize,
    pub played: u32,
    pub won: u32,
    pub lost: u32,
    pub runs_for: u32,
    pub runs_against: u32,
}

impl Row {
    /// By how many runs it is ahead of what it has let in, or behind if
    /// less than nought.
    pub fn difference(&self) -> i64 {
        i64::from(self.runs_for) - i64::from(self.runs_against)
    }

    /// How a side stands that has played these.
    fn of<'a>(side: usize, cards: impl IntoIterator<Item = &'a Card>) -> Row {
        let mut row = Row {
            side,
            played: 0,
            won: 0,
            lost: 0,
            runs_for: 0,
            runs_against: 0,
        };
        for card in cards {
            let (ours, theirs) = if card.home.side == side {
                (&card.home, &card.away)
            } else if card.away.side == side {
                (&card.away, &card.home)
            } else {
                continue;
            };
            row.played += 1;
            row.won += u32::from(card.winner() == side);
            row.lost += u32::from(card.loser() == side);
            // A side's runs are its score, innings by innings.
            row.runs_for += ours.total();
            row.runs_against += theirs.total();
        }
        row
    }
}

/// The table of `sides`, each known by its place in the draw, from the
/// cards of the matches between them, the side at the top first. A card
/// of a match one of whose sides is not among them is not counted.
///
/// A match is won or lost, so the sides are in order of matches won. Those
/// level on that are parted by the matches won among themselves, then by
/// how far their runs are ahead of the runs against them, then by their
/// runs, and at the last by their place in the draw.
pub fn table<'a>(sides: &[usize], cards: impl IntoIterator<Item = &'a Card>) -> Vec<Row> {
    let among = |card: &&Card| {
        let has = |side| sides.contains(&side);
        has(card.home.side) && has(card.away.side)
    };
    let counted: Vec<&Card> = cards.into_iter().filter(among).collect();
    let mut rows: Vec<Row> = sides
        .iter()
        .map(|&side| Row::of(side, counted.iter().copied()))
        .collect();
    // The matches a side has won against the sides it is level with on
    // matches won. They are counted once, among all who are level, and
    // those still level after it are not counted over again.
    let level = |side: usize| rows.iter().find(|row| row.side == side).map(|row| row.won);
    let among_the_level: Vec<(usize, usize)> = rows
        .iter()
        .map(|row| {
            let beaten = counted
                .iter()
                .filter(|card| card.winner() == row.side && level(card.loser()) == Some(row.won));
            (row.side, beaten.count())
        })
        .collect();
    let beaten = |side: usize| {
        let found = among_the_level.iter().find(|(of, _)| *of == side);
        found.map_or(0, |(_, beaten)| *beaten)
    };
    rows.sort_by(|one, other| {
        other
            .won
            .cmp(&one.won)
            .then(beaten(other.side).cmp(&beaten(one.side)))
            .then(other.difference().cmp(&one.difference()))
            .then(other.runs_for.cmp(&one.runs_for))
            .then(one.side.cmp(&other.side))
    });
    rows
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;
    use crate::tournament::testing::result;

    fn order(rows: &[Row]) -> Vec<usize> {
        rows.iter().map(|row| row.side).collect()
    }

    #[test]
    fn before_a_ball_is_thrown_the_sides_stand_as_they_were_drawn() {
        let rows = table(&[4, 5, 6, 7], &[]);
        assert_eq!(order(&rows), [4, 5, 6, 7]);
        assert!(
            rows.iter()
                .all(|row| row.played == 0 && row.difference() == 0)
        );
    }

    #[test]
    fn the_side_that_has_won_more_is_above_whatever_its_runs() {
        // Side 1 has won twice by a run, side 0 once by ten.
        let cards = [
            result(0, (1, 2), (2, 1)),
            result(1, (1, 3), (0, 2)),
            result(2, (0, 12), (2, 2)),
        ];
        let rows = table(&[0, 1, 2], &cards);
        assert_eq!(order(&rows), [1, 0, 2]);
        let top = rows[0];
        assert_eq!((top.played, top.won, top.lost), (2, 2, 0));
        assert_eq!(
            (top.runs_for, top.runs_against, top.difference()),
            (5, 3, 2)
        );
        assert_eq!(rows[2].difference(), -11);
    }

    #[test]
    fn two_level_on_wins_are_parted_by_the_match_between_them() {
        // Each of 0 and 1 has won two. Side 0 has far the better runs,
        // but it was 1 who won when they met.
        let cards = [
            result(0, (1, 1), (0, 0)),
            result(1, (0, 9), (2, 0)),
            result(2, (0, 9), (3, 0)),
            result(3, (1, 1), (2, 0)),
            result(4, (3, 1), (1, 0)),
        ];
        assert_eq!(order(&table(&[0, 1, 2, 3], &cards))[..2], [1, 0]);
    }

    #[test]
    fn three_who_have_each_beaten_one_of_the_others_are_parted_by_their_runs() {
        // 0 beat 1, 1 beat 2 and 2 beat 0: a win apiece among them.
        let round = [
            result(0, (0, 3), (1, 2)),
            result(1, (1, 6), (2, 1)),
            result(2, (2, 4), (0, 1)),
        ];
        // Side 1 is four runs to the good, 2 two to the bad and 0 two to
        // the bad as well, with fewer runs than 2 has.
        assert_eq!(order(&table(&[0, 1, 2], &round)), [1, 2, 0]);
        // With everything the same, the place in the draw is all there
        // is left.
        let same = [
            result(0, (0, 2), (1, 1)),
            result(1, (1, 2), (2, 1)),
            result(2, (2, 2), (0, 1)),
        ];
        assert_eq!(order(&table(&[2, 0, 1], &same)), [0, 1, 2]);
    }

    #[test]
    fn a_match_against_a_side_from_outside_is_not_the_tables() {
        let cards = [result(0, (0, 5), (1, 1)), result(1, (9, 7), (0, 0))];
        let rows = table(&[0, 1], &cards);
        assert_eq!((rows[0].side, rows[0].played, rows[0].lost), (0, 1, 0));
    }

    proptest! {
        #[test]
        fn a_table_adds_up_whatever_was_played(
            // Any matches at all between six sides: who was at home, who
            // away, and the runs of each, which are never level.
            played in prop::collection::vec((0usize..6, 1usize..6, 0u32..15, 1u32..15), 0..40),
        ) {
            let cards: Vec<Card> = played
                .into_iter()
                .enumerate()
                .map(|(fixture, (home, apart, runs, more))| {
                    let away = (home + apart) % 6;
                    // The side at home wins every other one of them.
                    let (ours, theirs) = if fixture % 2 == 0 {
                        (runs + more, runs)
                    } else {
                        (runs, runs + more)
                    };
                    result(fixture, (home, ours), (away, theirs))
                })
                .collect();
            let sides: Vec<usize> = (0..6).collect();
            let rows = table(&sides, &cards);
            // Every side is there once.
            let mut there = order(&rows);
            there.sort_unstable();
            prop_assert_eq!(there, sides);
            // Every match is one side's win and another's loss, and its
            // runs are one side's for and the other's against.
            let sum = |of: fn(&Row) -> u32| rows.iter().map(of).sum::<u32>();
            prop_assert_eq!(sum(|row| row.won), cards.len() as u32);
            prop_assert_eq!(sum(|row| row.lost), cards.len() as u32);
            prop_assert_eq!(sum(|row| row.played), 2 * cards.len() as u32);
            prop_assert_eq!(sum(|row| row.runs_for), sum(|row| row.runs_against));
            let runs: u32 = cards.iter().map(|card| card.home.total() + card.away.total()).sum();
            prop_assert_eq!(sum(|row| row.runs_for), runs);
            for row in &rows {
                prop_assert_eq!(row.played, row.won + row.lost);
            }
            // And nobody is above a side that has won more.
            for pair in rows.windows(2) {
                prop_assert!(pair[0].won >= pair[1].won);
            }
        }
    }
}
