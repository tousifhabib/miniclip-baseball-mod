use std::collections::BTreeSet;

use proptest::prelude::*;

use super::*;

/// How often each of `count` places is at home over these rounds.
fn homes(count: usize, rounds: &[Vec<(usize, usize)>]) -> Vec<usize> {
    (0..count)
        .map(|place| {
            rounds
                .iter()
                .flatten()
                .filter(|(home, _)| *home == place)
                .count()
        })
        .collect()
}

proptest! {
    #[test]
    fn everyone_meets_everyone_once_and_nobody_twice_in_a_round(half in 1usize..=8) {
        let count = half * 2;
        let rounds = rounds(count);
        prop_assert_eq!(rounds.len(), count - 1);
        let mut met = BTreeSet::new();
        for round in &rounds {
            // Everybody plays in every round, once.
            let playing: BTreeSet<usize> =
                round.iter().flat_map(|&(home, away)| [home, away]).collect();
            prop_assert_eq!(round.len(), half);
            prop_assert_eq!(playing.len(), count);
            for &(home, away) in round {
                prop_assert!(home != away && home < count && away < count);
                let new = met.insert((home.min(away), home.max(away)));
                prop_assert!(new, "{} and {} meet twice", home, away);
            }
        }
        prop_assert_eq!(met.len(), count * (count - 1) / 2);
    }

    #[test]
    fn nobody_is_at_home_much_more_often_than_anybody_else(half in 1usize..=8) {
        let count = half * 2;
        let homes = homes(count, &rounds(count));
        // With an odd number of matches each, half of them rounded down
        // or rounded up.
        let fewest = (count - 1) / 2;
        for (place, at_home) in homes.into_iter().enumerate() {
            let fair = at_home == fewest || at_home == fewest + 1;
            prop_assert!(fair, "place {} is at home {} times of {}", place, at_home, count - 1);
        }
    }
}

#[test]
fn every_tie_of_every_shape_is_between_two_that_are_known_by_then() {
    for format in Format::ALL {
        let ties = ties(format);
        let mut round = 0;
        for (number, tie) in ties.iter().enumerate() {
            // The ties are numbered round by round.
            assert!(
                tie.round >= round,
                "{format:?}: tie {number} is out of turn"
            );
            round = tie.round;
            assert_eq!(format.round(tie.round).is_knockout(), tie.group.is_none());
            for slot in [tie.first, tie.second] {
                match slot {
                    Slot::Place(place) => assert!(place < format.sides()),
                    Slot::Finisher { group, place } => {
                        assert!(group < format.groups() && place < format.go_through());
                    }
                    // A tie whose winner is waited for is an earlier
                    // one, of the round before.
                    Slot::WinnerOf(earlier) => {
                        assert!(earlier < number);
                        assert_eq!(ties[earlier].round + 1, tie.round);
                    }
                }
            }
            assert_ne!(tie.first, tie.second);
        }
        assert_eq!(round + 1, format.rounds(), "{format:?}");
    }
}

#[test]
fn in_every_round_of_a_group_each_of_its_sides_plays_once() {
    for format in [Format::Groups, Format::League] {
        let ties = ties(format);
        for round in 0..format.in_a_group() - 1 {
            for group in 0..format.groups() {
                let playing: BTreeSet<usize> = ties
                    .iter()
                    .filter(|tie| tie.round == round && tie.group == Some(group))
                    .flat_map(|tie| [tie.first, tie.second])
                    .map(|slot| match slot {
                        Slot::Place(place) => place,
                        other => panic!("{other:?} in a group"),
                    })
                    .collect();
                let all: BTreeSet<usize> = places(format, group).collect();
                assert_eq!(playing, all, "{format:?}, round {round}, group {group}");
            }
        }
    }
}
