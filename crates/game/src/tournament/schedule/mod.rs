//! Who is to meet whom: every tie of a tournament, as far as it can be
//! said before a ball is thrown.
//!
//! A side is known here by its place in the draw. The ties in which
//! everyone meets everyone are between places. Those of the knockout
//! rounds are between whoever finishes where in a group, or whoever wins
//! an earlier tie.

mod knockout;
#[cfg(test)]
mod properties;
mod round_robin;

use super::format::Format;
pub use round_robin::rounds;

/// One of the two sides of a tie, as far as it is known beforehand.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slot {
    /// The side at this place in the draw.
    Place(usize),
    /// Whoever finishes at this place of this group, the top being
    /// nought.
    Finisher { group: usize, place: usize },
    /// Whoever wins the tie with this number.
    WinnerOf(usize),
}

/// A tie: two sides to meet, in a round. Its number is where it comes
/// among all the ties of the tournament.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tie {
    /// The round it is in, the first being nought.
    pub round: usize,
    /// The group it is a tie of, if it is one of a group's.
    pub group: Option<usize>,
    pub first: Slot,
    pub second: Slot,
    /// Whether a coin says which of them is at home. If not, the first
    /// is.
    pub tossed: bool,
}

/// The places in the draw that make up a group of a tournament of this
/// shape.
pub fn places(format: Format, group: usize) -> std::ops::Range<usize> {
    let each = format.in_a_group();
    group * each..(group + 1) * each
}

/// Every tie of a tournament of this shape, in the order of their numbers:
/// round by round, and within a round group by group.
pub fn ties(format: Format) -> Vec<Tie> {
    if format == Format::Cup {
        return knockout::cup();
    }
    let rounds = rounds(format.in_a_group());
    let mut ties = Vec::new();
    for (round, pairs) in rounds.iter().enumerate() {
        for group in 0..format.groups() {
            let from = places(format, group).start;
            ties.extend(pairs.iter().map(|&(home, away)| Tie {
                round,
                group: Some(group),
                first: Slot::Place(from + home),
                second: Slot::Place(from + away),
                tossed: false,
            }));
        }
    }
    if format == Format::Groups {
        let after = knockout::after_two_groups(rounds.len(), ties.len());
        ties.extend(after);
    }
    ties
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_groups_of_four_lead_to_semi_finals_and_a_final() {
        let ties = ties(Format::Groups);
        assert_eq!(ties.len(), 15);
        // The first round: two ties of the first group, then two of the
        // second, whose places follow the first's.
        let group = |tie: &Tie| tie.group;
        assert_eq!(
            ties[..4].iter().map(group).collect::<Vec<_>>(),
            [Some(0), Some(0), Some(1), Some(1)]
        );
        assert_eq!(places(Format::Groups, 1), 4..8);
        assert!(ties[..12].iter().all(|tie| !tie.tossed));
        // The winner of each group is at home to the runner-up of the
        // other.
        let finisher = |group, place| Slot::Finisher { group, place };
        assert_eq!(
            (ties[12].first, ties[12].second),
            (finisher(0, 0), finisher(1, 1))
        );
        assert_eq!(
            (ties[13].first, ties[13].second),
            (finisher(1, 0), finisher(0, 1))
        );
        assert!(!ties[12].tossed && !ties[13].tossed);
        let last = ties[14];
        assert_eq!(
            (last.first, last.second),
            (Slot::WinnerOf(12), Slot::WinnerOf(13))
        );
        assert!(last.tossed && last.round == 4 && last.group.is_none());
    }

    #[test]
    fn a_league_is_one_group_of_six_over_five_rounds() {
        let ties = ties(Format::League);
        assert_eq!(ties.len(), 15);
        assert!(ties.iter().all(|tie| tie.group == Some(0) && !tie.tossed));
        assert_eq!(ties.iter().map(|tie| tie.round).max(), Some(4));
        assert_eq!(places(Format::League, 0), 0..6);
    }

    #[test]
    fn a_cup_of_eight_is_three_rounds_of_ties_each_tossed_for() {
        let ties = ties(Format::Cup);
        assert_eq!(ties.len(), 7);
        assert!(ties.iter().all(|tie| tie.tossed && tie.group.is_none()));
        let rounds: Vec<usize> = ties.iter().map(|tie| tie.round).collect();
        assert_eq!(rounds, [0, 0, 0, 0, 1, 1, 2]);
        assert_eq!(
            (ties[0].first, ties[0].second),
            (Slot::Place(0), Slot::Place(1))
        );
        assert_eq!(
            (ties[3].first, ties[3].second),
            (Slot::Place(6), Slot::Place(7))
        );
        assert_eq!(
            (ties[5].first, ties[5].second),
            (Slot::WinnerOf(2), Slot::WinnerOf(3))
        );
        assert_eq!(
            (ties[6].first, ties[6].second),
            (Slot::WinnerOf(4), Slot::WinnerOf(5))
        );
    }
}
