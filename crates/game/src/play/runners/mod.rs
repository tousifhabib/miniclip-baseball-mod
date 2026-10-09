//! The batters of the half being played, where each has got to, and the
//! count on the one at the plate.
//!
//! Everything here is a question that can be answered without looking at
//! the stage: who is up, who stands on which base, who is between bases,
//! who has to go when the batter does. Moving a runner's clip is the
//! fielding's business.

mod count;

pub(crate) use count::Count;

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

#[cfg(test)]
pub(crate) mod tests;
