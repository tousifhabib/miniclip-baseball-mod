//! The games of the record that go through the menu's pages.

use bb_game::mods::Mod;

use super::{ARCADE, BACK, BOTTOM_OF_THE_NINTH, By, FULL_MATCH, Game, MODS, NEXT, PLAY_BALL};

/// Round the menu by written steps: every page, and what can be done on
/// each.
pub fn the_menu() -> Vec<Game> {
    let written = |name: &str, screen: &'static str, mods: &[Mod], steps: String| Game {
        by: By::Written(steps),
        most: 6_000,
        ..Game::new(format!("menu, {name}"), screen, 370, mods)
    };
    vec![
        written(
            "the intro left to play",
            "intro",
            &[],
            "wait 500".to_owned(),
        ),
        written(
            "the intro skipped",
            "intro",
            &[],
            "wait 100; find SKIP; wait 200".to_owned(),
        ),
        written(
            "setting up a match",
            "menu",
            &[],
            format!(
                "wait 60; {BOTTOM_OF_THE_NINTH}; wait 60; click 450 134; type Red Sox 9; \
                 key backspace; key backspace; click 300 300; type xyz; click 440 194; wait 5; \
                 click 440 232; wait 5; click 420 270; wait 5; find EASY; wait 5; find HARD; \
                 wait 5; find MEDIUM; wait 5; {NEXT}; wait 60; {BACK}; wait 60; {NEXT}; wait 60; \
                 {PLAY_BALL}; wait 200"
            ),
        ),
        written(
            "setting up the arcade game",
            "menu",
            &[],
            format!(
                "wait 60; {ARCADE}; wait 60; click 440 194; wait 5; find HARD; wait 5; {NEXT}; \
                 wait 60; {BACK}; wait 60; {BACK}; wait 60; {ARCADE}; wait 60; {NEXT}; wait 60; \
                 {PLAY_BALL}; wait 200"
            ),
        ),
        written(
            "setting up a full match",
            "menu",
            &[],
            format!(
                "wait 60; {FULL_MATCH}; wait 60; click 445 312; wait 5; click 380 312; wait 5; \
                 click 510 312; wait 5; {NEXT}; wait 60; {BACK}; wait 60; click 445 312; wait 5; \
                 {NEXT}; wait 60; {PLAY_BALL}; wait 200"
            ),
        ),
        written(
            "the mods' pages",
            "menu",
            &[],
            format!(
                "wait 60; {MODS}; wait 90; click 300 140; wait 5; click 300 175; wait 5; \
                 click 300 210; wait 5; click 300 245; wait 5; click 329 271; wait 5; \
                 click 379 271; wait 5; click 353 271; wait 5; click 300 140; wait 5; \
                 click 419 290; wait 30; click 300 140; wait 5; click 300 245; wait 5; \
                 click 419 290; wait 30; click 300 175; wait 5; click 419 290; wait 30; \
                 click 419 290; wait 30; click 419 290; wait 30; click 419 290; wait 30; \
                 click 338 290; wait 30; click 338 290; wait 30; {BACK}; wait 90; {MODS}; \
                 wait 90; {BACK}; wait 90"
            ),
        ),
        written(
            "the high scores and the instructions",
            "menu",
            &[],
            "wait 60; find HIGH SCORES 1; wait 120; find BACK; wait 120; find INSTRUCTIONS; \
             wait 120; find NEXT; wait 60; find NEXT; wait 60; find BACK; wait 60; find NEXT; \
             wait 60; find NEXT; wait 60; find MENU; wait 120"
                .to_owned(),
        ),
        written(
            "quitting a match",
            "match",
            &[],
            "wait 200; find QUIT; wait 30; find NO; wait 60; find QUIT; wait 30; find YES; \
             wait 200"
                .to_owned(),
        ),
        written(
            "an arcade game let go by, and its score",
            "arcade",
            &[],
            format!(
                "{}wait 420; find HIGH SCORES; wait 120; find BACK; wait 120",
                "wait 320; click 545 355; ".repeat(10)
            ),
        ),
    ]
}
