//! What every screen works from: the rules, the player's choices and the
//! mods that are on.

use crate::mods::Mods;
use crate::rules::Rules;
use crate::settings::Settings;

/// What every screen works from: the numbers the game is played by, what
/// the player has chosen, and the mods that are switched on.
#[derive(Clone, Debug, Default)]
pub struct Game {
    pub rules: Rules,
    pub settings: Settings,
    pub mods: Mods,
}

impl Game {
    /// The game as it is to be played: the same, with what the mods that
    /// are on change of the numbers laid over them. `a_match` is whether
    /// it is a match and not the arcade game, which the mods that change
    /// how the ball flies over the field leave alone.
    pub fn as_played(&self, a_match: bool) -> Game {
        let mut played = self.clone();
        if a_match {
            played.rules.field = crate::play::mods::field_in_a_match(self);
        }
        played
    }
}
