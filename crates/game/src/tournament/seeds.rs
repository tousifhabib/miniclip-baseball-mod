//! What each thing in a tournament that draws numbers draws them from, so
//! that no two draw the same, and a tournament begun from one seed goes
//! the same way every time.

use crate::rng::{Rng, mixed_with};

/// More than the fixtures any tournament has, so that a fixture's number
/// and how many had been begun before it make one number between them.
const MORE_THAN_THE_FIXTURES: u64 = 16;

/// What the draw is made from.
pub fn of_the_draw(seed: u64) -> u64 {
    seed ^ mixed_with::THE_DRAW
}

/// What a fixture is played from: the tournament's seed, the fixture's
/// number, and how many fixtures the player had begun before this one was.
/// A fixture given up half way and begun again is so another game. One
/// played on paper has had none begun before it.
pub fn of_a_fixture(seed: u64, fixture: usize, begun: u32) -> u64 {
    let which = 1 + fixture as u64 + MORE_THAN_THE_FIXTURES * u64::from(begun);
    seed ^ mixed_with::A_FIXTURE.wrapping_mul(which)
}

/// What one side of a fixture played on paper makes its runs and plays
/// its halves from: the fixture's seed, and which side it is.
pub(super) fn of_a_side_on_paper(fixture: u64, at_home: bool) -> u64 {
    if at_home {
        fixture ^ mixed_with::THE_HOME_SIDE_ON_PAPER
    } else {
        fixture ^ mixed_with::THE_VISITORS_ON_PAPER
    }
}

/// Whether the first-named side of a tie that is tossed for is the one at
/// home.
pub fn first_named_is_at_home(seed: u64, fixture: usize) -> bool {
    let coin = of_a_fixture(seed, fixture, 0) ^ mixed_with::THE_COIN;
    Rng::new(coin).below(2) == 0
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn no_two_fixtures_are_played_from_the_same_numbers_however_often_begun() {
        let seed = 0x1234_5678_9abc_def0;
        let mut all = BTreeSet::from([seed, of_the_draw(seed)]);
        for fixture in 0..15 {
            for begun in 0..40 {
                let new = all.insert(of_a_fixture(seed, fixture, begun));
                assert!(new, "fixture {fixture}, begun {begun}");
            }
        }
    }

    #[test]
    fn a_coin_comes_down_both_ways_and_the_same_way_for_the_same_tie() {
        let homes = (0..200)
            .filter(|&seed| first_named_is_at_home(seed, 3))
            .count();
        assert!((70..=130).contains(&homes), "{homes} of 200");
        assert_eq!(first_named_is_at_home(9, 3), first_named_is_at_home(9, 3));
    }
}
