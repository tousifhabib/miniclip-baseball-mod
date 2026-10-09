//! Sudden death: one strike and the batter is out, but every run counts
//! double.
//!
//! A foul is still never the last strike, so with only one to give it is no
//! strike at all. Runs a called shot or a sign is worth are not doubled.

use crate::mods::About;
use crate::rules::SuddenDeathRules;

/// What the menu and the files know this mod by.
pub(crate) const ABOUT: About = About {
    key: "sudden_death",
    name: "SUDDEN DEATH",
    does: "ONE STRIKE AND YOU ARE OUT, BUT RUNS COUNT DOUBLE",
    setting: None,
};

pub(crate) struct SuddenDeath {
    rules: SuddenDeathRules,
}

impl SuddenDeath {
    pub fn new(rules: &SuddenDeathRules) -> SuddenDeath {
        SuddenDeath {
            rules: rules.clone(),
        }
    }

    /// How many strikes put a batter out.
    pub fn strikes(&self) -> u32 {
        self.rules.strikes
    }

    /// How many times over a run counts.
    pub fn runs(&self) -> u32 {
        self.rules.runs
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::Rules;

    #[test]
    fn one_strike_is_out_and_a_run_counts_double() {
        let sudden = SuddenDeath::new(&Rules::default().sudden_death);
        assert_eq!(sudden.strikes(), 1);
        assert_eq!(sudden.runs(), 2);
    }
}
