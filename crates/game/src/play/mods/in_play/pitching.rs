//! What the mods say about the pitch that is coming and the arm that
//! throws it.

use super::ModsInPlay;
use crate::play::mods::Line;
use crate::play::mods::heat_check::HeatCheck;
use crate::play::mods::southpaw::Southpaw;
use crate::play::mods::tired_arm::TiredArm;
use crate::play::snapshot::ArmSeen;
use crate::rules::PitchRules;

impl ModsInPlay {
    /// Takes in the runs scored since the last pitch and makes the coming
    /// one faster by the heat that is on. Returns what the corner of the
    /// view says of it.
    pub fn heat_the_pitch(&mut self, score: u32, table: &mut PitchRules) -> Option<Line> {
        let heat = self.heat_check.as_mut()?;
        heat.warm(score, table);
        heat.line()
    }

    /// How much heat is on.
    pub fn heat(&self) -> u32 {
        self.heat_check.as_ref().map_or(0, HeatCheck::heat)
    }

    pub(super) fn cool(&mut self) {
        if let Some(heat) = &mut self.heat_check {
            heat.cool();
        }
    }

    /// A new view is being got ready: what the last pitch struck has been
    /// told, and is done with.
    pub fn a_new_pitch_is_coming(&mut self) {
        if let Some(signs) = &mut self.hit_the_sign {
            signs.struck = None;
        }
    }

    /// Whether the batter is batting left-handed, once he has taken his
    /// stand.
    pub fn batting_left_handed(&self) -> bool {
        self.southpaw.as_ref().is_some_and(Southpaw::has_stood)
    }

    /// The ball has left the pitcher's hand.
    pub fn the_ball_was_thrown(&mut self) {
        if let Some(arm) = &mut self.tired_arm {
            arm.threw();
        }
    }

    /// How the pitcher's arm is holding up.
    pub fn arm(&self) -> Option<ArmSeen> {
        self.tired_arm.as_ref().and_then(TiredArm::seen)
    }
}
