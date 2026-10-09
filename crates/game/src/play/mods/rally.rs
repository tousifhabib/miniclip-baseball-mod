//! Rally: batters who reach base one after another make runs worth more.
//!
//! Each one who gets on, by a hit, a walk or a fielder's slip, adds one to
//! what a run counts for from the next pitch on, up to a most. An out of
//! any kind takes it back to one. What a run is worth is settled when the
//! pitch is thrown, so the batter who keeps the rally going does not raise
//! the worth of his own hit. The arcade game has no runs, so the mod has no
//! place there.

use super::Line;
use crate::play::hot_colour;
use crate::rules::RallyRules;

pub(crate) struct Rally {
    rules: RallyRules,
    /// How many batters in a row have reached base, with nobody put out
    /// since.
    in_a_row: u32,
}

impl Rally {
    pub fn new(rules: &RallyRules) -> Rally {
        Rally {
            rules: rules.clone(),
            in_a_row: 0,
        }
    }

    pub fn in_a_row(&self) -> u32 {
        self.in_a_row
    }

    /// A batter has reached base, which keeps the rally going.
    pub fn kept_up(&mut self) {
        self.in_a_row += 1;
    }

    /// Somebody is out, which ends it.
    pub fn broken(&mut self) {
        self.in_a_row = 0;
    }

    /// How many times over a run counts as the rally stands.
    pub fn worth(&self) -> u32 {
        self.rules.worth(self.in_a_row)
    }

    /// What the corner of the batting view says while a rally is on:
    /// hotter in colour the longer it has gone.
    pub fn line(&self) -> Option<Line> {
        let most = self.rules.most;
        (self.in_a_row > 0).then(|| Line {
            name: "rally",
            words: format!("RALLY: RUNS X{}", self.worth()),
            colour: hot_colour(self.in_a_row.min(most), most),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::Rules;

    fn rally() -> Rally {
        Rally::new(&Rules::default().rally)
    }

    #[test]
    fn each_batter_who_reaches_makes_a_run_worth_one_more_up_to_five() {
        let mut rally = rally();
        let mut worth = vec![rally.worth()];
        for _ in 0..6 {
            rally.kept_up();
            worth.push(rally.worth());
        }
        assert_eq!(worth, [1, 2, 3, 4, 5, 5, 5]);
    }

    #[test]
    fn an_out_takes_a_run_back_to_being_worth_one() {
        let mut rally = rally();
        rally.kept_up();
        rally.kept_up();
        rally.broken();
        assert_eq!((rally.in_a_row(), rally.worth()), (0, 1));
    }

    #[test]
    fn the_corner_says_what_runs_are_worth_only_while_a_rally_is_on() {
        let mut rally = rally();
        assert!(rally.line().is_none());
        rally.kept_up();
        rally.kept_up();
        rally.kept_up();
        let line = rally.line().expect("a line while a rally is on");
        assert_eq!(
            (line.name, line.words.as_str()),
            ("rally", "RALLY: RUNS X4")
        );
    }
}
