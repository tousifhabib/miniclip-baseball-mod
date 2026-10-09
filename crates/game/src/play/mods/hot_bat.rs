//! Hot bat: each hit in a row widens your timing window, and a strike
//! resets it.
//!
//! Every swing in a row that meets the ball adds a frame to each end of the
//! window for the next, up to a most, each as good as the frame that was
//! the end. A strike that is not a foul takes the window back to what it
//! was. The corner of the batting view says how hot the bat is, and the
//! mark on the bat glows, redder the hotter.

use bb_engine::math::ColorTransform;

use super::{Line, hot_colour};
use crate::look;
use crate::mods::About;
use crate::play::pitch;
use crate::rules::{HotBatRules, PitchRules};

/// What the menu and the files know this mod by.
pub(crate) const ABOUT: About = About {
    key: "hot_bat",
    name: "HOT BAT",
    does: "EACH HIT IN A ROW WIDENS THE TIMING, A MISS RESETS IT",
    setting: None,
};

pub(crate) struct HotBat {
    rules: HotBatRules,
    /// How many swings in a row have met the ball.
    streak: u32,
}

impl HotBat {
    pub fn new(rules: &HotBatRules) -> HotBat {
        HotBat {
            rules: rules.clone(),
            streak: 0,
        }
    }

    pub fn streak(&self) -> u32 {
        self.streak
    }

    /// How hot the bat is: the hits in a row, as far as they count.
    fn heat(&self) -> u32 {
        self.streak.min(self.rules.most)
    }

    /// The bat has met the ball.
    pub fn met(&mut self) {
        self.streak += 1;
    }

    /// A strike that was not a foul.
    pub fn missed(&mut self) {
        self.streak = 0;
    }

    /// Widens the window the coming pitch can be met in by a frame at each
    /// end for every hit in a row.
    pub fn widen(&self, table: &mut PitchRules) {
        if self.streak > 0 {
            table.window = pitch::widened(&table.window, self.heat());
        }
    }

    /// What the corner of the batting view says while the bat is hot.
    pub fn line(&self) -> Option<Line> {
        (self.streak > 0).then(|| Line {
            name: "hotBat",
            words: format!("HOT BAT {}", self.heat()),
            colour: hot_colour(self.heat(), self.rules.most),
        })
    }

    /// What the mark on the bat is tinted while the bat is hot.
    pub fn glow(&self) -> Option<ColorTransform> {
        (self.streak > 0).then(|| look::tint(hot_colour(self.heat(), self.rules.most)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::Rules;
    use crate::settings::Difficulty;

    fn bat_and_table() -> (HotBat, PitchRules) {
        let rules = Rules::default();
        let table = rules.pitch.at(Difficulty::Medium).clone();
        (HotBat::new(&rules.hot_bat), table)
    }

    #[test]
    fn a_cold_bat_changes_nothing_and_says_nothing() {
        let (bat, usual) = bat_and_table();
        let mut table = usual.clone();
        bat.widen(&mut table);
        assert_eq!(table, usual);
        assert!(bat.line().is_none());
        assert!(bat.glow().is_none());
    }

    #[test]
    fn each_hit_in_a_row_adds_a_frame_at_each_end_up_to_three() {
        let (mut bat, usual) = bat_and_table();
        let mut widths = Vec::new();
        for _ in 0..5 {
            bat.met();
            let mut table = usual.clone();
            bat.widen(&mut table);
            widths.push(table.window.len() - usual.window.len());
        }
        assert_eq!(widths, [2, 4, 6, 6, 6]);
        assert_eq!(bat.line().expect("a line").words, "HOT BAT 3");
    }

    #[test]
    fn a_strike_takes_the_window_back_to_what_it_was() {
        let (mut bat, usual) = bat_and_table();
        bat.met();
        bat.met();
        bat.missed();
        let mut table = usual.clone();
        bat.widen(&mut table);
        assert_eq!((bat.streak(), table), (0, usual));
    }
}
