//! The games of the record that are of a tournament: each shape of one,
//! from its tables by way of the menu into a fixture, and on from the
//! screen that ends on to wherever the batter's presses take him.

use bb_game::mods::Mod;
use bb_game::tournament::Format;

use super::menu::written;
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

/// Round the menu's pages for a tournament by written steps.
pub fn on_the_menu() -> Vec<Game> {
    // Its shape and its innings chosen, drawn, gone back on and drawn
    // again, its tables looked at, and its first fixture begun.
    let setting_up = "wait 60; find TOURNAMENT; wait 60; click 434 290; wait 5; click 513 290; \
                      wait 5; click 489 317; wait 5; find HARD; wait 5; find NEXT; wait 70; \
                      find BACK; wait 70; click 355 290; wait 5; find NEXT; wait 70; \
                      click 304 292; wait 130; click 151 63; wait 10; click 255 63; wait 10; \
                      click 52 120; wait 10; click 359 63; wait 10; click 463 63; wait 10; \
                      click 542 357; wait 70; find PLAY BALL; wait 200";
    // Drawn, given up at the second asking, and another begun to be set
    // up.
    let giving_up = "wait 60; find TOURNAMENT; wait 60; find NEXT; wait 70; click 450 292; \
                     wait 10; click 450 292; wait 70; find TOURNAMENT; wait 60; find BACK; \
                     wait 60";
    vec![
        written(
            "setting up a tournament",
            "menu",
            &[],
            setting_up.to_owned(),
        ),
        written("giving a tournament up", "menu", &[], giving_up.to_owned()),
    ]
}
