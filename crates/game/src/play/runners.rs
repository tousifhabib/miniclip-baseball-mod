//! The batters of the half being played, where each has got to, and the
//! count on the one at the plate.
//!
//! Everything here is a question that can be answered without looking at
//! the stage: who is up, who stands on which base, who is between bases,
//! who has to go when the batter does. Moving a runner's clip is the
//! fielding's business.

use std::ops::{Deref, DerefMut};

use bb_engine::display::Path;

use crate::look::Rgb;

/// Where a batter has got to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Place {
    /// At the plate, batting.
    AtBat,
    /// Standing on first, second or third.
    Base(u8),
    Out,
    /// Round all the bases: a run.
    Home,
}

#[derive(Clone, Debug)]
pub(crate) struct Runner {
    pub place: Place,
    /// The base he is running to now, home being 4.
    pub running_to: Option<u8>,
    pub sliding: bool,
    pub runs: u32,
    /// His place in the batting order, counting from nought. A full match
    /// has nine, who come round again. Otherwise every batter is new.
    pub order: usize,
    /// The base he left to steal the next, on the pitch in hand.
    pub stole_from: Option<u8>,
    pub skin: Option<Rgb>,
    pub logo: Option<String>,
    /// His clip on the field, for as long as this pitch's view lasts.
    pub path: Option<Path>,
}

/// Every batter of the half so far, in the order they came up: the one at
/// the plate, those on base, and those who are out or home. It is a list
/// like any other, with the questions the play asks of it.
#[derive(Clone, Debug, Default)]
pub(crate) struct Runners(Vec<Runner>);

impl Deref for Runners {
    type Target = Vec<Runner>;

    fn deref(&self) -> &Vec<Runner> {
        &self.0
    }
}

impl DerefMut for Runners {
    fn deref_mut(&mut self) -> &mut Vec<Runner> {
        &mut self.0
    }
}

impl IntoIterator for Runners {
    type Item = Runner;
    type IntoIter = std::vec::IntoIter<Runner>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a Runners {
    type Item = &'a Runner;
    type IntoIter = std::slice::Iter<'a, Runner>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<'a> IntoIterator for &'a mut Runners {
    type Item = &'a mut Runner;
    type IntoIter = std::slice::IterMut<'a, Runner>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter_mut()
    }
}

impl Runners {
    /// The batter at the plate: his place in the list.
    pub fn batter(&self) -> Option<usize> {
        self.iter().position(|runner| runner.place == Place::AtBat)
    }

    /// The runner standing on a base, if there is one. A runner who has
    /// set off from it is not standing on it.
    pub fn on_base(&self, base: u8) -> Option<usize> {
        self.iter()
            .position(|runner| runner.place == Place::Base(base) && runner.running_to.is_none())
    }

    /// Whether a base is somebody's: he stands on it, or has set off from
    /// it and not yet got to the next.
    pub fn has_one_from(&self, base: u8) -> bool {
        self.iter().any(|runner| runner.place == Place::Base(base))
    }

    /// How many runners are on the bases, standing or between them.
    pub fn on_the_bases(&self) -> usize {
        self.iter()
            .filter(|runner| matches!(runner.place, Place::Base(_)))
            .count()
    }

    /// Whether anybody is between bases.
    pub fn anyone_running(&self) -> bool {
        self.iter().any(|runner| runner.running_to.is_some())
    }

    /// Whether anybody is on his way to this base, home being 4.
    pub fn anyone_making_for(&self, base: u8) -> bool {
        self.iter().any(|runner| runner.running_to == Some(base))
    }

    /// Whether anybody left his base to steal the next, on the pitch in
    /// hand.
    pub fn anyone_stealing(&self) -> bool {
        self.iter().any(|runner| runner.stole_from.is_some())
    }

