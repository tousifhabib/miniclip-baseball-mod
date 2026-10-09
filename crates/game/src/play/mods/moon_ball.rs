//! Moon ball: every hit floats. It goes where it would have gone, and
//! takes several times as long over it, as you set.
//!
//! The mod keeps nothing as the game is played: how the ball flies is
//! settled in the numbers the game is played by, when it starts. The arcade
//! game is left as it is.

use crate::mods::{About, Setting};

/// What the menu and the files know this mod by.
pub(crate) const ABOUT: About = About {
    key: "moon_ball",
    name: "MOON BALL",
    does: "EVERY HIT FLOATS: THE SAME FLIGHT, MANY TIMES SLOWER",
    setting: Some(Setting {
        name: "FLOAT",
        usual: 2,
        levels: |rules| rules.moon.slow.count(),
        words: |level, rules| format!("{}X", rules.moon.slow.at(level).unwrap_or(1.0)),
    }),
};
