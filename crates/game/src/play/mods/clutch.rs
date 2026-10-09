//! Clutch: a pitch thrown with one out left and a runner standing on second
//! or third is one on which every run counts double.
//!
//! A runner on first alone is not enough. The corner of the batting view
//! says so, and the organ plays as the batter comes up. The arcade game has
//! no runs, so the mod has no place there.

use super::Line;
use crate::mods::About;
use crate::rules::ClutchRules;

/// What the menu and the files know this mod by.
pub(crate) const ABOUT: About = About {
    key: "clutch",
    name: "CLUTCH",
    does: "TWO OUT AND A RUNNER ON SECOND OR THIRD: RUNS COUNT DOUBLE",
    setting: None,
};

pub(crate) struct Clutch {
    rules: ClutchRules,
    /// Whether the pitch in hand was thrown in the clutch. It is settled as
    /// the view is got ready, and stands until the next pitch.
    pub this_pitch: bool,
}

impl Clutch {
    pub fn new(rules: &ClutchRules) -> Clutch {
        Clutch {
            rules: rules.clone(),
            this_pitch: false,
        }
    }

    /// Whether a pitch thrown now is in the clutch: the side has one out
    /// left, and a runner is on second or third.
    pub fn is_now(one_out_left: bool, runner_in_reach_of_home: bool) -> bool {
        one_out_left && runner_in_reach_of_home
    }

    /// How many times over a run counts in the clutch.
    pub fn runs(&self) -> u32 {
        self.rules.runs
    }

    /// What the corner of the batting view says in the clutch.
    pub fn line(&self) -> Line {
        Line {
            name: "clutch",
            words: format!("CLUTCH: RUNS X{}", self.rules.runs),
            colour: [0xff, 0x8a, 0x6a],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::Rules;

    #[test]
    fn it_takes_one_out_left_and_a_runner_on_second_or_third() {
        assert!(Clutch::is_now(true, true));
        assert!(!Clutch::is_now(true, false));
        assert!(!Clutch::is_now(false, true));
    }

    #[test]
    fn a_run_in_the_clutch_counts_double_and_the_corner_says_so() {
        let clutch = Clutch::new(&Rules::default().clutch);
        assert_eq!(clutch.runs(), 2);
        assert_eq!(clutch.line().words, "CLUTCH: RUNS X2");
    }
}
