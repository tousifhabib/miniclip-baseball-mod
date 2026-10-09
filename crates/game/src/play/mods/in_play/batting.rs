//! What the mods say about the swing: the hot bat, the timing bar, a
//! shot that was called, and what a hit or a strike tells them.

use bb_engine::math::ColorTransform;

use super::ModsInPlay;
use crate::play::mods::Line;
use crate::play::mods::hot_bat::HotBat;
use crate::rules::PitchRules;

impl ModsInPlay {
    /// Widens the window the coming pitch can be met in, for a bat that is
    /// hot. Returns what the corner of the view says of it.
    pub fn widen_for_a_hot_bat(&self, table: &mut PitchRules) -> Option<Line> {
        let bat = self.hot_bat.as_ref()?;
        bat.widen(table);
        bat.line()
    }

    /// What the mark on the bat is tinted, while the bat is hot.
    pub fn glow_of_the_bat(&self) -> Option<ColorTransform> {
        self.hot_bat.as_ref().and_then(HotBat::glow)
    }

    /// How many swings in a row have met the ball.
    pub fn hits_in_a_row(&self) -> u32 {
        self.hot_bat.as_ref().map_or(0, HotBat::streak)
    }

    /// The bat has met the ball.
    pub fn the_bat_met_the_ball(&mut self) {
        if let Some(bat) = &mut self.hot_bat {
            bat.met();
        }
    }

    /// A strike has been called on the batter, swung at or not. In the
    /// arcade game, which calls nothing, it is a pitch that went by unhit.
    pub fn a_strike_was_called(&mut self) {
        if let Some(bat) = &mut self.hot_bat {
            bat.missed();
        }
        self.cool();
    }

    /// A foul has counted as a strike against the batter.
    pub fn a_foul_took_a_strike(&mut self) {
        self.cool();
    }

    /// Whether the bar that shows when to swing is put up.
    pub fn the_timing_bar_is_shown(&self) -> bool {
        self.timing_indicator.is_some()
    }

    /// Whether the batter may call where his hit will come down.
    pub fn shots_are_called(&self) -> bool {
        self.called_shot.is_some()
    }

    /// A ball that was hit has put the batter on base, or all the way
    /// home, or in the arcade game has scored.
    pub fn a_hit_came_off(&mut self, got_home: bool) {
        if let Some(bullet) = &mut self.bullet_time {
            bullet.refill(got_home);
        }
    }
}
