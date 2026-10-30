//! The games of the record that are sessions of batting practice: by way of
//! the menu and on to the report, by a batter who never stops, and by a
//! monkey.

use bb_game::baseball::Screen;
use bb_game::mods::Mod;

use super::menu::written;
use super::{BATTING_PRACTICE, By, Game, NEXT};

/// On the setup page, the boxes of the match's view and of no timing bar.
const MATCH_VIEW: &str = "click 510 289";
const BAR_OFF: &str = "click 510 316";
/// On the report, the arrow on to its next page and the button out of it.
const PAGE_ON: &str = "click 357 322";
const OUT: &str = "click 542 357";

/// The steps of a few pitches let go by, then of giving the session up and
/// reading what it comes to.
fn to_the_report() -> String {
    format!(
        "{}wait 200; find QUIT; wait 30; find YES; wait 120; {PAGE_ON}; wait 30; {PAGE_ON}; \
         wait 30; {OUT}; wait 120",
        "wait 320; click 545 355; ".repeat(3)
    )
}

/// The sessions there are.
pub fn practice() -> Vec<Game> {
    let mut games = vec![
        written(
            "a session of practice on the arcade game's view, and the report",
            "menu",
            &[],
            format!(
                "wait 60; {BATTING_PRACTICE}; wait 90; {NEXT}; wait 90; {}",
                to_the_report()
            ),
        ),
        written(
            "a session of practice on a match's view with no bar, and the report",
            "menu",
            &[],
            format!(
                "wait 60; {BATTING_PRACTICE}; wait 90; {MATCH_VIEW}; wait 5; {BAR_OFF}; wait 5; \
                 find BACK; wait 90; {BATTING_PRACTICE}; wait 90; {NEXT}; wait 90; {}",
                to_the_report()
            ),
        ),
        written(
            "a session of practice given up before a pitch",
            Screen::PRACTICE_ARCADE,
            &[],
            "wait 100; find QUIT; wait 30; find YES; wait 120".to_owned(),
        ),
    ];
    let every = &Mod::ALL[..];
    for (index, (screen, called)) in [
        (Screen::PRACTICE_ARCADE, "arcade game's view"),
        (Screen::PRACTICE_MATCH, "match's view"),
    ]
    .into_iter()
    .enumerate()
    {
        let seed = 600 + index as u64 * 10;
        games.push(Game {
            most: 6_000,
            ..Game::new(format!("practice on the {called}"), screen, seed, &[])
        });
        games.push(Game {
            most: 6_000,
            ..Game::new(
                format!("practice on the {called}, every mod"),
                screen,
                seed + 1,
                every,
            )
        });
        for monkey in 0..4 {
            let seed = seed + 2 + monkey;
            games.push(Game {
                by: By::Monkey,
                most: 6_000,
                ..Game::new(
                    format!("monkey {monkey}, practice on the {called}"),
                    screen,
                    seed,
                    &[],
                )
            });
        }
    }
    games
}