    /// The bases that are being stolen, in the order their runners came
    /// up.
    pub fn bases_being_stolen(&self) -> impl Iterator<Item = u8> + '_ {
        self.iter()
            .filter(|runner| runner.stole_from.is_some())
            .filter_map(|runner| runner.running_to)
    }

    /// The base a runner may be sent to steal, if he may be: he is standing
    /// on first or second, and the base in front of him is free, or will be
    /// because the runner on it is going himself.
    pub fn may_steal(&self, runner: usize) -> Option<u8> {
        let stands = self.get(runner)?;
        let Place::Base(base) = stands.place else {
            return None;
        };
        if stands.running_to.is_some() || base >= 3 {
            return None;
        }
        let next = base + 1;
        let free = self.iter().all(|other| {
            let in_the_way = other.place == Place::Base(next) && other.running_to.is_none();
            !in_the_way && other.running_to != Some(next)
        });
        free.then_some(next)
    }

    /// Who has to go when the batter runs to first, and where to: the
    /// batter, and each runner standing on a base with someone coming up
    /// behind him.
    pub fn forced_on(&self) -> Vec<(usize, u8)> {
        let mut going = Vec::new();
        if let Some(batter) = self.batter() {
            going.push((batter, 1));
        }
        for base in 1..=3 {
            // Only a runner with someone coming up behind him has to go.
            match self.on_base(base) {
                Some(runner) if going.len() == usize::from(base) => {
                    going.push((runner, base + 1));
                }
                _ => break,
            }
        }
        going
    }

    /// Which of first, second and third a walk pushes its runner on from:
    /// first, and each base behind which there is no gap.
    pub fn pushed_by_a_walk(&self) -> [bool; 3] {
        [1, 2, 3].map(|base: u8| (1..=base).all(|behind| self.has_one_from(behind)))
    }

    /// The pitch was hit fair: whoever was stealing is a runner like any
    /// other now, and has stolen nothing.
    pub fn steals_are_runs(&mut self) {
        for runner in self {
            runner.stole_from = None;
        }
    }

    /// Four balls: a runner the walk pushes on was going there anyway, and
    /// has stolen nothing. One it does not push still has a base to steal,
    /// with nobody throwing.
    pub fn a_walk_takes_the_steals_it_pushes(&mut self) {
        let pushed = self.pushed_by_a_walk();
        for runner in self {
            if let Some(from) = runner.stole_from
                && pushed[usize::from(from) - 1]
            {
                runner.stole_from = None;
            }
        }
    }

    /// The runs made by each place in the batting order: what was made
    /// `before`, in the innings gone by, and what these runners have made.
    pub fn runs_by_order(&self, before: &[u32]) -> Vec<u32> {
        let mut runs = before.to_vec();
        for runner in self {
            if runs.len() <= runner.order {
                runs.resize(runner.order + 1, 0);
            }
            runs[runner.order] += runner.runs;
        }
        runs
    }
}

/// The count on the batter at the plate.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Count {
    pub strikes: u32,
    pub balls: u32,
}

impl Count {
    /// Whether nothing has been called on this batter yet.
    pub fn is_clean(self) -> bool {
        self.strikes + self.balls == 0
    }

    /// A strike is called or swung at. On a golden ball one strike is all
    /// the strikes there are, however many are `allowed`.
    pub fn strike(&mut self, golden: bool, allowed: u32) {
        self.strikes += 1;
        if golden {
            self.strikes = self.strikes.max(allowed);
        }
    }

    /// A foul is a strike, but never the last one. Says whether it was
    /// counted as one.
    pub fn foul(&mut self, allowed: u32) -> bool {
        let counted = self.strikes + 1 < allowed;
        if counted {
            self.strikes += 1;
        }
        counted
    }

    /// Whether the batter is out on strikes, with this many allowed.
    pub fn is_out(self, allowed: u32) -> bool {
        self.strikes >= allowed
    }

    /// Whether one more strike puts the batter out.
    pub fn is_one_strike_from_out(self, allowed: u32) -> bool {
        self.strikes + 1 == allowed
    }

    /// Whether the batter walks, with this many balls for a walk.
    pub fn is_a_walk(self, balls_for_a_walk: u32) -> bool {
        self.balls >= balls_for_a_walk
    }
}

#[cfg(test)]
pub(crate) mod tests {
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

    #[test]
    fn a_foul_is_a_strike_but_never_the_last_one() {
        let mut count = Count::default();
        assert!(count.is_clean());
        assert!(count.foul(3));
        assert!(count.foul(3));
        assert!(!count.foul(3));
        assert_eq!(count.strikes, 2);
        assert!(count.is_one_strike_from_out(3));
        assert!(!count.is_out(3));
        // With one strike and out, a foul is no strike at all.
        let mut sudden = Count::default();
        assert!(!sudden.foul(1));
        assert!(sudden.is_clean());
    }

    #[test]
    fn a_strike_on_a_golden_ball_is_all_the_strikes_there_are() {
        let mut count = Count::default();
        count.strike(false, 3);
        assert_eq!(count.strikes, 1);
        assert!(!count.is_out(3));
        count.strike(true, 3);
        assert!(count.is_out(3));
        assert_eq!(count.strikes, 3);
    }

    #[test]
    fn four_balls_are_a_walk() {
        let mut count = Count::default();
        for _ in 0..3 {
            count.balls += 1;
            assert!(!count.is_a_walk(4));
        }
        count.balls += 1;
        assert!(count.is_a_walk(4));
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
}
