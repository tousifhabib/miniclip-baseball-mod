use proptest::prelude::*;

use super::*;

/// A runner at a place, with nothing else to say of him.
pub(crate) fn at(place: Place) -> Runner {
    Runner {
        place,
        running_to: None,
        sliding: false,
        runs: 0,
        order: 0,
        stole_from: None,
        skin: None,
        logo: None,
        path: None,
    }
}

fn running(from: u8, to: u8) -> Runner {
    Runner {
        running_to: Some(to),
        ..at(Place::Base(from))
    }
}

fn stealing(from: u8) -> Runner {
    Runner {
        stole_from: Some(from),
        ..running(from, from + 1)
    }
}

pub(crate) fn runners(list: Vec<Runner>) -> Runners {
    Runners(list)
}

#[test]
fn the_batter_is_whoever_is_at_the_plate() {
    let half = runners(vec![at(Place::Out), at(Place::Base(1)), at(Place::AtBat)]);
    assert_eq!(half.batter(), Some(2));
    assert_eq!(runners(vec![at(Place::Home)]).batter(), None);
}

#[test]
fn a_runner_who_has_set_off_is_not_standing_on_his_base_but_it_is_still_his() {
    let half = runners(vec![at(Place::Base(2)), running(1, 2)]);
    assert_eq!(half.on_base(2), Some(0));
    assert_eq!(half.on_base(1), None);
    assert!(half.has_one_from(1));
    assert!(half.anyone_running());
    assert!(half.anyone_making_for(2));
    assert!(!half.anyone_making_for(3));
    assert_eq!(half.on_the_bases(), 2);
}

#[test]
fn a_runner_may_steal_a_base_that_is_free_or_is_being_left() {
    // Second is free.
    let half = runners(vec![at(Place::Base(1))]);
    assert_eq!(half.may_steal(0), Some(2));
    // Somebody stands on it.
    let half = runners(vec![at(Place::Base(1)), at(Place::Base(2))]);
    assert_eq!(half.may_steal(0), None);
    // He is going himself, so it will be free.
    let half = runners(vec![at(Place::Base(1)), running(2, 3)]);
    assert_eq!(half.may_steal(0), Some(2));
    // Somebody else is already on his way there.
    let half = runners(vec![at(Place::Base(1)), running(1, 2)]);
    assert_eq!(half.may_steal(0), None);
}

#[test]
fn nobody_steals_home_and_nobody_steals_who_is_not_standing_on_a_base() {
    let half = runners(vec![
        at(Place::Base(3)),
        at(Place::AtBat),
        at(Place::Out),
        running(1, 2),
    ]);
    for runner in 0..5 {
        assert_eq!(half.may_steal(runner), None, "runner {runner}");
    }
}

#[test]
fn the_bases_being_stolen_are_those_of_runners_who_left_one_to_steal() {
    let half = runners(vec![stealing(2), running(1, 2), stealing(1)]);
    assert!(half.anyone_stealing());
    assert_eq!(half.bases_being_stolen().collect::<Vec<u8>>(), [3, 2]);
    assert!(!runners(vec![running(1, 2)]).anyone_stealing());
}

#[test]
fn only_runners_with_someone_coming_up_behind_them_are_forced_on() {
    // Runners on first and third: the one on third may stay.
    let half = runners(vec![
        at(Place::Base(3)),
        at(Place::Base(1)),
        at(Place::AtBat),
    ]);
    assert_eq!(half.forced_on(), [(2, 1), (1, 2)]);
    // The bases loaded: everybody goes.
    let half = runners(vec![
        at(Place::Base(3)),
        at(Place::Base(2)),
        at(Place::Base(1)),
        at(Place::AtBat),
    ]);
    assert_eq!(half.forced_on(), [(3, 1), (2, 2), (1, 3), (0, 4)]);
    // Nobody on first: only the batter.
    let half = runners(vec![at(Place::Base(2)), at(Place::AtBat)]);
    assert_eq!(half.forced_on(), [(1, 1)]);
}

#[test]
fn a_walk_pushes_on_the_runner_on_first_and_those_with_no_gap_behind_them() {
    let first_and_third = runners(vec![at(Place::Base(1)), at(Place::Base(3))]);
    assert_eq!(first_and_third.pushed_by_a_walk(), [true, false, false]);
    let first_and_second = runners(vec![at(Place::Base(1)), stealing(2)]);
    assert_eq!(first_and_second.pushed_by_a_walk(), [true, true, false]);
    let second_only = runners(vec![at(Place::Base(2))]);
    assert_eq!(second_only.pushed_by_a_walk(), [false, false, false]);
}

#[test]
fn a_walk_leaves_a_steal_only_to_a_runner_it_does_not_push() {
    // The runner on first is pushed, and the one on third is not.
    let mut half = runners(vec![stealing(1), at(Place::Base(3))]);
    half.a_walk_takes_the_steals_it_pushes();
    assert!(!half.anyone_stealing());
    let mut half = runners(vec![stealing(2)]);
    half.a_walk_takes_the_steals_it_pushes();
    assert_eq!(half.bases_being_stolen().collect::<Vec<u8>>(), [3]);
    half.steals_are_runs();
    assert!(!half.anyone_stealing());
    // He is still on his way, as a runner like any other.
    assert!(half.anyone_running());
}

#[test]
fn runs_are_added_up_by_place_in_the_order_on_top_of_what_was_made_before() {
    let scored = |order, runs| Runner {
        runs,
        order,
        ..at(Place::Home)
    };
    let half = runners(vec![scored(0, 1), scored(2, 2), scored(0, 1)]);
    assert_eq!(half.runs_by_order(&[]), [2, 0, 2]);
    assert_eq!(half.runs_by_order(&[1, 1, 1, 1]), [3, 1, 3, 1]);
}

/// Any runner there could be: where he is, and whether he is on his
/// way to the next base.
fn any_runner() -> impl Strategy<Value = Runner> {
    let place = prop_oneof![
        Just(Place::AtBat),
        (1u8..=3).prop_map(Place::Base),
        Just(Place::Out),
        Just(Place::Home),
    ];
    (place, any::<bool>()).prop_map(|(place, going)| match place {
        Place::Base(base) if going => running(base, base + 1),
        _ => at(place),
    })
}

proptest! {
    #[test]
    fn a_base_that_may_be_stolen_is_the_next_one_and_nobody_else_wants_it(
        list in proptest::collection::vec(any_runner(), 0..6),
    ) {
        let half = runners(list);
        for runner in 0..half.len() {
            let Some(to) = half.may_steal(runner) else {
                continue;
            };
            prop_assert_eq!(half[runner].place, Place::Base(to - 1));
            prop_assert!(to <= 3);
            prop_assert_eq!(half.on_base(to), None);
            prop_assert!(!half.anyone_making_for(to));
        }
    }

    #[test]
    fn those_forced_on_each_go_one_base_on_to_a_base_of_their_own(
        list in proptest::collection::vec(any_runner(), 0..6),
    ) {
        let half = runners(list);
        let going = half.forced_on();
        for (index, &(runner, to)) in going.iter().enumerate() {
            // The batter first, then first base, second and third.
            prop_assert_eq!(usize::from(to), index + 1);
            let from = match half[runner].place {
                Place::AtBat => 0,
                Place::Base(base) => base,
                other => return Err(TestCaseError::fail(format!("{other:?} was sent on"))),
            };
            prop_assert_eq!(from + 1, to);
        }
        if half.batter().is_none() {
            prop_assert!(going.is_empty());
        }
    }
}
