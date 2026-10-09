//! Heat check: every run you score makes the next pitch faster, and every
//! strike slows them again.
//!
//! A foul that counts as a strike cools the pitches as any strike does. The
//! arcade game has no runs, so there the heat never rises and it plays as
//! it did.

use super::Line;
use crate::rules::{HeatRules, PitchRules};

pub(crate) struct HeatCheck {
    rules: HeatRules,
    /// How much heat is on: a run's worth for each run, up to the most.
    heat: u32,
    /// The score when the heat was last brought up to date, so that only
    /// the runs since are added.
    seen_score: u32,
}

impl HeatCheck {
    pub fn new(rules: &HeatRules) -> HeatCheck {
        HeatCheck {
            rules: rules.clone(),
            heat: 0,
            seen_score: 0,
        }
    }

    pub fn heat(&self) -> u32 {
        self.heat
    }

    /// Takes in the runs scored since the last pitch, and makes the pitch
    /// that is coming as much faster as the heat now on says.
    pub fn warm(&mut self, score: u32, table: &mut PitchRules) {
        let runs = score.saturating_sub(self.seen_score);
        self.heat = (self.heat + runs).min(self.rules.most);
        self.seen_score = score;
        table.speed = table.speed.times(self.rules.time(self.heat));
    }

    /// A strike has been called: the pitches slow down by a run's worth.
    pub fn cool(&mut self) {
        self.heat = self.heat.saturating_sub(1);
    }

    /// What the corner of the batting view says while any heat is on:
    /// redder the more there is.
    pub fn line(&self) -> Option<Line> {
        (self.heat > 0).then(|| {
            let hot = self.heat as f32 / self.rules.most.max(1) as f32;
            Line {
                name: "heat",
                words: format!("HEAT {}", self.heat),
                colour: [0xff, (0xe0 as f32 - 0xa0 as f32 * hot) as u8, 0x30],
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::Rules;
    use crate::settings::Difficulty;

    fn heat_and_table() -> (HeatCheck, PitchRules) {
        let rules = Rules::default();
        let table = rules.pitch.at(Difficulty::Medium).clone();
        (HeatCheck::new(&rules.heat), table)
    }

    #[test]
    fn each_run_since_the_last_pitch_adds_heat_and_a_strike_takes_one_off() {
        let (mut heat, usual) = heat_and_table();
        heat.warm(0, &mut usual.clone());
        assert_eq!(heat.heat(), 0);
        heat.warm(3, &mut usual.clone());
        assert_eq!(heat.heat(), 3);
        // The same score again adds nothing.
        heat.warm(3, &mut usual.clone());
        assert_eq!(heat.heat(), 3);
        heat.cool();
        assert_eq!(heat.heat(), 2);
        // And it never goes over the most there can be.
        heat.warm(100, &mut usual.clone());
        assert_eq!(heat.heat(), Rules::default().heat.most);
    }

    #[test]
    fn a_pitch_takes_less_time_the_more_heat_is_on() {
        let (mut heat, usual) = heat_and_table();
        let mut cold = usual.clone();
        heat.warm(0, &mut cold);
        assert_eq!(cold.speed, usual.speed);
        let mut hot = usual.clone();
        heat.warm(4, &mut hot);
        assert!(hot.speed.high < usual.speed.high, "{:?}", hot.speed);
    }

    #[test]
    fn the_corner_says_how_much_heat_is_on_once_there_is_some() {
        let (mut heat, usual) = heat_and_table();
        assert!(heat.line().is_none());
        heat.warm(2, &mut usual.clone());
        assert_eq!(heat.line().expect("a line").words, "HEAT 2");
    }
}
