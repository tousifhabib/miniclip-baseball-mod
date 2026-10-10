//! The games of the record that are of a tournament: each shape of one,
//! from its tables by way of the menu into a fixture, and on from the
//! screen that ends on to wherever the batter's presses take him.

use bb_game::mods::Mod;
use bb_game::tournament::Format;

use super::{A_SHORT_TOURNAMENT, By, Game};

/// Where things are on the way into a fixture, in stage pixels: the
/// button on the board the tables are on, and the menu's.
const CARRY_ON: &str = "click 542 357";
const PLAY_BALL: &str = "click 480 362";

/// A tournament of each shape with matches of one innings, with no mods
/// and with every mod. The batter plays its first fixture, and then stays
/// and presses what he finds: the pages of the match, the tables, the
/// menu's summary, and as often as not the next fixture.
pub fn tournaments() -> Vec<Game> {
    let steps = format!("wait 120; {CARRY_ON}; wait 70; {PLAY_BALL}; wait 60");
    let mut games = Vec::new();
    for shape in Format::ALL {
        for (name, mods) in [("no mods", &[][..]), ("every mod", &Mod::ALL[..])] {
            games.push(Game {
                shape: Some(shape),
                rules: A_SHORT_TOURNAMENT,
                by: By::WrittenThenBatter(steps.clone()),
                most: 18_000,
                ..Game::new(
                    format!("tournament, {}, {name}", shape.key()),
                    "tournament",
                    520 + games.len() as u64,
                    mods,
                )
            });
        }
    }
    games
}
