//! The ties in which the loser goes out: a cup's, and the semi-finals and
//! final that follow two groups.

use super::{Slot, Tie};

/// A tie of a knockout round between two that are known only by where
/// they come from.
fn tie(round: usize, first: Slot, second: Slot, tossed: bool) -> Tie {
    Tie {
        round,
        group: None,
        first,
        second,
        tossed,
    }
}

/// The ties of a cup of eight: neighbours in the draw meet in the
/// quarter-finals, the winners of neighbouring ties after that, and a coin
/// says who is at home every time.
pub(super) fn cup() -> Vec<Tie> {
    let winners = |round, of| tie(round, Slot::WinnerOf(of), Slot::WinnerOf(of + 1), true);
    let mut ties: Vec<Tie> = (0..4)
        .map(|pair| tie(0, Slot::Place(pair * 2), Slot::Place(pair * 2 + 1), true))
        .collect();
    ties.extend([winners(1, 0), winners(1, 2), winners(2, 4)]);
    ties
}

/// The ties that follow two groups. In the semi-finals the winner of each
/// group is at home to the runner-up of the other, and the final is tossed
/// for. `round` is the first round after the groups', and `number` the
/// number its first tie has.
pub(super) fn after_two_groups(round: usize, number: usize) -> Vec<Tie> {
    let finisher = |group, place| Slot::Finisher { group, place };
    vec![
        tie(round, finisher(0, 0), finisher(1, 1), false),
        tie(round, finisher(1, 0), finisher(0, 1), false),
        tie(
            round + 1,
            Slot::WinnerOf(number),
            Slot::WinnerOf(number + 1),
            true,
        ),
    ]
}
